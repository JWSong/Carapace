use serde::{Deserialize, Serialize};

use super::types::{PeerId, PeerInfo, RoomCode};

/// Messages sent from client to server
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Create a new room (becomes the first peer)
    #[serde(rename = "create_room")]
    CreateRoom,

    /// Join an existing room by code
    #[serde(rename = "join_room")]
    JoinRoom { code: String },

    /// Leave the current room
    #[serde(rename = "leave_room")]
    LeaveRoom,
}

/// Messages sent from server to client
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Room created successfully
    #[serde(rename = "room_created")]
    RoomCreated { code: RoomCode, your_id: PeerId },

    /// Joined room successfully (includes existing peers with their addresses)
    #[serde(rename = "room_joined")]
    RoomJoined {
        code: RoomCode,
        your_id: PeerId,
        peers: Vec<PeerInfo>,
    },

    /// A new peer joined the room (use their address for P2P connection)
    #[serde(rename = "peer_joined")]
    PeerJoined { peer: PeerInfo },

    /// Error response
    #[serde(rename = "error")]
    Error { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_create_room() {
        let json = r#"{"type": "create_room"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        matches!(msg, ClientMessage::CreateRoom);
    }

    #[test]
    fn parse_join_room() {
        let json = r#"{"type": "join_room", "code": "abc12345"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        if let ClientMessage::JoinRoom { code } = msg {
            assert_eq!(code, "abc12345");
        } else {
            panic!("Expected JoinRoom");
        }
    }

    #[test]
    fn parse_leave_room() {
        let json = r#"{"type": "leave_room"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        matches!(msg, ClientMessage::LeaveRoom);
    }

    #[test]
    fn serialize_room_created() {
        let msg = ServerMessage::RoomCreated {
            code: RoomCode::from("test1234"),
            your_id: PeerId::from("peer_abc12345"),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("room_created"));
        assert!(json.contains("test1234"));
        assert!(json.contains("peer_abc12345"));
    }

    #[test]
    fn serialize_room_joined() {
        let msg = ServerMessage::RoomJoined {
            code: RoomCode::from("test1234"),
            your_id: PeerId::from("peer_new12345"),
            peers: vec![PeerInfo {
                id: PeerId::from("peer_existing"),
                public_addr: None,
            }],
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("room_joined"));
        assert!(json.contains("peer_existing"));
    }

    #[test]
    fn serialize_peer_joined() {
        let msg = ServerMessage::PeerJoined {
            peer: PeerInfo {
                id: PeerId::from("peer_new12345"),
                public_addr: Some("192.168.1.1:5000".parse().unwrap()),
            },
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("peer_joined"));
        assert!(json.contains("peer_new12345"));
        assert!(json.contains("192.168.1.1:5000"));
    }

    #[test]
    fn serialize_error() {
        let msg = ServerMessage::Error {
            message: "Room not found".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("Room not found"));
    }
}
