use crate::routing::RoutingTable;
use crate::types::{HealthResponse, NodeInfo, PeersResponse, ReceiveMessageRequest, SendMessageRequest, SendMessageResponse};
use anyhow::{Context, Result};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use reqwest::Client;
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
    pub routing_table: RoutingTable,
    pub http_client: Client,
}

impl AppState {
    pub fn new(node_id: String, listen_addr: String, routing_table: RoutingTable, http_client: Client) -> Self {
        Self {
            node_id,
            listen_addr,
            start_time: std::time::SystemTime::now(),
            routing_table,
            http_client,
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
    routing_table: RoutingTable,
    http_client: Client,
) -> Result<()> {
    tracing::info!("Starting HTTPS server on {}", listen_addr);

    // Create shared application state
    let state = AppState::new(node_id.clone(), listen_addr.to_string(), routing_table, http_client);

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
        .route("/message/receive", post(receive_message_handler))
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
    let peer_ids: Vec<String> = state
        .routing_table
        .get_all_peers()
        .iter()
        .map(|p| p.node_id.clone())
        .collect();

    Json(NodeInfo {
        node_id: state.node_id.clone(),
        listen_addr: state.listen_addr.clone(),
        peers: peer_ids,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// List all peers endpoint
async fn peers_handler(State(state): State<AppState>) -> Json<PeersResponse> {
    let peers = state.routing_table.get_all_peers();
    Json(PeersResponse { peers })
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

    // Find route to destination
    let route = state.routing_table.find_route(&request.to);

    match route {
        Some(route_path) => {
            // Get next hop (first node in route)
            let next_hop = &route_path[0];

            // Get peer info to find address
            let peer_info = state.routing_table.get_peer(next_hop);

            if let Some(peer) = peer_info {
                // Build full route including current node
                let mut full_route = vec![state.node_id.clone()];
                full_route.extend(route_path.clone());

                // Forward message to next hop
                let forward_request = ReceiveMessageRequest {
                    from: state.node_id.clone(),
                    to: request.to.clone(),
                    content: request.content.clone(),
                    route: full_route.clone(),
                };

                // Note: peer.address is validated in config.rs to be in "host:port" format
                // without protocol prefix, so this URL construction is safe
                let url = format!("https://{}/message/receive", peer.address);

                match state.http_client
                    .post(&url)
                    .json(&forward_request)
                    .send()
                    .await
                {
                    Ok(response) => {
                        if response.status().is_success() {
                            tracing::info!("Message forwarded to {} via {}", request.to, next_hop);
                            Json(SendMessageResponse {
                                status: "delivered".to_string(),
                                route: full_route,
                            })
                        } else {
                            tracing::error!("Failed to forward message: HTTP {}", response.status());
                            Json(SendMessageResponse {
                                status: "failed".to_string(),
                                route: vec![state.node_id.clone()],
                            })
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to forward message to {}: {}", next_hop, e);
                        Json(SendMessageResponse {
                            status: "failed".to_string(),
                            route: vec![state.node_id.clone()],
                        })
                    }
                }
            } else {
                tracing::error!("Peer {} not found in routing table", next_hop);
                Json(SendMessageResponse {
                    status: "no_route".to_string(),
                    route: vec![state.node_id.clone()],
                })
            }
        }
        None => {
            tracing::warn!("No route found to {}", request.to);
            Json(SendMessageResponse {
                status: "no_route".to_string(),
                route: vec![state.node_id.clone()],
            })
        }
    }
}

/// Receive message endpoint - receives forwarded messages from other gateways
async fn receive_message_handler(
    State(state): State<AppState>,
    Json(request): Json<ReceiveMessageRequest>,
) -> Json<SendMessageResponse> {
    tracing::info!(
        "Received forwarded message from {} to {}: {}",
        request.from,
        request.to,
        request.content
    );

    // Check if this message is for us
    if request.to == state.node_id {
        tracing::info!("Message delivered to final destination: {}", request.content);
        Json(SendMessageResponse {
            status: "delivered".to_string(),
            route: request.route,
        })
    } else {
        // TODO: Multi-hop forwarding - forward to next hop
        tracing::warn!("Multi-hop routing not yet implemented. Message for {} cannot be forwarded.", request.to);
        Json(SendMessageResponse {
            status: "no_route".to_string(),
            route: request.route,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_response() {
        let routing_table = RoutingTable::new();
        let client = reqwest::Client::new();
        let state = AppState::new("test-node".to_string(), "127.0.0.1:8001".to_string(), routing_table, client);
        let response = health_handler(State(state)).await;
        assert_eq!(response.0.status, "healthy");
        assert_eq!(response.0.node_id, "test-node");
    }

    #[tokio::test]
    async fn test_peer_info() {
        let routing_table = RoutingTable::new();
        let client = reqwest::Client::new();
        let state = AppState::new("test-node".to_string(), "127.0.0.1:8001".to_string(), routing_table, client);
        let response = peer_info_handler(State(state)).await;
        assert_eq!(response.0.node_id, "test-node");
        assert_eq!(response.0.listen_addr, "127.0.0.1:8001");
    }
}
