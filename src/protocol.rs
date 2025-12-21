use std::net::SocketAddrV4;

use thiserror::Error;

/// STUN protocol errors
///
/// Using an enum instead of string errors provides:
/// - Pattern matching for error handling
/// - Type safety (compiler catches typos)
/// - Self-documenting API (errors visible in signature)
#[derive(Debug, Error)]
pub enum StunError {
    #[error("message too short: expected at least {expected} bytes, got {actual}")]
    MessageTooShort { expected: usize, actual: usize },

    #[error("invalid magic cookie: expected 0x{expected:08X}, got 0x{actual:08X}")]
    InvalidMagicCookie { expected: u32, actual: u32 },

    #[error("unknown message type: 0x{0:04X}")]
    UnknownMessageType(u16),

    #[error("unsupported message type: {0:?}")]
    UnsupportedMessageType(MessageType),

    #[error("IPv6 is not supported yet")]
    Ipv6NotSupported,
}

/// STUN Magic Cookie (RFC 5389)
pub const MAGIC_COOKIE: u32 = 0x2112A442;

/// STUN Header size in bytes
pub const HEADER_SIZE: usize = 20;

/// Binding Response size: 20 (header) + 12 (XOR-MAPPED-ADDRESS for IPv4)
pub const BINDING_RESPONSE_SIZE: usize = 32;

/// STUN Request
#[derive(Debug)]
pub struct StunRequest<'a> {
    pub msg_type: MessageType,
    pub transaction_id: &'a [u8],
}

impl<'a> StunRequest<'a> {
    /// Parse a STUN request from raw bytes
    ///
    /// # Errors
    /// - `StunError::MessageTooShort` - if data is less than 20 bytes
    /// - `StunError::InvalidMagicCookie` - if magic cookie doesn't match
    /// - `StunError::UnknownMessageType` - if message type is not recognized
    #[inline]
    pub fn parse(data: &'a [u8]) -> Result<Self, StunError> {
        if data.len() < HEADER_SIZE {
            return Err(StunError::MessageTooShort {
                expected: HEADER_SIZE,
                actual: data.len(),
            });
        }

        let msg_type_raw = u16::from_be_bytes([data[0], data[1]]);
        let msg_type = MessageType::from_u16(msg_type_raw)
            .ok_or(StunError::UnknownMessageType(msg_type_raw))?;

        let cookie = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        if cookie != MAGIC_COOKIE {
            return Err(StunError::InvalidMagicCookie {
                expected: MAGIC_COOKIE,
                actual: cookie,
            });
        }

        let transaction_id = &data[8..20];

        Ok(Self {
            msg_type,
            transaction_id,
        })
    }

    #[inline]
    pub fn is_binding_request(&self) -> bool {
        self.msg_type == MessageType::BindingRequest
    }
}

/// STUN Response
#[derive(Debug)]
pub struct StunResponse {
    buffer: [u8; BINDING_RESPONSE_SIZE],
}

impl StunResponse {
    /// create a binding response
    #[inline]
    pub fn binding_response(transaction_id: &[u8], client_addr: SocketAddrV4) -> Self {
        let mut buffer = [0u8; BINDING_RESPONSE_SIZE];

        buffer[0] = 0x01;
        buffer[1] = 0x01;
        buffer[2] = 0x00;
        buffer[3] = 0x0C;
        buffer[4..8].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
        buffer[8..20].copy_from_slice(transaction_id);

        buffer[20] = 0x00;
        buffer[21] = 0x20;
        buffer[22] = 0x00;
        buffer[23] = 0x08;
        buffer[24] = 0x00;
        buffer[25] = 0x01;

        let xor_port = client_addr.port() ^ ((MAGIC_COOKIE >> 16) as u16);
        buffer[26..28].copy_from_slice(&xor_port.to_be_bytes());

        let ip_bytes = client_addr.ip().octets();
        let magic_bytes = MAGIC_COOKIE.to_be_bytes();
        buffer[28] = ip_bytes[0] ^ magic_bytes[0];
        buffer[29] = ip_bytes[1] ^ magic_bytes[1];
        buffer[30] = ip_bytes[2] ^ magic_bytes[2];
        buffer[31] = ip_bytes[3] ^ magic_bytes[3];

        Self { buffer }
    }

    /// return the response bytes slice
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }
}

/// STUN Message Types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageType {
    BindingRequest,
    BindingResponse,
    BindingErrorResponse,
}

impl MessageType {
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            0x0001 => Some(MessageType::BindingRequest),
            0x0101 => Some(MessageType::BindingResponse),
            0x0111 => Some(MessageType::BindingErrorResponse),
            _ => None,
        }
    }

    pub fn to_u16(self) -> u16 {
        match self {
            MessageType::BindingRequest => 0x0001,
            MessageType::BindingResponse => 0x0101,
            MessageType::BindingErrorResponse => 0x0111,
        }
    }
}
