use anyhow::Result;
use clap::Parser;
use mesh_gateway::config::GatewayConfig;
use mesh_gateway::routing::RoutingTable;
use std::net::SocketAddr;

#[derive(Parser, Debug)]
#[command(name = "mesh-gateway")]
#[command(about = "Zero-trust mesh gateway network", long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "configs/gateway-a.toml")]
    config: String,
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

    // Load configuration from file
    tracing::info!("📄 Loading configuration from: {}", args.config);
    let config = GatewayConfig::from_file(&args.config)?;

    tracing::info!("🚀 Starting Mesh Gateway: {}", config.node_id);
    tracing::info!("📁 Certificate: {}", config.cert_path);
    tracing::info!("🔐 Private Key: {}", config.key_path);
    tracing::info!("🏛️  CA Certificate: {}", config.ca_cert_path);
    tracing::info!("👥 Configured peers: {}", config.peers.len());

    // Create routing table from config
    let routing_table = RoutingTable::from_config(config.peers.clone());
    tracing::info!("🗺️  Routing table initialized with {} peers", routing_table.peer_count());

    let listen_addr: SocketAddr = config.listen_addr().parse()?;

    // Start the HTTPS server
    mesh_gateway::server::start_server(
        config.node_id,
        listen_addr,
        config.cert_path,
        config.key_path,
        config.ca_cert_path,
        routing_table,
    )
    .await?;

    Ok(())
}
