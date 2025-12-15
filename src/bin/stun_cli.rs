use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt};
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Bind to ANY available port.
    let socket = UdpSocket::bind("0.0.0.0:0").await?;

    // Wrap in Arc for sharing between tasks
    let socket = Arc::new(socket);

    // 2. Connect to the STUN Server
    let server_addr = "0.0.0.0:4000";

    // The following could be authentication info or the like
    socket.send_to(b"HELLO", server_addr).await?;

    // 3. Wait for Server to reply with Peer's Address
    let mut buf = [0u8; 1024];
    let (len, _src) = socket.recv_from(&mut buf).await?;
    let peer_ip_str = String::from_utf8_lossy(&buf[..len]);
    let peer_addr = peer_ip_str.parse::<std::net::SocketAddr>()?;

    println!("Received Peer Address: {}", peer_addr);
    println!("Starting Hole Punching...");

    // 4. HOLE PUNCHING PHASE
    // Send a few dummy packets to the peer.
    // The first few will likely be dropped by the peer's NAT,
    // but they open the 'hole' in the NAT.
    for _ in 0..3 {
        socket.send_to(b"HOLE_PUNCH", peer_addr).await?;
    }

    println!("Hole punching packets sent. Chat is ready!");
    println!("Type a message and press Enter:");

    // 5. Chat Loop (Split into Read and Write tasks)
    let recv_socket = socket.clone();

    // Task 1: Listen for messages from Peer
    tokio::spawn(listen(recv_socket, peer_addr));

    // Task 2: Read stdin and send to Peer
    let mut stdin = io::BufReader::new(io::stdin()).lines();
    while let Ok(Some(line)) = stdin.next_line().await {
        socket.send_to(line.as_bytes(), peer_addr).await?;
    }

    Ok(())
}

async fn listen(socket: Arc<UdpSocket>, peer_addr: SocketAddr) {
    let mut buf = [0u8; 1024];
    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, src)) => {
                // Only accept messages from the actual peer
                if src == peer_addr {
                    let msg = String::from_utf8_lossy(&buf[..len]);
                    // Filter out the punch packets
                    if msg != "HOLE_PUNCH" {
                        println!("\n> Peer: {}", msg);
                    }
                }
            }
            Err(e) => eprintln!("Read error: {}", e),
        }
    }
}
