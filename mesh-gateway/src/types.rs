use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Represents information about a peer gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub node_id: String,
    pub address: String,
    pub status: PeerStatus,
    pub last_seen: Option<SystemTime>,
}

/// Status of a peer connection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PeerStatus {
    Connected,
    Disconnected,
    Unknown,
}

/// Request to send a message to another gateway
#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub to: String,
    pub content: String,
}

/// Response after sending a message
#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageResponse {
    pub status: String,
    pub route: Vec<String>,
}

/// Information about this gateway node
#[derive(Debug, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: String,
    pub listen_addr: String,
    pub peers: Vec<String>,
    pub version: String,
}

/// List of peers response
#[derive(Debug, Serialize, Deserialize)]
pub struct PeersResponse {
    pub peers: Vec<PeerInfo>,
}

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub node_id: String,
    pub uptime_seconds: u64,
}
