use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Configuration for a gateway node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Unique identifier for this gateway
    pub node_id: String,

    /// Port to listen on
    pub listen_port: u16,

    /// Path to certificate file
    #[serde(default = "default_cert_path")]
    pub cert_path: String,

    /// Path to private key file
    #[serde(default = "default_key_path")]
    pub key_path: String,

    /// Path to CA certificate
    #[serde(default = "default_ca_cert_path")]
    pub ca_cert_path: String,

    /// List of peer gateways
    #[serde(default)]
    pub peers: Vec<PeerConfig>,
}

/// Configuration for a peer gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerConfig {
    /// Node ID of the peer
    pub node_id: String,

    /// Address of the peer (host:port)
    pub address: String,
}

fn default_cert_path() -> String {
    "certs/gateway.crt".to_string()
}

fn default_key_path() -> String {
    "certs/gateway.key".to_string()
}

fn default_ca_cert_path() -> String {
    "certs/ca.crt".to_string()
}

impl GatewayConfig {
    /// Load configuration from a TOML file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let contents = fs::read_to_string(path.as_ref())
            .context(format!("Failed to read config file: {:?}", path.as_ref()))?;

        let config: GatewayConfig = toml::from_str(&contents)
            .context("Failed to parse TOML configuration")?;

        // Override cert paths if they use the gateway's node_id
        let mut config = config;
        if config.cert_path == default_cert_path() {
            config.cert_path = format!("certs/{}.crt", config.node_id);
        }
        if config.key_path == default_key_path() {
            config.key_path = format!("certs/{}.key", config.node_id);
        }

        Ok(config)
    }

    /// Get the listen address
    pub fn listen_addr(&self) -> String {
        format!("127.0.0.1:{}", self.listen_port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let toml = r#"
            node_id = "gateway-a"
            listen_port = 8001

            [[peers]]
            node_id = "gateway-b"
            address = "127.0.0.1:8002"

            [[peers]]
            node_id = "gateway-c"
            address = "127.0.0.1:8003"
        "#;

        let config: GatewayConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.node_id, "gateway-a");
        assert_eq!(config.listen_port, 8001);
        assert_eq!(config.peers.len(), 2);
        assert_eq!(config.peers[0].node_id, "gateway-b");
        assert_eq!(config.peers[0].address, "127.0.0.1:8002");
    }

    #[test]
    fn test_default_paths() {
        let toml = r#"
            node_id = "gateway-a"
            listen_port = 8001
        "#;

        let config: GatewayConfig = toml::from_str(toml).unwrap();
        // Before calling from_file, defaults are generic
        assert_eq!(config.cert_path, "certs/gateway.crt");
        assert_eq!(config.key_path, "certs/gateway.key");
        assert_eq!(config.ca_cert_path, "certs/ca.crt");
    }

    #[test]
    fn test_config_from_file() {
        let config = GatewayConfig::from_file("configs/gateway-a.toml").unwrap();
        assert_eq!(config.node_id, "gateway-a");
        assert_eq!(config.listen_port, 8001);
        assert_eq!(config.cert_path, "certs/gateway-a.crt");
        assert_eq!(config.key_path, "certs/gateway-a.key");
        assert_eq!(config.ca_cert_path, "certs/ca.crt");
        assert_eq!(config.peers.len(), 2);
    }
}
