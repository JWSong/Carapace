use std::net::SocketAddr;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{Bytes, Message};
use tracing::{debug, error, info, warn};

use super::actor::{RoomCommand, RoomManagerHandle, room_manager_actor};
use super::messages::{ClientMessage, ServerMessage};
use super::types::{OutboundMessage, PeerId, RoomCode};

pub const DEFAULT_SIGNALING_PORT: u16 = 3479;
const PING_INTERVAL: Duration = Duration::from_secs(30);
const PONG_TIMEOUT: Duration = Duration::from_secs(10);

pub struct SignalingServer {
    handle: RoomManagerHandle,
}

impl Default for SignalingServer {
    fn default() -> Self {
        Self::new()
    }
}

impl SignalingServer {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<RoomCommand>(1024);
        tokio::spawn(room_manager_actor(rx));

        Self {
            handle: RoomManagerHandle { tx },
        }
    }

    pub async fn run(&self, addr: &str) -> std::io::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        info!("Signaling server listening on {}", addr);

        loop {
            let (stream, addr) = listener.accept().await?;
            let handle = self.handle.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, addr, handle).await {
                    error!("Connection error from {}: {}", addr, e);
                }
            });
        }
    }
}

async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    handle: RoomManagerHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws_stream = tokio_tungstenite::accept_async(stream).await?;
    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    info!("WebSocket connection from {}", addr);

    let (tx, mut rx) = mpsc::unbounded_channel::<OutboundMessage>();
    let (ctrl_tx, mut ctrl_rx) = mpsc::unbounded_channel::<Message>();

    let mut peer_id: Option<PeerId> = None;
    let mut ping_interval = tokio::time::interval(PING_INTERVAL);
    let mut waiting_for_pong = false;
    let mut pong_deadline: Option<tokio::time::Instant> = None;

    let send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(msg) = rx.recv() => {
                    let ws_msg = Message::Text(msg.into_inner());
                    if ws_tx.send(ws_msg).await.is_err() {
                        break;
                    }
                }
                Some(ctrl_msg) = ctrl_rx.recv() => {
                    if ws_tx.send(ctrl_msg).await.is_err() {
                        break;
                    }
                }
                else => break,
            }
        }
    });

    loop {
        let pong_timeout = async {
            match pong_deadline {
                Some(deadline) => tokio::time::sleep_until(deadline).await,
                None => std::future::pending().await,
            }
        };

        tokio::select! {
            _ = ping_interval.tick() => {
                if waiting_for_pong {
                    warn!("No Pong received, disconnecting {}", addr);
                    break;
                }
                if ctrl_tx.send(Message::Ping(Bytes::new())).is_err() {
                    break;
                }
                waiting_for_pong = true;
                pong_deadline = Some(tokio::time::Instant::now() + PONG_TIMEOUT);
                debug!("Ping sent to {}", addr);
            }

            _ = pong_timeout => {
                warn!("Pong timeout, disconnecting {}", addr);
                break;
            }

            msg = ws_rx.next() => {
                let msg = match msg {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                };

                match msg {
                    Message::Text(text) => {
                        if let Err(e) = handle_text_message(&text, &tx, &handle, addr, &mut peer_id).await {
                            warn!("Message handling error: {}", e);
                        }
                    }
                    Message::Pong(_) => {
                        waiting_for_pong = false;
                        pong_deadline = None;
                        debug!("Pong received from {}", addr);
                    }
                    Message::Close(_) => {
                        info!("Close received from {}", addr);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    if let Some(ref pid) = peer_id {
        handle.leave_room(pid).await;
    }

    send_task.abort();
    info!("WebSocket disconnected: {}", addr);

    Ok(())
}

async fn handle_text_message(
    text: &str,
    tx: &mpsc::UnboundedSender<OutboundMessage>,
    handle: &RoomManagerHandle,
    addr: SocketAddr,
    peer_id: &mut Option<PeerId>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client_msg: ClientMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            let err = ServerMessage::Error {
                message: format!("Invalid message: {}", e),
            };
            let _ = tx.send(OutboundMessage::from(serde_json::to_string(&err)?));
            return Ok(());
        }
    };

    match client_msg {
        ClientMessage::CreateRoom => match handle.create_room(addr, tx.clone()).await {
            Ok((code, new_peer_id)) => {
                *peer_id = Some(new_peer_id);

                let response = ServerMessage::RoomCreated {
                    code,
                    your_id: new_peer_id,
                };
                let _ = tx.send(OutboundMessage::from(serde_json::to_string(&response)?));
            }
            Err(e) => {
                let err = ServerMessage::Error {
                    message: e.to_string(),
                };
                let _ = tx.send(OutboundMessage::from(serde_json::to_string(&err)?));
            }
        },

        ClientMessage::JoinRoom { code } => {
            let room_code = RoomCode::from(code.as_str());
            match handle.join_room(room_code, addr, tx.clone()).await {
                Ok((new_peer_id, peers)) => {
                    *peer_id = Some(new_peer_id);

                    let response = ServerMessage::RoomJoined {
                        code: room_code,
                        your_id: new_peer_id,
                        peers,
                    };
                    let _ = tx.send(OutboundMessage::from(serde_json::to_string(&response)?));
                }
                Err(e) => {
                    let err = ServerMessage::Error {
                        message: e.to_string(),
                    };
                    let _ = tx.send(OutboundMessage::from(serde_json::to_string(&err)?));
                }
            }
        }

        ClientMessage::LeaveRoom => {
            if let Some(pid) = peer_id.as_ref() {
                handle.leave_room(pid).await;
            }
            *peer_id = None;
        }
    }

    Ok(())
}
