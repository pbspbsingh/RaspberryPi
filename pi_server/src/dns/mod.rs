use futures_util::StreamExt;
use tokio::net::{TcpListener, UdpSocket};
use tokio::time::{Duration, Instant};
use trust_dns_client::client::AsyncClient;
use trust_dns_proto::op::Message;
use trust_dns_proto::udp::{UdpClientStream, UdpStream};
use trust_dns_proto::xfer::dns_handle::DnsHandle;
use trust_dns_proto::xfer::{DnsRequest, SerialMessage};
use trust_dns_proto::BufStreamHandle;

use crate::PiConfig;
use trust_dns_proto::iocompat::AsyncIoTokioAsStd;
use trust_dns_proto::tcp::TcpStream;
use trust_dns_server::server::TimeoutStream;

pub async fn start_dns_server(config: &PiConfig) -> anyhow::Result<()> {
    let forward_add = format!("{}:{}", config.forward_server, config.forward_port);
    log::info!("Forwarding dns requests to {}", forward_add);
    let connection = UdpClientStream::<UdpSocket>::new(forward_add.parse()?);
    let (client, req_sender) = AsyncClient::connect(connection).await?;
    let _ = tokio::spawn(req_sender);

    tokio::try_join!(
        register_udp(client.clone(), config.port),
        register_tcp(client, config.port)
    )?;
    Ok(())
}

async fn register_udp(client: AsyncClient, port: u64) -> anyhow::Result<()> {
    log::debug!("Listening for UDP requests at port {}", port);
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", port)).await?;
    let (mut receiver, sender) = UdpStream::with_bound(socket);
    while let Some(message) = receiver.next().await {
        match message {
            Err(e) => log::warn!("Illegal UDP message received {:?}", e),
            Ok(message) => {
                log::debug!("UDP request from {:?}", message.addr());
                let processor = MessageProcessor {
                    client: client.clone(),
                    message,
                    sender: sender.clone(),
                };
                tokio::spawn(processor.process());
            }
        }
    }
    Ok(())
}

async fn register_tcp(client: AsyncClient, port: u64) -> anyhow::Result<()> {
    log::debug!("Listening for TCP requests at port {}", port);
    let tcp = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    loop {
        let (tcp_stream, src_addr) = match tcp.accept().await {
            Err(e) => {
                log::warn!("Illegal TCP message received {:?}", e);
                continue;
            }
            Ok((stream, addr)) => (stream, addr),
        };

        let client = client.clone();
        tokio::spawn(async move {
            let (receiver, sender) =
                TcpStream::from_stream(AsyncIoTokioAsStd(tcp_stream), src_addr);
            let mut receiver = TimeoutStream::new(receiver, Duration::from_secs(5));
            while let Some(message) = receiver.next().await {
                match message {
                    Err(e) => log::warn!("Invalid TCP request from {}: {}", src_addr, e),
                    Ok(message) => {
                        log::debug!("TCP request from {:?}", message.addr());
                        let processor = MessageProcessor {
                            client: client.clone(),
                            message,
                            sender: sender.clone(),
                        };
                        processor.process().await;
                    }
                };
            }
        });
    }
}

struct MessageProcessor {
    client: AsyncClient,
    message: SerialMessage,
    sender: BufStreamHandle,
}

impl MessageProcessor {
    async fn process(mut self) -> Option<()> {
        let request = self.parse_message()?;
        let response = self.forward_request(request).await?;
        Some(self.respond_back(response)?)
    }

    async fn forward_request(&mut self, message: Message) -> Option<Message> {
        let start = Instant::now();
        let request = DnsRequest::new(message, Default::default());
        let id = request.id();

        match self.client.send(request).await {
            Err(e) => {
                let elapsed = start.elapsed().as_millis();
                log::error!("[{}] DNS request failed in {} ms: {}", id, elapsed, e);
                return None;
            }
            Ok(mut response) => {
                response.set_id(id); // For some reason response id is different from request Id
                let elapsed = start.elapsed().as_millis();
                log::info!("[{}] DNS request succeeded in {} ms", id, elapsed);
                for answer in response.answers() {
                    log::debug!(
                        "[{}] {} {} {} => {}",
                        id,
                        answer.name().to_string(),
                        answer.record_type(),
                        answer.dns_class(),
                        answer.rdata()
                    );
                }
                if let Some(soa) = response.soa() {
                    log::debug!(
                        "[{}] SOA: {} {}",
                        id,
                        soa.mname().to_string(),
                        soa.rname().to_string(),
                    );
                }
                Some(response.into())
            }
        }
    }

    fn respond_back(&mut self, response: Message) -> Option<()> {
        let start = Instant::now();
        let id = response.id();
        let response = SerialMessage::new(response.to_vec().ok()?, self.message.addr());
        match self.sender.send(response) {
            Err(e) => {
                let elapsed = start.elapsed().as_millis();
                log::error!("[{}] Failed to respond back [{}ms]: {:?}", id, elapsed, e);
            }
            Ok(_) => {
                let elapsed = start.elapsed().as_millis();
                log::info!("[{}] Successfully responded back [{}ms]", id, elapsed);
            }
        }
        Some(())
    }

    fn parse_message(&self) -> Option<Message> {
        match Message::from_vec(self.message.bytes()) {
            Err(e) => {
                log::warn!("Failed to parse the message: {}", e);
                None
            }
            Ok(msg) => {
                log::debug!(
                    "[{}] Parsed message: {} edns: {}",
                    msg.id(),
                    msg.queries()
                        .first()
                        .map(|q| format!(
                            "{} {} {}",
                            q.name().to_string(),
                            q.query_type(),
                            q.query_class()
                        ))
                        .unwrap_or_else(|| Default::default()),
                    msg.edns().is_some()
                );
                Some(msg)
            }
        }
    }
}
