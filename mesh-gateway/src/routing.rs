use crate::config::PeerConfig;
use crate::types::{PeerInfo, PeerStatus};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

/// Thread-safe routing table for tracking peers in the mesh network
#[derive(Clone)]
pub struct RoutingTable {
    inner: Arc<RwLock<RoutingTableInner>>,
}

struct RoutingTableInner {
    /// Map of node_id -> PeerInfo
    peers: HashMap<String, PeerInfo>,
}

impl RoutingTable {
    /// Create a new empty routing table
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RoutingTableInner {
                peers: HashMap::new(),
            })),
        }
    }

    /// Initialize routing table from configuration
    pub fn from_config(peer_configs: Vec<PeerConfig>) -> Self {
        let mut peers = HashMap::new();

        for peer_config in peer_configs {
            let peer_info = PeerInfo {
                node_id: peer_config.node_id.clone(),
                address: peer_config.address,
                status: PeerStatus::Unknown,
                last_seen: None,
            };
            peers.insert(peer_config.node_id, peer_info);
        }

        Self {
            inner: Arc::new(RwLock::new(RoutingTableInner { peers })),
        }
    }

    /// Add or update a peer in the routing table
    pub fn add_peer(&self, peer: PeerInfo) {
        let mut inner = self.inner.write().unwrap();
        inner.peers.insert(peer.node_id.clone(), peer);
    }

    /// Update peer status
    pub fn update_peer_status(&self, node_id: &str, status: PeerStatus) {
        let mut inner = self.inner.write().unwrap();
        if let Some(peer) = inner.peers.get_mut(node_id) {
            peer.status = status;
            if status == PeerStatus::Connected {
                peer.last_seen = Some(SystemTime::now());
            }
        }
    }

    /// Mark a peer as seen (updates last_seen timestamp)
    pub fn mark_peer_seen(&self, node_id: &str) {
        let mut inner = self.inner.write().unwrap();
        if let Some(peer) = inner.peers.get_mut(node_id) {
            peer.last_seen = Some(SystemTime::now());
        }
    }

    /// Get information about a specific peer
    pub fn get_peer(&self, node_id: &str) -> Option<PeerInfo> {
        let inner = self.inner.read().unwrap();
        inner.peers.get(node_id).cloned()
    }

    /// Get all peers
    pub fn get_all_peers(&self) -> Vec<PeerInfo> {
        let inner = self.inner.read().unwrap();
        inner.peers.values().cloned().collect()
    }

    /// Get all connected peers
    pub fn get_connected_peers(&self) -> Vec<PeerInfo> {
        let inner = self.inner.read().unwrap();
        inner
            .peers
            .values()
            .filter(|p| p.status == PeerStatus::Connected)
            .cloned()
            .collect()
    }

    /// Remove a peer from the routing table
    pub fn remove_peer(&self, node_id: &str) -> Option<PeerInfo> {
        let mut inner = self.inner.write().unwrap();
        inner.peers.remove(node_id)
    }

    /// Get the number of peers
    pub fn peer_count(&self) -> usize {
        let inner = self.inner.read().unwrap();
        inner.peers.len()
    }

    /// Find a route to a destination node
    /// Returns a vector of node_ids representing the path
    /// For now, implements simple direct routing (single hop)
    pub fn find_route(&self, destination: &str) -> Option<Vec<String>> {
        let inner = self.inner.read().unwrap();

        // Check if destination is a direct peer
        if let Some(peer) = inner.peers.get(destination) {
            if peer.status == PeerStatus::Connected {
                return Some(vec![destination.to_string()]);
            }
        }

        // TODO: Implement multi-hop routing using graph algorithms
        None
    }
}

impl Default for RoutingTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_routing_table() {
        let table = RoutingTable::new();
        assert_eq!(table.peer_count(), 0);
    }

    #[test]
    fn test_add_and_get_peer() {
        let table = RoutingTable::new();

        let peer = PeerInfo {
            node_id: "gateway-b".to_string(),
            address: "127.0.0.1:8002".to_string(),
            status: PeerStatus::Connected,
            last_seen: Some(SystemTime::now()),
        };

        table.add_peer(peer.clone());
        assert_eq!(table.peer_count(), 1);

        let retrieved = table.get_peer("gateway-b").unwrap();
        assert_eq!(retrieved.node_id, "gateway-b");
        assert_eq!(retrieved.address, "127.0.0.1:8002");
    }

    #[test]
    fn test_from_config() {
        let peers = vec![
            PeerConfig {
                node_id: "gateway-b".to_string(),
                address: "127.0.0.1:8002".to_string(),
            },
            PeerConfig {
                node_id: "gateway-c".to_string(),
                address: "127.0.0.1:8003".to_string(),
            },
        ];

        let table = RoutingTable::from_config(peers);
        assert_eq!(table.peer_count(), 2);

        let peer_b = table.get_peer("gateway-b").unwrap();
        assert_eq!(peer_b.status, PeerStatus::Unknown);
    }

    #[test]
    fn test_update_peer_status() {
        let table = RoutingTable::new();

        let peer = PeerInfo {
            node_id: "gateway-b".to_string(),
            address: "127.0.0.1:8002".to_string(),
            status: PeerStatus::Unknown,
            last_seen: None,
        };

        table.add_peer(peer);
        table.update_peer_status("gateway-b", PeerStatus::Connected);

        let updated = table.get_peer("gateway-b").unwrap();
        assert_eq!(updated.status, PeerStatus::Connected);
        assert!(updated.last_seen.is_some());
    }

    #[test]
    fn test_get_connected_peers() {
        let table = RoutingTable::new();

        table.add_peer(PeerInfo {
            node_id: "gateway-b".to_string(),
            address: "127.0.0.1:8002".to_string(),
            status: PeerStatus::Connected,
            last_seen: Some(SystemTime::now()),
        });

        table.add_peer(PeerInfo {
            node_id: "gateway-c".to_string(),
            address: "127.0.0.1:8003".to_string(),
            status: PeerStatus::Disconnected,
            last_seen: None,
        });

        let connected = table.get_connected_peers();
        assert_eq!(connected.len(), 1);
        assert_eq!(connected[0].node_id, "gateway-b");
    }

    #[test]
    fn test_find_direct_route() {
        let table = RoutingTable::new();

        table.add_peer(PeerInfo {
            node_id: "gateway-b".to_string(),
            address: "127.0.0.1:8002".to_string(),
            status: PeerStatus::Connected,
            last_seen: Some(SystemTime::now()),
        });

        let route = table.find_route("gateway-b");
        assert!(route.is_some());
        assert_eq!(route.unwrap(), vec!["gateway-b"]);

        let no_route = table.find_route("gateway-unknown");
        assert!(no_route.is_none());
    }

    #[test]
    fn test_remove_peer() {
        let table = RoutingTable::new();

        table.add_peer(PeerInfo {
            node_id: "gateway-b".to_string(),
            address: "127.0.0.1:8002".to_string(),
            status: PeerStatus::Connected,
            last_seen: Some(SystemTime::now()),
        });

        assert_eq!(table.peer_count(), 1);

        let removed = table.remove_peer("gateway-b");
        assert!(removed.is_some());
        assert_eq!(table.peer_count(), 0);
    }
}
