use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Custom serializer for SystemTime to RFC3339/ISO 8601 format
mod systemtime_serialization {
    use super::*;
    use serde::ser::Serializer;
    use serde::de::Deserializer;

    pub fn serialize<S>(time: &Option<SystemTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match time {
            Some(t) => {
                let duration = t.duration_since(UNIX_EPOCH).map_err(serde::ser::Error::custom)?;
                let datetime = time::OffsetDateTime::from_unix_timestamp(duration.as_secs() as i64)
                    .map_err(serde::ser::Error::custom)?;
                let formatted = datetime
                    .format(&time::format_description::well_known::Rfc3339)
                    .map_err(serde::ser::Error::custom)?;
                serializer.serialize_str(&formatted)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<SystemTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<String> = Option::deserialize(deserializer)?;
        match opt {
            Some(s) => {
                let datetime = time::OffsetDateTime::parse(&s, &time::format_description::well_known::Rfc3339)
                    .map_err(serde::de::Error::custom)?;
                let duration = std::time::Duration::from_secs(datetime.unix_timestamp() as u64);
                Ok(Some(UNIX_EPOCH + duration))
            }
            None => Ok(None),
        }
    }
}

/// Represents information about a peer gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub node_id: String,
    pub address: String,
    pub status: PeerStatus,
    #[serde(with = "systemtime_serialization")]
    pub last_seen: Option<SystemTime>,
}

/// Status of a peer connection
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
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

/// Request to receive a forwarded message from another gateway
#[derive(Debug, Serialize, Deserialize)]
pub struct ReceiveMessageRequest {
    pub from: String,
    pub to: String,
    pub content: String,
    pub route: Vec<String>,
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

/// Link State Advertisement - shares topology information with peers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkStateAdvertisement {
    /// The node that originated this LSA
    pub node_id: String,

    /// List of direct peers (neighbors) this node can reach
    pub neighbors: Vec<String>,

    /// Sequence number to detect newer LSAs (higher is newer)
    pub sequence: u64,

    /// Timestamp when this LSA was created
    #[serde(with = "systemtime_serialization")]
    pub timestamp: Option<SystemTime>,
}

/// Response when receiving an LSA
#[derive(Debug, Serialize, Deserialize)]
pub struct LsaResponse {
    pub status: String,
    pub message: String,
}
