use std::collections::HashMap;
use std::fmt;
use std::net::SocketAddr;

use rand::Rng;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Utf8Bytes;

/// Signaling server errors
#[derive(Debug, Error)]
pub enum SignalingError {
    #[error("room not found: {0}")]
    RoomNotFound(RoomCode),

    #[error("internal error: {0}")]
    Internal(String),
}

const ROOM_CODE_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
const ROOM_CODE_LEN: usize = 8;
const PEER_ID_LEN: usize = 13;
const HEX_CHARS: &[u8] = b"0123456789abcdef";

/// Room code: 8-byte fixed array
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoomCode {
    bytes: [u8; ROOM_CODE_LEN],
    len: u8,
}

impl RoomCode {
    pub fn generate() -> Self {
        let mut rng = rand::rng();
        let mut bytes = [0u8; ROOM_CODE_LEN];
        for byte in &mut bytes {
            *byte = ROOM_CODE_CHARS[rng.random_range(0..ROOM_CODE_CHARS.len())];
        }
        Self {
            bytes,
            len: ROOM_CODE_LEN as u8,
        }
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len as usize]).unwrap_or("")
    }
}

impl fmt::Display for RoomCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for RoomCode {
    fn from(s: &str) -> Self {
        let mut bytes = [0u8; ROOM_CODE_LEN];
        let src = s.as_bytes();
        let len = src.len().min(ROOM_CODE_LEN);
        bytes[..len].copy_from_slice(&src[..len]);
        Self {
            bytes,
            len: len as u8,
        }
    }
}

impl Serialize for RoomCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RoomCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = <&str>::deserialize(deserializer)?;
        Ok(RoomCode::from(s))
    }
}

/// Peer ID: 13-byte fixed array ("peer_" + 8 hex)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PeerId {
    bytes: [u8; PEER_ID_LEN],
    len: u8,
}

impl PeerId {
    pub fn generate() -> Self {
        let mut bytes = [0u8; PEER_ID_LEN];
        bytes[..5].copy_from_slice(b"peer_");

        let mut rng = rand::rng();
        let value: u32 = rng.random();

        for i in 0..8 {
            let nibble = ((value >> (28 - i * 4)) & 0xF) as usize;
            bytes[5 + i] = HEX_CHARS[nibble];
        }
        Self {
            bytes,
            len: PEER_ID_LEN as u8,
        }
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len as usize]).unwrap_or("")
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for PeerId {
    fn from(s: &str) -> Self {
        let mut bytes = [0u8; PEER_ID_LEN];
        let src = s.as_bytes();
        let len = src.len().min(PEER_ID_LEN);
        bytes[..len].copy_from_slice(&src[..len]);
        Self {
            bytes,
            len: len as u8,
        }
    }
}

impl Serialize for PeerId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for PeerId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = <&str>::deserialize(deserializer)?;
        Ok(PeerId::from(s))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: PeerId,
    pub public_addr: Option<SocketAddr>,
}

/// Wrapper for outbound WebSocket messages using tungstenite's Utf8Bytes.
#[derive(Debug, Clone)]
pub struct OutboundMessage(Utf8Bytes);

impl OutboundMessage {
    /// Create a new outbound message from any string type
    pub fn new(s: impl Into<Utf8Bytes>) -> Self {
        Self(s.into())
    }

    /// Get the inner Utf8Bytes for tungstenite Message::Text
    pub fn into_inner(self) -> Utf8Bytes {
        self.0
    }
}

impl From<String> for OutboundMessage {
    fn from(s: String) -> Self {
        Self(Utf8Bytes::from(s))
    }
}

#[derive(Debug)]
pub(crate) struct PeerState {
    pub info: PeerInfo,
    /// Channel for outbound messages to this peer.
    /// Uses OutboundMessage (Arc<str>) for O(1) broadcast cloning.
    pub tx: mpsc::UnboundedSender<OutboundMessage>,
}

#[derive(Debug)]
pub(crate) struct Room {
    pub peers: HashMap<PeerId, PeerState>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_code_generate_has_correct_length() {
        let code = RoomCode::generate();
        assert_eq!(code.as_str().len(), 8);
    }

    #[test]
    fn room_code_generate_uses_valid_chars() {
        let code = RoomCode::generate();
        let valid_chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
        for c in code.as_str().chars() {
            assert!(valid_chars.contains(&c), "Invalid char: {}", c);
        }
    }

    #[test]
    fn peer_id_generate_has_correct_format() {
        let peer_id = PeerId::generate();
        assert!(peer_id.as_str().starts_with("peer_"));
        assert_eq!(peer_id.as_str().len(), 13);
    }

    #[test]
    fn room_code_from_str() {
        let code = RoomCode::from("test1234");
        assert_eq!(code.as_str(), "test1234");
    }

    #[test]
    fn peer_id_from_str() {
        let peer_id = PeerId::from("peer_12345678");
        assert_eq!(peer_id.as_str(), "peer_12345678");
    }

    #[test]
    fn room_code_display() {
        let code = RoomCode::from("abc12345");
        assert_eq!(format!("{}", code), "abc12345");
    }

    #[test]
    fn peer_id_display() {
        let peer_id = PeerId::from("peer_abcd1234");
        assert_eq!(format!("{}", peer_id), "peer_abcd1234");
    }

    #[test]
    fn room_code_serialization() {
        let code = RoomCode::from("testcode");
        let json = serde_json::to_string(&code).unwrap();
        assert_eq!(json, "\"testcode\"");
    }

    #[test]
    fn peer_id_serialization() {
        let peer_id = PeerId::from("peer_test1234");
        let json = serde_json::to_string(&peer_id).unwrap();
        assert_eq!(json, "\"peer_test1234\"");
    }

    #[test]
    fn room_code_deserialization() {
        let code: RoomCode = serde_json::from_str("\"testcode\"").unwrap();
        assert_eq!(code.as_str(), "testcode");
    }

    #[test]
    fn peer_id_deserialization() {
        let peer_id: PeerId = serde_json::from_str("\"peer_test1234\"").unwrap();
        assert_eq!(peer_id.as_str(), "peer_test1234");
    }

    #[test]
    fn peer_info_serialization() {
        let peer_info = PeerInfo {
            id: PeerId::from("peer_abc12345"),
            public_addr: Some("127.0.0.1:8080".parse().unwrap()),
        };
        let json = serde_json::to_string(&peer_info).unwrap();
        assert!(json.contains("peer_abc12345"));
        assert!(json.contains("127.0.0.1:8080"));
    }

    #[test]
    fn room_code_is_copy() {
        let code = RoomCode::generate();
        let copy = code;
        assert_eq!(code.as_str(), copy.as_str());
    }

    #[test]
    fn peer_id_is_copy() {
        let id = PeerId::generate();
        let copy = id;
        assert_eq!(id.as_str(), copy.as_str());
    }
}
