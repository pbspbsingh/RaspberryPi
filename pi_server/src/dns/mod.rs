use crate::dns::resolver::dns_lookup;
use futures_util::StreamExt;
use std::time::Instant;
use tokio::net::UdpSocket;
use trust_dns_proto::op::{Edns, Header, MessageType, ResponseCode};
use trust_dns_proto::rr::Record;
use trust_dns_proto::serialize::binary::BinDecoder;
use trust_dns_proto::udp::UdpStream;
use trust_dns_proto::xfer::SerialMessage;
use trust_dns_proto::BufStreamHandle;
use trust_dns_resolver::error::ResolveErrorKind;
use trust_dns_server::authority::{MessageRequest, MessageResponseBuilder};
use trust_dns_server::server::{ResponseHandle, ResponseHandler};

mod resolver;

pub async fn start_dns_server(port: u64) -> anyhow::Result<()> {
    log::debug!("Starting DNS server at port {}", port);

    let socket = UdpSocket::bind(format!("0.0.0.0:{}", port)).await?;
    let (mut buf_stream, stream_handle) = UdpStream::with_bound(socket);
    while let Some(message) = buf_stream.next().await {
        match message {
            Err(e) => log::warn!("Invalid message received: {:?}", e),
            Ok(msg) => {
                log::debug!(
                    "Dns message of size {} from {}",
                    msg.bytes().len(),
                    msg.addr(),
                );
                tokio::spawn(handle_message(msg, stream_handle.clone()));
            }
        }
    }
    Ok(())
}

async fn handle_message(msg: SerialMessage, handle: BufStreamHandle) {
    use trust_dns_proto::serialize::binary::BinDecodable;

    match MessageRequest::read(&mut BinDecoder::new(msg.bytes())) {
        Err(e) => log::warn!("Failed to read message: {:?}", e),
        Ok(message) => {
            log::info!(
                "Request: {}, Type: {:?}, OpCode: {:?}, DnsSec: {}, {}",
                message.id(),
                message.message_type(),
                message.op_code(),
                message.edns().map_or(false, Edns::dnssec_ok),
                message
                    .queries()
                    .first()
                    .map(|q| q.original().to_string())
                    .unwrap_or_else(|| "empty_queries".to_string())
            );

            let start = Instant::now();
            if let Err(e) = process(message, ResponseHandle::new(msg.addr(), handle)).await {
                log::warn!("Failed to send response: {:?}, in {}ms", e, t(&start));
            } else {
                log::debug!(
                    "Successfully responded to {}, in {} ms",
                    msg.addr(),
                    t(&start)
                );
            }
        }
    }
}

const SUPPORTED_EDNS_VERSION: u8 = 0;

async fn process(
    request: MessageRequest,
    mut response_handle: ResponseHandle,
) -> anyhow::Result<()> {
    let start = Instant::now();

    let mut response_header = Header::default();
    response_header.set_id(request.id());
    response_header.set_op_code(request.op_code());
    response_header.set_message_type(MessageType::Response);
    response_header.set_recursion_desired(request.recursion_desired());
    response_header.set_recursion_available(request.recursion_desired());

    let response_edns = if let Some(req_edns) = request.edns() {
        let mut resp_edns: Edns = Edns::new();
        resp_edns.set_dnssec_ok(true);
        resp_edns.set_max_payload(req_edns.max_payload().max(512));
        resp_edns.set_version(SUPPORTED_EDNS_VERSION);

        if req_edns.version() > SUPPORTED_EDNS_VERSION {
            log::warn!(
                "request edns version greater than {}: {}",
                SUPPORTED_EDNS_VERSION,
                req_edns.version()
            );
            let mut response = MessageResponseBuilder::new(Some(request.raw_queries()));
            response_header.set_response_code(ResponseCode::BADVERS);
            response.edns(resp_edns);
            return Ok(response_handle.send_response(response.build_no_records(response_header))?);
        }
        Some(resp_edns)
    } else {
        None
    };

    let result = match request.message_type() {
        MessageType::Query => {
            log::debug!("Query[{}]: {:?}", request.id(), request.queries(),);
            log::info!("Query[{}] count: {}", request.id(), request.queries().len());
            log::info!("Time taken[{}]: {} ms", request.id(), t(&start));

            let mut response = MessageResponseBuilder::new(Some(request.raw_queries()));
            if let Some(edns) = response_edns {
                response.edns(edns);
            }
            if request.queries().is_empty() {
                log::warn!("Query[{}] is empty, returning NxDomain", request.id());
                return Ok(response_handle.send_response(response.error_msg(
                    request.id(),
                    request.op_code(),
                    ResponseCode::NXDomain,
                ))?);
            }

            let query = &request.queries()[0];
            log::info!(
                "Resolving '{}' record type: {}, time: {} ms",
                query.name().to_string(),
                query.query_type(),
                t(&start)
            );
            match dns_lookup(query.name().to_string(), query.query_type()).await {
                Ok(lookup) => {
                    log::info!(
                        "Resolved: {:?}, time: {} ms",
                        lookup
                            .record_iter()
                            .map(|r| r.to_string())
                            .collect::<Vec<_>>(),
                        t(&start)
                    );
                    let success_response = response.build(
                        response_header,
                        Box::new(lookup.record_iter()) as Box<dyn Iterator<Item = &Record> + Send>,
                        Box::new(Vec::new().into_iter())
                            as Box<dyn Iterator<Item = &Record> + Send>,
                        Box::new(Vec::new().into_iter())
                            as Box<dyn Iterator<Item = &Record> + Send>,
                        Box::new(Vec::new().into_iter())
                            as Box<dyn Iterator<Item = &Record> + Send>,
                    );
                    response_handle.send_response(success_response)
                }
                Err(resolve_err) => {
                    log::warn!("Resolution failed: {:?}", resolve_err.kind());
                    let res_code = match resolve_err.kind() {
                        ResolveErrorKind::NoRecordsFound { response_code, .. } => *response_code,
                        _ => ResponseCode::ServFail,
                    };
                    response_handle.send_response(response.error_msg(
                        request.id(),
                        request.op_code(),
                        res_code,
                    ))
                }
            }
        }
        _ => {
            log::error!("Unimplemented op_code: {:?}", request.op_code());
            let response = MessageResponseBuilder::new(Some(request.raw_queries()));
            response_handle.send_response(response.error_msg(
                request.id(),
                request.op_code(),
                ResponseCode::NotImp,
            ))
        }
    };
    result.map_err(|e| e.into())
}

#[inline]
fn t(instant: &Instant) -> u128 {
    instant.elapsed().as_millis()
}
