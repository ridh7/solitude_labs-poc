use crate::types::{HealthResponse, NodeInfo, PeersResponse, SendMessageRequest, SendMessageResponse};
use anyhow::{Context, Result};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use rustls::{server::AllowAnyAuthenticatedClient, ServerConfig};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use crate::certs::{load_ca_cert, load_cert, load_private_key};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub node_id: String,
    pub listen_addr: String,
    pub start_time: std::time::SystemTime,
}

impl AppState {
    pub fn new(node_id: String, listen_addr: String) -> Self {
        Self {
            node_id,
            listen_addr,
            start_time: std::time::SystemTime::now(),
        }
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.start_time
            .elapsed()
            .unwrap_or_default()
            .as_secs()
    }
}

/// Starts the HTTPS server with mTLS
pub async fn start_server(
    node_id: String,
    listen_addr: SocketAddr,
    cert_path: impl AsRef<Path>,
    key_path: impl AsRef<Path>,
    ca_cert_path: impl AsRef<Path>,
) -> Result<()> {
    tracing::info!("Starting HTTPS server on {}", listen_addr);

    // Create shared application state
    let state = AppState::new(node_id.clone(), listen_addr.to_string());

    // Build the Axum application with routes
    let app = create_app(state);

    // Configure mTLS
    // 1. Load CA certificate to verify clients
    let ca_store = load_ca_cert(&ca_cert_path)
        .context("Failed to load CA certificate")?;
    
    let client_verifier = AllowAnyAuthenticatedClient::new(ca_store);

    // 2. Load server certificate and private key
    let certs = load_cert(&cert_path)
        .context("Failed to load server certificate")?;
    let key = load_private_key(&key_path)
        .context("Failed to load server private key")?;

    // 3. Build Rustls configuration
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_client_cert_verifier(Arc::new(client_verifier))
        .with_single_cert(certs, key)
        .context("Failed to create TLS configuration")?;

    let tls_config = RustlsConfig::from_config(Arc::new(config));

    tracing::info!("TLS configured for node: {}", node_id);
    tracing::info!("Listening on https://{}", listen_addr);

    // Start the server
    axum_server::bind_rustls(listen_addr, tls_config)
        .serve(app.into_make_service())
        .await
        .context("Server error")?;

    Ok(())
}

/// Creates the Axum application with all routes
fn create_app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/peer/info", get(peer_info_handler))
        .route("/peers", get(peers_handler))
        .route("/message/send", post(send_message_handler))
        .with_state(state)
}

/// Health check endpoint handler
async fn health_handler(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        node_id: state.node_id.clone(),
        uptime_seconds: state.uptime_seconds(),
    })
}

/// Peer info endpoint - returns information about this gateway
async fn peer_info_handler(State(state): State<AppState>) -> Json<NodeInfo> {
    Json(NodeInfo {
        node_id: state.node_id.clone(),
        listen_addr: state.listen_addr.clone(),
        peers: vec![], // TODO: Add actual peer list
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// List all peers endpoint
async fn peers_handler(State(_state): State<AppState>) -> Json<PeersResponse> {
    // TODO: Return actual peer list from routing table
    Json(PeersResponse { peers: vec![] })
}

/// Send message endpoint
async fn send_message_handler(
    State(state): State<AppState>,
    Json(request): Json<SendMessageRequest>,
) -> Json<SendMessageResponse> {
    tracing::info!(
        "Received message for {}: {}",
        request.to,
        request.content
    );

    // TODO: Implement actual message routing
    Json(SendMessageResponse {
        status: "queued".to_string(),
        route: vec![state.node_id.clone()],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_response() {
        let state = AppState::new("test-node".to_string(), "127.0.0.1:8001".to_string());
        let response = health_handler(State(state)).await;
        assert_eq!(response.0.status, "healthy");
        assert_eq!(response.0.node_id, "test-node");
    }

    #[tokio::test]
    async fn test_peer_info() {
        let state = AppState::new("test-node".to_string(), "127.0.0.1:8001".to_string());
        let response = peer_info_handler(State(state)).await;
        assert_eq!(response.0.node_id, "test-node");
        assert_eq!(response.0.listen_addr, "127.0.0.1:8001");
    }
}
