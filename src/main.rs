use carapace::server::{DEFAULT_PORT, StunServer};
use carapace::signaling::{DEFAULT_SIGNALING_PORT, SignalingServer};
use tracing::{error, info};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    let stun_addr = format!("0.0.0.0:{}", DEFAULT_PORT);
    let signaling_addr = format!("0.0.0.0:{}", DEFAULT_SIGNALING_PORT);

    info!("Carapace P2P Server starting...");
    info!("STUN:      {}", stun_addr);
    info!("Signaling: {} (WebSocket)", signaling_addr);

    let stun_server = StunServer::bind(&stun_addr).await?;
    let signaling_server = SignalingServer::new();

    let stun_handle = tokio::spawn(async move {
        if let Err(e) = stun_server.run().await {
            error!("STUN server error: {}", e);
        }
    });

    let signaling_addr_clone = signaling_addr.clone();
    let signaling_handle = tokio::spawn(async move {
        if let Err(e) = signaling_server.run(&signaling_addr_clone).await {
            error!("Signaling server error: {}", e);
        }
    });

    tokio::signal::ctrl_c().await?;
    info!("Shutdown signal received, stopping servers...");

    stun_handle.abort();
    signaling_handle.abort();

    info!("Servers stopped. Goodbye!");
    Ok(())
}
