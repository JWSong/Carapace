use std::collections::HashMap;
use std::net::SocketAddr;

use tokio::sync::{mpsc, oneshot};
use tracing::info;

use super::messages::ServerMessage;
use super::types::{OutboundMessage, PeerId, PeerInfo, PeerState, Room, RoomCode, SignalingError};

/// Commands sent to the room manager actor
pub(crate) enum RoomCommand {
    Create {
        addr: SocketAddr,
        peer_tx: mpsc::UnboundedSender<OutboundMessage>,
        reply: oneshot::Sender<(RoomCode, PeerId)>,
    },
    Join {
        code: RoomCode,
        addr: SocketAddr,
        peer_tx: mpsc::UnboundedSender<OutboundMessage>,
        reply: oneshot::Sender<Result<(PeerId, Vec<PeerInfo>), SignalingError>>,
    },
    Leave {
        peer_id: PeerId,
    },
}

pub(crate) async fn room_manager_actor(mut rx: mpsc::Receiver<RoomCommand>) {
    let mut rooms: HashMap<RoomCode, Room> = HashMap::new();
    let mut peer_rooms: HashMap<PeerId, RoomCode> = HashMap::new();

    while let Some(cmd) = rx.recv().await {
        match cmd {
            RoomCommand::Create {
                addr,
                peer_tx,
                reply,
            } => {
                let code = RoomCode::generate();
                let peer_id = PeerId::generate();

                let peer_state = PeerState {
                    info: PeerInfo {
                        id: peer_id,
                        public_addr: Some(addr),
                    },
                    tx: peer_tx,
                };

                let room = Room {
                    peers: HashMap::from([(peer_id, peer_state)]),
                };

                rooms.insert(code, room);
                peer_rooms.insert(peer_id, code);

                info!("Room created: {} by peer {}", code, peer_id);
                let _ = reply.send((code, peer_id));
            }

            RoomCommand::Join {
                code,
                addr,
                peer_tx,
                reply,
            } => {
                let result = if let Some(room) = rooms.get_mut(&code) {
                    let peer_id = PeerId::generate();

                    let existing_peers: Vec<PeerInfo> =
                        room.peers.values().map(|p| p.info).collect();

                    let join_msg = ServerMessage::PeerJoined {
                        peer: PeerInfo {
                            id: peer_id,
                            public_addr: Some(addr),
                        },
                    };
                    let join_json = serde_json::to_string(&join_msg)
                        .expect("ServerMessage serialization should never fail");
                    let msg = OutboundMessage::from(join_json);
                    for peer in room.peers.values() {
                        let _ = peer.tx.send(msg.clone());
                    }

                    let peer_state = PeerState {
                        info: PeerInfo {
                            id: peer_id,
                            public_addr: Some(addr),
                        },
                        tx: peer_tx,
                    };
                    room.peers.insert(peer_id, peer_state);
                    peer_rooms.insert(peer_id, code);

                    info!("Peer {} joined room {}", peer_id, code);
                    Ok((peer_id, existing_peers))
                } else {
                    Err(SignalingError::RoomNotFound(code))
                };

                let _ = reply.send(result);
            }

            RoomCommand::Leave { peer_id } => {
                if let Some(code) = peer_rooms.remove(&peer_id) {
                    if let Some(room) = rooms.get_mut(&code) {
                        room.peers.remove(&peer_id);

                        if room.peers.is_empty() {
                            rooms.remove(&code);
                            info!("Room {} removed (empty)", code);
                        }
                    }
                    info!("Peer {} left room {}", peer_id, code);
                }
            }
        }
    }
}

/// Handle to communicate with the room manager actor
#[derive(Clone)]
pub struct RoomManagerHandle {
    pub(crate) tx: mpsc::Sender<RoomCommand>,
}

impl RoomManagerHandle {
    /// Create a new room and become the first peer
    pub async fn create_room(
        &self,
        addr: SocketAddr,
        peer_tx: mpsc::UnboundedSender<OutboundMessage>,
    ) -> Result<(RoomCode, PeerId), SignalingError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = self
            .tx
            .send(RoomCommand::Create {
                addr,
                peer_tx,
                reply: reply_tx,
            })
            .await;
        reply_rx
            .await
            .map_err(|_| SignalingError::Internal("actor channel closed".to_string()))
    }

    /// Join an existing room
    pub async fn join_room(
        &self,
        code: RoomCode,
        addr: SocketAddr,
        peer_tx: mpsc::UnboundedSender<OutboundMessage>,
    ) -> Result<(PeerId, Vec<PeerInfo>), SignalingError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = self
            .tx
            .send(RoomCommand::Join {
                code,
                addr,
                peer_tx,
                reply: reply_tx,
            })
            .await;
        reply_rx
            .await
            .map_err(|_| SignalingError::Internal("actor channel closed".to_string()))?
    }

    /// Leave the current room
    pub async fn leave_room(&self, peer_id: &PeerId) {
        let _ = self.tx.send(RoomCommand::Leave { peer_id: *peer_id }).await;
    }
}
