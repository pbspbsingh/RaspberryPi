use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use once_cell::sync::OnceCell;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::time::timeout;
use tokio::time::Duration;
use warp::filters::ws::{Message, WebSocket};

use crate::Timer;

static WS_SENDER: OnceCell<UnboundedSender<WsMessage>> = OnceCell::new();
static WS_ID: AtomicU32 = AtomicU32::new(0);

const WS_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub enum WsMessage {
    Store(u32, SplitSink<WebSocket, Message>),
    Drop(u32),
    Send(u32, String),
    SendAll(String),
}

pub fn send_ws_msg(msg: WsMessage) {
    WS_SENDER.get().unwrap().send(msg).ok();
}

pub async fn ws_sender() -> anyhow::Result<()> {
    let (sender, mut receiver) = unbounded_channel::<WsMessage>();
    WS_SENDER
        .set(sender)
        .map_err(|_| anyhow::anyhow!("OnceCell init failed"))?;

    let mut store = HashMap::new();
    while let Some(msg) = receiver.recv().await {
        match msg {
            WsMessage::Store(id, sink) => {
                log::debug!("Got store message with id: {}", id);
                store.insert(id, sink);
            }
            WsMessage::Drop(id) => {
                log::debug!("Got drop message with id: {}", id);
                if let Some(mut sink) = store.remove(&id) {
                    sink.close().await.ok();
                }
            }
            WsMessage::Send(id, msg) => {
                let start = Instant::now();
                log::trace!("Sending text message of size {} to {}", msg.len(), id);
                if let Some(sink) = store.get_mut(&id) {
                    match timeout(WS_TIMEOUT, sink.send(Message::text(msg))).await {
                        Ok(_) => log::trace!("Sent ws message in {}", start.t()),
                        Err(e) => {
                            log::warn!("Failed to send ws message to {}: {}", id, e);
                            send_ws_msg(WsMessage::Drop(id));
                        }
                    }
                } else {
                    log::warn!("No ws connection found for {}", id);
                }
            }
            WsMessage::SendAll(msg) => {
                store
                    .keys()
                    .for_each(|id| send_ws_msg(WsMessage::Send(*id, msg.clone())));
            }
        }
    }
    Ok(())
}

pub async fn handle_ws(ws: WebSocket) {
    let id = WS_ID.fetch_add(1, Ordering::SeqCst);
    log::info!("WS connection with id: {}", id);

    let (sink, mut stream) = ws.split();
    send_ws_msg(WsMessage::Store(id, sink));

    while let Some(msg) = stream.next().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                log::warn!("WS message error: {}", e);
                break;
            }
        };
        log::debug!("Got ws message[{}]: {:?}", id, msg);
        if msg.is_close() {
            break;
        }
    }
    send_ws_msg(WsMessage::Drop(id));
    log::info!("Closed ws connection: {}", id);
}
