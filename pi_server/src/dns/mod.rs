use chrono::Local;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Instant;

use futures_util::StreamExt;
use log::{debug, error, info, warn};
use once_cell::sync::Lazy;
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use trust_dns_client::client::AsyncClient;
use trust_dns_proto::op::{Message, MessageType};
use trust_dns_proto::rr::{RData, Record, RecordType};
use trust_dns_proto::serialize::binary::BinEncodable;
use trust_dns_proto::udp::{UdpClientStream, UdpStream};
use trust_dns_proto::xfer::{DnsRequest, DnsResponse, SerialMessage};
use trust_dns_proto::{BufDnsStreamHandle, DnsHandle, DnsStreamHandle};

use crate::cloudflared;
use crate::db::dns_requests::save_request;
pub use crate::dns::filter::update_filters;
use crate::dns::filter::Filters;
use crate::{PiConfig, Timer, PI_CONFIG};

pub mod domain;
pub mod filter;

static ALLOW: Lazy<RwLock<Filters>> = Lazy::new(|| RwLock::new(Filters::default()));
static BLOCK: Lazy<RwLock<Filters>> = Lazy::new(|| RwLock::new(Filters::default()));

pub async fn start_dns_server() -> anyhow::Result<()> {
    let PiConfig {
        cloudflared_port,
        dns_port,
        ..
    } = PI_CONFIG.get().unwrap();
    info!(
        "Forwarding dns requests to dns://127.0.0.1:{}",
        cloudflared_port
    );

    let connection = UdpClientStream::<UdpSocket>::new(([127, 0, 0, 1], *cloudflared_port).into());
    let (cloudflare_client, req_sender) = AsyncClient::connect(connection).await?;

    info!("Starting DNS request sender");
    tokio::spawn(req_sender);

    let socket = UdpSocket::bind((IpAddr::from([0, 0, 0, 0]), *dns_port)).await?;
    // The IP address isn't relevant, and ideally goes essentially no where.
    // the address used is acquired from the inbound queries
    let server_addr = socket.local_addr().unwrap();
    let (mut buf_stream, stream_handle) = UdpStream::with_bound(socket, server_addr);
    info!("Registered UDP client at: {server_addr}");

    while let Some(message) = buf_stream.next().await {
        let message = match message {
            Err(e) => {
                warn!("Invalid message received: {e:?}");
                continue;
            }
            Ok(message) => {
                debug!("Message received from {}", message.addr());
                message
            }
        };
        let client = cloudflare_client.clone();
        let sender = stream_handle.with_remote_addr(message.addr());
        tokio::spawn(async move {
            let addr = message.addr();
            if let Err(e) = process_dns_request(message, client, sender).await {
                warn!("Failed to process message from {addr}: {e}");
            }
        });
    }
    Ok(())
}

async fn process_dns_request(
    message: SerialMessage,
    async_client: AsyncClient,
    stream_handle: BufDnsStreamHandle,
) -> anyhow::Result<()> {
    let start = Instant::now();
    let req_time = Local::now().naive_local();
    let mut processor = MessageProcessor {
        client: async_client,
        sender: stream_handle,
        addr: message.addr(),
        request: message.to_message()?,
        responses: Vec::with_capacity(1),
        allowed: None,
    };
    processor.process().await;
    info!("Time taken to process dns request: {}", start.elapsed().t());

    let (reason, allowed) = processor
        .allowed
        .take()
        .map(|(reason, allowed)| (Some(reason), Some(allowed)))
        .unwrap_or((None, None));
    let resp_ms = start.elapsed().as_millis() as i64;
    let log_res = if processor.responses.is_empty() {
        cloudflared::error::inc_count();
        &processor.request
    } else {
        &processor.responses[0]
    };
    save_request(req_time, log_res, allowed, reason, true, resp_ms).await?;
    Ok(())
}

struct MessageProcessor {
    client: AsyncClient,
    sender: BufDnsStreamHandle,
    addr: SocketAddr,
    request: Message,
    responses: Vec<DnsResponse>,
    allowed: Option<(String, bool)>,
}

impl MessageProcessor {
    async fn process(&mut self) {
        self.allowed = self.allow_request().await;
        if self
            .allowed
            .as_ref()
            .map(|(_, allowed)| *allowed)
            .unwrap_or(true)
        {
            self.forward_to_cloudflare().await;
        } else {
            self.create_fake_response();
        }
        self.reply_back();
        self.log_msg();
    }

    async fn allow_request(&self) -> Option<(String, bool)> {
        let mut block_reason = None;
        for query in self.request.queries() {
            let name = query.name();
            if let Some(allow_name) = { ALLOW.read().await.check(name) } {
                debug!("{name} is allowed by {allow_name}");
                return Some((allow_name.to_string(), true));
            }
            if let Some(block_name) = { BLOCK.read().await.check(name) } {
                info!("{name} is blocked by {block_name}");
                block_reason = Some(block_name.to_string());
            }
        }
        block_reason.map(|r| (r, false))
    }

    async fn forward_to_cloudflare(&mut self) {
        let start = Instant::now();
        let id = self.request.id();
        let request = DnsRequest::new(self.request.clone(), Default::default());
        let mut res_stream = self.client.send(request);
        while let Some(response) = res_stream.next().await {
            let mut res = match response {
                Err(e) => {
                    error!("Dns forwarding failed: {e}");
                    continue;
                }
                Ok(res) => res,
            };
            res.set_id(id); // Somehow the id has changed
            self.responses.push(res);
        }
        info!("Time taken to forward dns request: {}", start.elapsed().t());
    }

    fn create_fake_response(&mut self) {
        let mut response = DnsResponse::from(self.request.clone());
        response.set_message_type(MessageType::Response);
        response.set_recursion_available(true);
        response.set_authentic_data(false);
        for query in self.request.queries() {
            let mut record = Record::default();
            record.set_name(query.name().clone());
            record.set_rr_type(query.query_type());
            record.set_dns_class(query.query_class());
            let rdata = match query.query_type() {
                RecordType::A => Some(RData::A(Ipv4Addr::UNSPECIFIED)),
                RecordType::AAAA => Some(RData::AAAA(Ipv6Addr::UNSPECIFIED)),
                _ => None,
            };
            record.set_data(rdata);
            record.set_ttl(0);
            response.add_answer(record);
        }
        self.responses.push(response);
    }

    fn reply_back(&mut self) {
        if !self.responses.is_empty() {
            let payload = self
                .responses
                .iter()
                .filter_map(|res| res.to_bytes().ok())
                .flatten()
                .collect::<Vec<_>>();
            let response = SerialMessage::new(payload, self.addr);
            match self.sender.send(response) {
                Ok(_) => debug!("Successfully replied back to {}", self.addr),
                Err(e) => error!("Failed to send the response back: {e}"),
            };
        } else {
            warn!("Response is empty, can't reply back");
        }
    }

    fn log_msg(&self) {
        info!(
            "*************************** DNS Record ***************************\nRequest: {:?}\nResponse: {:?}\n***************************************************************\n",
            self.request, self.responses
        );
    }
}
