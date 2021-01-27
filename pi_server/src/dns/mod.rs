use std::time::{Duration, Instant};

use chrono::Local;
use futures_util::StreamExt;
use once_cell::sync::OnceCell;
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::RwLock;
use trust_dns_client::client::AsyncClient;
use trust_dns_proto::iocompat::AsyncIoTokioAsStd;
use trust_dns_proto::op::Message;
use trust_dns_proto::rr::{RData, RecordType};
use trust_dns_proto::tcp::TcpStream;
use trust_dns_proto::udp::{UdpClientStream, UdpStream};
use trust_dns_proto::xfer::dns_handle::DnsHandle;
use trust_dns_proto::xfer::{DnsRequest, SerialMessage};
use trust_dns_proto::BufStreamHandle;
use trust_dns_server::server::TimeoutStream;

use dns_requests::save_request;

use crate::db::dns_requests;
pub use crate::dns::filter::update_filters;
use crate::dns::filter::Filters;
use crate::{PiConfig, Timer};

pub mod domain;
pub mod filter;

static ALLOW: OnceCell<RwLock<Filters>> = OnceCell::new();
static BLOCK: OnceCell<RwLock<Filters>> = OnceCell::new();

pub async fn start_dns_server(config: &PiConfig) -> anyhow::Result<()> {
    log::info!("Forwarding dns requests to {}", config.forward_server);
    let connection = UdpClientStream::<UdpSocket>::new(config.forward_server.parse()?);
    let (client, req_sender) = AsyncClient::connect(connection).await?;
    let _ = tokio::spawn(req_sender);

    tokio::try_join!(
        register_udp(client.clone(), config.dns_port),
        register_tcp(client, config.dns_port)
    )?;
    Ok(())
}

async fn register_udp(client: AsyncClient, port: u32) -> anyhow::Result<()> {
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

async fn register_tcp(client: AsyncClient, port: u32) -> anyhow::Result<()> {
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
        let start = Instant::now();
        let req_time = Local::now().naive_local();
        let request = self.parse_message()?;
        if let Some(mut response) = self.forward_request(request.clone()).await {
            let mut filtered = None;
            let mut cause = None;
            if let Some((reason, allowed)) = self.filter(&request).await {
                if !allowed {
                    MessageProcessor::update_response(&mut response)
                }
                filtered = Some(allowed);
                cause = Some(reason);
            }
            self.respond_back(&response, &start)?;
            let resp_ms = start.elapsed().as_millis() as i64;
            save_request(req_time, &response, filtered, cause, true, resp_ms)
                .await
                .ok()?;
        } else {
            let resp_ms = start.elapsed().as_millis() as i64;
            save_request(req_time, &request, None, None, false, resp_ms)
                .await
                .ok()?;
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

    async fn filter(&self, request: &Message) -> Option<(String, bool)> {
        let mut block_reason = None;
        for query in request.queries() {
            let name = query.name();
            if let Some(allow) = ALLOW.get() {
                if let Some(allow_name) = { allow.read().await.check(name) } {
                    log::info!("{} is allowed by {}", name.to_string(), allow_name);
                    return Some((allow_name.to_string(), true));
                }
            } else {
                log::warn!("ALLOW is not initialized yet!!");
            }

            if let Some(block) = BLOCK.get() {
                if let Some(block_name) = { block.read().await.check(name) } {
                    log::warn!("{} is blocked by {}", name.to_string(), block_name);
                    block_reason = Some(block_name.to_string());
                }
            } else {
                log::warn!("BLOCK is not initialized yet!!");
            }
        }
        block_reason.map(|r| (r, false))
    }

    fn update_response(response: &mut Message) {
        for ans in response.answers_mut() {
            match ans.record_type() {
                RecordType::A => {
                    ans.set_rdata(RData::A("0.0.0.0".parse().unwrap()));
                }
                RecordType::AAAA => {
                    ans.set_rdata(RData::AAAA("::/0".parse().unwrap()));
                }
                _ => {
                    log::warn!(
                        "Unexpected record type in blocked response: {}",
                        ans.record_type()
                    );
                }
            }
        }
    }

    async fn forward_request(&mut self, message: Message) -> Option<Message> {
        let start = Instant::now();
        let request = DnsRequest::new(message, Default::default());
        let id = request.id();

        match self.client.send(request).await {
            Err(e) => {
                log::error!("[{}] DNS request failed in {}: {}", id, start.t(), e);
                return None;
            }
            Ok(mut response) => {
                response.set_id(id); // For some reason response id is different from request Id
                log::info!("[{}] DNS request succeeded in {}", id, start.t());
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

    fn respond_back(&mut self, response: &Message, start: &Instant) -> Option<()> {
        let id = response.id();
        let response = SerialMessage::new(response.to_vec().ok()?, self.message.addr());
        match self.sender.send(response) {
            Err(e) => {
                log::error!("[{}] Failed to respond back [{}]: {:?}", id, start.t(), e);
            }
            Ok(_) => {
                log::info!("[{}] Successfully responded back [{}]", id, start.t());
            }
        }
        Some(())
    }
}
