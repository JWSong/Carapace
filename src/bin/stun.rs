use std::error::Error;
use std::net::SocketAddr;
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let socket = UdpSocket::bind("0.0.0.0:4000").await?;

    let mut peers: Vec<SocketAddr> = Vec::new();
    let mut buf = [0u8; 1024];

    loop {
        let (_len, addr) = socket.recv_from(&mut buf).await?;
        println!("Received handshake from: {}", addr);

        if !peers.contains(&addr) {
            peers.push(addr);
        }

        // Once we have 2 peers, introduce them
        if peers.len() == 2 {
            let peer_a = peers[0];
            let peer_b = peers[1];

            // Send B's address to A
            socket
                .send_to(peer_b.to_string().as_bytes(), peer_a)
                .await?;
            // Send A's address to B
            socket
                .send_to(peer_a.to_string().as_bytes(), peer_b)
                .await?;

            println!("Peers exchanged! Resetting...");
            peers.clear();
        }
    }
}
