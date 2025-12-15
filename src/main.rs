use carapace::server::{DEFAULT_PORT, StunServer};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    let bind_addr = format!("0.0.0.0:{}", DEFAULT_PORT);

    println!("   Carapace STUN Server");
    println!("   Binding to {}", bind_addr);
    println!("   Press Ctrl+C to stop\n");

    let server = StunServer::bind(&bind_addr).await?;
    server.run().await
}
