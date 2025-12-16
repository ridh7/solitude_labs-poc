use crate::routing::RoutingTable;
use crate::types::{HealthResponse, LinkStateAdvertisement, LsaResponse, NodeInfo, PeersResponse, ReceiveMessageRequest, SendMessageRequest, SendMessageResponse};
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
use std::time::Duration;
use tokio::time;

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
        .route("/topology/lsa", post(lsa_handler))
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

    // Find route to destination using link-state routing
    let route = state.routing_table.find_route_from(&state.node_id, &request.to);

    match route {
        Some(route_path) => {
            // Get next hop (first node in route)
            let next_hop = &route_path[0];

            // Get peer info to find address
            let peer_info = state.routing_table.get_peer(next_hop);

            if let Some(peer) = peer_info {
                // Build initial route with just the current node (sender)
                // Each hop will add itself when forwarding
                let full_route = vec![state.node_id.clone()];

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
                            // Parse the response to get the actual route taken
                            match response.json::<SendMessageResponse>().await {
                                Ok(send_response) => {
                                    tracing::info!("Message forwarded to {} via {}", request.to, next_hop);
                                    Json(send_response)
                                }
                                Err(e) => {
                                    tracing::error!("Failed to parse response from {}: {}", next_hop, e);
                                    Json(SendMessageResponse {
                                        status: "failed".to_string(),
                                        route: vec![state.node_id.clone()],
                                    })
                                }
                            }
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
        // Add ourselves to the route to show final destination
        let mut final_route = request.route;
        final_route.push(state.node_id.clone());
        return Json(SendMessageResponse {
            status: "delivered".to_string(),
            route: final_route,
        });
    }

    // Multi-hop forwarding: message is not for us, try to forward it

    // Check if we've already seen this message (loop prevention)
    if request.route.contains(&state.node_id) {
        tracing::warn!(
            "Loop detected: {} already in route {:?}. Dropping message.",
            state.node_id,
            request.route
        );
        return Json(SendMessageResponse {
            status: "loop_detected".to_string(),
            route: request.route,
        });
    }

    // Try to find a route to the destination
    let route = state.routing_table.find_route_from(&state.node_id, &request.to);

    match route {
        Some(route_path) => {
            let next_hop = &route_path[0];

            // Get peer info
            let peer_info = state.routing_table.get_peer(next_hop);

            if let Some(peer) = peer_info {
                // Build updated route including current node
                let mut updated_route = request.route.clone();
                updated_route.push(state.node_id.clone());

                // Forward message to next hop
                let forward_request = ReceiveMessageRequest {
                    from: state.node_id.clone(),
                    to: request.to.clone(),
                    content: request.content.clone(),
                    route: updated_route.clone(),
                };

                let url = format!("https://{}/message/receive", peer.address);

                match state.http_client
                    .post(&url)
                    .json(&forward_request)
                    .send()
                    .await
                {
                    Ok(response) => {
                        if response.status().is_success() {
                            // Parse the response to get the actual route taken
                            match response.json::<SendMessageResponse>().await {
                                Ok(send_response) => {
                                    tracing::info!(
                                        "Multi-hop: Message for {} forwarded to {} (next hop: {})",
                                        request.to,
                                        next_hop,
                                        next_hop
                                    );
                                    Json(send_response)
                                }
                                Err(e) => {
                                    tracing::error!("Multi-hop: Failed to parse response from {}: {}", next_hop, e);
                                    Json(SendMessageResponse {
                                        status: "failed".to_string(),
                                        route: updated_route,
                                    })
                                }
                            }
                        } else {
                            tracing::error!(
                                "Multi-hop: Failed to forward message to {}: HTTP {}",
                                next_hop,
                                response.status()
                            );
                            Json(SendMessageResponse {
                                status: "failed".to_string(),
                                route: updated_route,
                            })
                        }
                    }
                    Err(e) => {
                        tracing::error!("Multi-hop: Failed to forward message to {}: {}", next_hop, e);
                        Json(SendMessageResponse {
                            status: "failed".to_string(),
                            route: updated_route,
                        })
                    }
                }
            } else {
                tracing::error!("Multi-hop: Peer {} not found in routing table", next_hop);
                Json(SendMessageResponse {
                    status: "no_route".to_string(),
                    route: request.route,
                })
            }
        }
        None => {
            tracing::warn!(
                "Multi-hop: No route to {} from {}. Message cannot be forwarded.",
                request.to,
                state.node_id
            );
            Json(SendMessageResponse {
                status: "no_route".to_string(),
                route: request.route,
            })
        }
    }
}

/// LSA handler - receives Link State Advertisements from peers
async fn lsa_handler(
    State(state): State<AppState>,
    Json(lsa): Json<LinkStateAdvertisement>,
) -> Json<LsaResponse> {
    tracing::info!(
        "Received LSA from {} (seq: {}, neighbors: {:?})",
        lsa.node_id,
        lsa.sequence,
        lsa.neighbors
    );

    // Process the LSA
    let is_new = state.routing_table.process_lsa(lsa.clone());

    if is_new {
        tracing::info!("New LSA processed from {}, flooding to neighbors", lsa.node_id);

        // Flood LSA to all connected peers (OSPF-style flooding)
        // This ensures rapid topology propagation across the mesh
        let peers = state.routing_table.get_connected_peers();
        let lsa_clone = lsa.clone();
        let client_clone = state.http_client.clone();

        // Spawn flooding task to not block the response
        tokio::spawn(async move {
            for peer in peers {
                // Skip flooding back to the originator
                if peer.node_id == lsa_clone.node_id {
                    continue;
                }

                let url = format!("https://{}/topology/lsa", peer.address);
                let lsa_to_send = lsa_clone.clone();
                let client = client_clone.clone();

                // Flood to each peer in parallel
                tokio::spawn(async move {
                    match client.post(&url).json(&lsa_to_send).send().await {
                        Ok(response) => {
                            if response.status().is_success() {
                                tracing::debug!("Flooded LSA from {} to {}", lsa_to_send.node_id, peer.node_id);
                            } else {
                                tracing::warn!(
                                    "Failed to flood LSA to {}: HTTP {}",
                                    peer.node_id,
                                    response.status()
                                );
                            }
                        }
                        Err(e) => {
                            tracing::debug!("Failed to flood LSA to {}: {}", peer.node_id, e);
                        }
                    }
                });
            }
        });

        Json(LsaResponse {
            status: "accepted".to_string(),
            message: format!("LSA from {} accepted and flooded", lsa.node_id),
        })
    } else {
        tracing::debug!("Duplicate or old LSA from {}, ignored", lsa.node_id);
        Json(LsaResponse {
            status: "ignored".to_string(),
            message: format!("LSA from {} already known or outdated", lsa.node_id),
        })
    }
}

/// Spawns a background task that periodically broadcasts LSAs to all connected peers
pub fn spawn_lsa_broadcast_task(
    node_id: String,
    routing_table: RoutingTable,
    http_client: Client,
) {
    tokio::spawn(async move {
        // Wait a bit before starting to let the network stabilize
        time::sleep(Duration::from_secs(5)).await;

        let mut interval = time::interval(Duration::from_secs(30));

        loop {
            interval.tick().await;

            // Generate our LSA
            let lsa = routing_table.generate_lsa(&node_id);
            tracing::debug!(
                "Broadcasting LSA (seq: {}, neighbors: {:?})",
                lsa.sequence,
                lsa.neighbors
            );

            // Get all connected peers
            let peers = routing_table.get_connected_peers();

            // Send LSA to each peer
            for peer in peers {
                let url = format!("https://{}/topology/lsa", peer.address);
                let lsa_clone = lsa.clone();
                let client_clone = http_client.clone();

                // Spawn a task for each peer to send in parallel
                tokio::spawn(async move {
                    match client_clone.post(&url).json(&lsa_clone).send().await {
                        Ok(response) => {
                            if response.status().is_success() {
                                tracing::debug!("LSA sent to {}", peer.node_id);
                            } else {
                                tracing::warn!(
                                    "Failed to send LSA to {}: HTTP {}",
                                    peer.node_id,
                                    response.status()
                                );
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to send LSA to {}: {}", peer.node_id, e);
                        }
                    }
                });
            }
        }
    });
}

/// Spawns a background task that periodically checks peer health
pub fn spawn_health_check_task(
    routing_table: RoutingTable,
    http_client: Client,
) {
    tokio::spawn(async move {
        // Wait before starting initial health checks
        time::sleep(Duration::from_secs(10)).await;

        let mut interval = time::interval(Duration::from_secs(15));

        loop {
            interval.tick().await;

            // Get all peers (both connected and disconnected)
            let peers = routing_table.get_all_peers();

            for peer in peers {
                let url = format!("https://{}/health", peer.address);
                let peer_node_id = peer.node_id.clone();
                let routing_table_clone = routing_table.clone();
                let client_clone = http_client.clone();

                // Check each peer in parallel
                tokio::spawn(async move {
                    // Set timeout for health check
                    let timeout_duration = Duration::from_secs(5);

                    match tokio::time::timeout(
                        timeout_duration,
                        client_clone.get(&url).send()
                    ).await {
                        Ok(Ok(response)) => {
                            if response.status().is_success() {
                                // Peer is healthy
                                let current_status = routing_table_clone.get_peer(&peer_node_id)
                                    .map(|p| p.status);

                                if current_status != Some(crate::types::PeerStatus::Connected) {
                                    tracing::info!("Peer {} is now reachable", peer_node_id);
                                }

                                routing_table_clone.update_peer_status(
                                    &peer_node_id,
                                    crate::types::PeerStatus::Connected
                                );
                            } else {
                                // Peer returned non-success status
                                tracing::warn!(
                                    "Health check failed for {}: HTTP {}",
                                    peer_node_id,
                                    response.status()
                                );
                                routing_table_clone.update_peer_status(
                                    &peer_node_id,
                                    crate::types::PeerStatus::Disconnected
                                );
                            }
                        }
                        Ok(Err(e)) => {
                            // Request failed
                            tracing::debug!("Health check failed for {}: {}", peer_node_id, e);
                            routing_table_clone.update_peer_status(
                                &peer_node_id,
                                crate::types::PeerStatus::Disconnected
                            );
                        }
                        Err(_) => {
                            // Timeout
                            tracing::debug!("Health check timeout for {}", peer_node_id);
                            routing_table_clone.update_peer_status(
                                &peer_node_id,
                                crate::types::PeerStatus::Disconnected
                            );
                        }
                    }
                });
            }
        }
    });
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
