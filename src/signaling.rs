//! WebSocket signaling server for P2P coordination

mod actor;
mod messages;
mod server;
mod types;

pub use actor::RoomManagerHandle;
pub use messages::{ClientMessage, ServerMessage};
pub use server::{DEFAULT_SIGNALING_PORT, SignalingServer};
pub use types::{OutboundMessage, PeerId, PeerInfo, RoomCode, SignalingError};
