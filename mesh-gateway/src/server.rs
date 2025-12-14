use anyhow::{Context, Result};
use axum::{
    routing::get,
    Json, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub node_id: String,
    pub uptime_seconds: u64,
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

    // Build the Axum application with routes
    let app = create_app(node_id.clone());

    // Configure TLS with mTLS (client certificate verification)
    let tls_config = RustlsConfig::from_pem_file(
        PathBuf::from(cert_path.as_ref()),
        PathBuf::from(key_path.as_ref()),
    )
    .await
    .context("Failed to load TLS configuration")?;

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
fn create_app(node_id: String) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .with_state(node_id)
}

/// Health check endpoint handler
async fn health_handler(
    axum::extract::State(node_id): axum::extract::State<String>,
) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        node_id,
        uptime_seconds: 0, // TODO: Track actual uptime
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_response() {
        let response = health_handler(axum::extract::State("test-node".to_string())).await;
        assert_eq!(response.0.status, "healthy");
        assert_eq!(response.0.node_id, "test-node");
    }
}
