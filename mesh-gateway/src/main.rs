use anyhow::Result;
use clap::Parser;
use mesh_gateway::client::create_mtls_client;
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

    // TODO: Remove this once health checks are implemented in Phase 4
    // For now, mark all peers as connected to enable message forwarding
    routing_table.mark_all_connected();
    tracing::info!("✓ All peers marked as connected (temporary until health checks)");

    // Create mTLS HTTP client for communicating with peers
    tracing::info!("🔐 Creating mTLS HTTP client...");
    let http_client = create_mtls_client(
        &config.cert_path,
        &config.key_path,
        &config.ca_cert_path,
    )?;
    tracing::info!("✓ mTLS client ready");

    let listen_addr: SocketAddr = config.listen_addr().parse()?;

    // Spawn background task for LSA broadcasts
    tracing::info!("🔄 Starting link-state routing protocol...");
    mesh_gateway::server::spawn_lsa_broadcast_task(
        config.node_id.clone(),
        routing_table.clone(),
        http_client.clone(),
    );
    tracing::info!("✓ LSA broadcast task started (30s interval)");

    // Start the HTTPS server
    mesh_gateway::server::start_server(
        config.node_id,
        listen_addr,
        config.cert_path,
        config.key_path,
        config.ca_cert_path,
        routing_table,
        http_client,
    )
    .await?;

    Ok(())
}
