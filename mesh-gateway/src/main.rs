use anyhow::Result;
use clap::Parser;
use std::net::SocketAddr;

#[derive(Parser, Debug)]
#[command(name = "mesh-gateway")]
#[command(about = "Zero-trust mesh gateway network", long_about = None)]
struct Args {
    /// Node ID for this gateway
    #[arg(short, long, default_value = "gateway-a")]
    node_id: String,

    /// Port to listen on
    #[arg(short, long, default_value = "8001")]
    port: u16,

    /// Path to certificate file
    #[arg(long, default_value = "certs/gateway-a.crt")]
    cert: String,

    /// Path to private key file
    #[arg(long, default_value = "certs/gateway-a.key")]
    key: String,

    /// Path to CA certificate
    #[arg(long, default_value = "certs/ca.crt")]
    ca_cert: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    tracing::info!("🚀 Starting Mesh Gateway: {}", args.node_id);
    tracing::info!("📁 Certificate: {}", args.cert);
    tracing::info!("🔐 Private Key: {}", args.key);
    tracing::info!("🏛️  CA Certificate: {}", args.ca_cert);

    let listen_addr: SocketAddr = format!("127.0.0.1:{}", args.port).parse()?;

    // Start the HTTPS server
    mesh_gateway::server::start_server(
        args.node_id,
        listen_addr,
        args.cert,
        args.key,
        args.ca_cert,
    )
    .await?;

    Ok(())
}
