use std::net::SocketAddr;
use std::sync::Arc;

use async_channel::{Receiver, Sender};
use tokio::net::UdpSocket;
use tracing::{debug, info, warn};

use crate::protocol::{BINDING_RESPONSE_SIZE, StunError, StunRequest, StunResponse};

pub const DEFAULT_PORT: u16 = 3478;

/// work item to be sent to the worker
struct WorkItem {
    data: [u8; 64], // STUN request is usually 20-48 bytes
    len: usize,
    client_addr: SocketAddr,
}

pub struct StunServer {
    socket: Arc<UdpSocket>,
    num_workers: usize,
}

impl StunServer {
    /// create and bind the server to the port
    pub async fn bind(addr: &str) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(addr).await?;
        let num_workers = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        info!("STUN server listening on {}", socket.local_addr()?);
        info!("Using {} worker tasks", num_workers);

        Ok(Self {
            socket: Arc::new(socket),
            num_workers,
        })
    }

    /// run the multi-task server
    ///
    /// - Main task: receives UDP packets and dispatches to workers
    /// - Worker tasks: process STUN requests and send responses
    pub async fn run(self) -> std::io::Result<()> {
        let (tx, rx): (Sender<WorkItem>, Receiver<WorkItem>) = async_channel::bounded(1024);

        for worker_id in 0..self.num_workers {
            let socket = self.socket.clone();
            let rx = rx.clone();

            tokio::spawn(async move {
                worker_loop(worker_id, socket, rx).await;
            });
        }

        let mut buf = [0u8; 64];
        loop {
            let (len, client_addr) = self.socket.recv_from(&mut buf).await?;

            debug!("Received {} bytes from {}", len, client_addr);

            let mut work_data = [0u8; 64];
            work_data[..len].copy_from_slice(&buf[..len]);

            let work_item = WorkItem {
                data: work_data,
                len,
                client_addr,
            };

            if tx.try_send(work_item).is_err() {
                warn!("Worker queue full, dropping packet");
            }
        }
    }

    /// single-threaded STUN server (for debugging/testing)
    pub async fn run_simple(&self) -> std::io::Result<()> {
        let mut buf = [0u8; 64];
        let mut response_buf = [0u8; BINDING_RESPONSE_SIZE];

        loop {
            let (len, client_addr) = self.socket.recv_from(&mut buf).await?;

            match handle_request(&buf[..len], client_addr, &mut response_buf) {
                Ok(response_len) => {
                    self.socket
                        .send_to(&response_buf[..response_len], client_addr)
                        .await?;
                }
                Err(e) => {
                    debug!("Request error: {}", e);
                }
            }
        }
    }
}

/// worker loop: receive work items from the channel and process them
///
/// With async-channel, multiple workers can call `rx.recv()` concurrently
/// without any Mutex. The channel internally handles fair distribution.
async fn worker_loop(_worker_id: usize, socket: Arc<UdpSocket>, rx: Receiver<WorkItem>) {
    let mut response_buf = [0u8; BINDING_RESPONSE_SIZE];

    while let Ok(work_item) = rx.recv().await {
        match handle_request(
            &work_item.data[..work_item.len],
            work_item.client_addr,
            &mut response_buf,
        ) {
            Ok(response_len) => {
                if let Err(e) = socket
                    .send_to(&response_buf[..response_len], work_item.client_addr)
                    .await
                {
                    warn!("Failed to send response: {}", e);
                }
            }
            Err(e) => {
                debug!("Request error from {}: {}", work_item.client_addr, e);
            }
        }
    }
}

/// handle the STUN request
///
/// # Errors
/// Returns `StunError` if parsing fails or the request is not supported
#[inline]
fn handle_request(
    data: &[u8],
    client_addr: SocketAddr,
    response_buf: &mut [u8; BINDING_RESPONSE_SIZE],
) -> Result<usize, StunError> {
    let request = StunRequest::parse(data)?;

    if !request.is_binding_request() {
        return Err(StunError::UnsupportedMessageType(request.msg_type));
    }

    let addr_v4 = match client_addr {
        SocketAddr::V4(v4) => v4,
        SocketAddr::V6(_) => return Err(StunError::Ipv6NotSupported),
    };

    let response = StunResponse::binding_response(request.transaction_id, addr_v4);
    response_buf.copy_from_slice(response.as_bytes());

    Ok(BINDING_RESPONSE_SIZE)
}
