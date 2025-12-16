use crate::config::PeerConfig;
use crate::types::{LinkStateAdvertisement, PeerInfo, PeerStatus};
use std::collections::{HashMap, HashSet, BinaryHeap};
use std::cmp::Ordering;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

/// Thread-safe routing table for tracking peers in the mesh network
#[derive(Clone)]
pub struct RoutingTable {
    inner: Arc<RwLock<RoutingTableInner>>,
}

struct RoutingTableInner {
    /// Map of node_id -> PeerInfo (direct peers only)
    peers: HashMap<String, PeerInfo>,

    /// Link-state database: map of node_id -> LSA
    /// Contains topology information from all nodes in the network
    lsa_database: HashMap<String, LinkStateAdvertisement>,

    /// Sequence number for our own LSAs
    own_lsa_sequence: u64,
}

/// Node for Dijkstra's algorithm priority queue
#[derive(Eq, PartialEq)]
struct DijkstraNode {
    node_id: String,
    distance: usize,
}

impl Ord for DijkstraNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap
        other.distance.cmp(&self.distance)
            .then_with(|| self.node_id.cmp(&other.node_id))
    }
}

impl PartialOrd for DijkstraNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl RoutingTable {
    /// Create a new empty routing table
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RoutingTableInner {
                peers: HashMap::new(),
                lsa_database: HashMap::new(),
                own_lsa_sequence: 0,
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
            inner: Arc::new(RwLock::new(RoutingTableInner {
                peers,
                lsa_database: HashMap::new(),
                own_lsa_sequence: 0,
            })),
        }
    }

    /// Mark all peers as connected (temporary for testing until health checks are implemented)
    pub fn mark_all_connected(&self) {
        let mut inner = self.inner.write().unwrap();
        for peer in inner.peers.values_mut() {
            peer.status = PeerStatus::Connected;
            peer.last_seen = Some(SystemTime::now());
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

    /// Generate a new LSA for this node
    pub fn generate_lsa(&self, node_id: &str) -> LinkStateAdvertisement {
        let mut inner = self.inner.write().unwrap();
        inner.own_lsa_sequence += 1;

        // Get list of connected neighbors
        let neighbors: Vec<String> = inner
            .peers
            .values()
            .filter(|p| p.status == PeerStatus::Connected)
            .map(|p| p.node_id.clone())
            .collect();

        LinkStateAdvertisement {
            node_id: node_id.to_string(),
            neighbors,
            sequence: inner.own_lsa_sequence,
            timestamp: Some(SystemTime::now()),
        }
    }

    /// Process a received LSA
    /// Returns true if the LSA was new or newer than what we had
    pub fn process_lsa(&self, lsa: LinkStateAdvertisement) -> bool {
        let mut inner = self.inner.write().unwrap();

        // Check if we already have an LSA from this node
        if let Some(existing_lsa) = inner.lsa_database.get(&lsa.node_id) {
            // Only accept if sequence number is higher (newer)
            if lsa.sequence <= existing_lsa.sequence {
                return false;
            }
        }

        // Store or update the LSA
        inner.lsa_database.insert(lsa.node_id.clone(), lsa);
        true
    }

    /// Get all LSAs in the database (for forwarding)
    pub fn get_all_lsas(&self) -> Vec<LinkStateAdvertisement> {
        let inner = self.inner.read().unwrap();
        inner.lsa_database.values().cloned().collect()
    }

    /// Find a route to a destination node using Dijkstra's algorithm
    /// Returns a vector of node_ids representing the path (excluding source)
    pub fn find_route(&self, destination: &str) -> Option<Vec<String>> {
        let inner = self.inner.read().unwrap();

        // Check if destination is a direct connected peer (fast path)
        if let Some(peer) = inner.peers.get(destination) {
            if peer.status == PeerStatus::Connected {
                return Some(vec![destination.to_string()]);
            }
        }

        // Use link-state database to find multi-hop route
        // This will only work if we have received LSAs from other nodes
        if inner.lsa_database.is_empty() {
            return None;
        }

        // We need to know our own node_id to run Dijkstra
        // Extract it from one of our LSAs in the database
        // For now, return None if we don't have complete topology
        // This will be properly set when we integrate with the server

        None // Placeholder - will be implemented with proper node_id context
    }

    /// Find route using Dijkstra's algorithm given a source node
    pub fn find_route_from(&self, source: &str, destination: &str) -> Option<Vec<String>> {
        let inner = self.inner.read().unwrap();

        if source == destination {
            return Some(vec![]);
        }

        // Build adjacency list from LSA database
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        // Add direct peers to graph
        for (node_id, peer) in &inner.peers {
            if peer.status == PeerStatus::Connected {
                graph.entry(source.to_string())
                    .or_insert_with(Vec::new)
                    .push(node_id.clone());
            }
        }

        // Add LSA information to graph
        for lsa in inner.lsa_database.values() {
            graph.entry(lsa.node_id.clone())
                .or_insert_with(Vec::new)
                .extend(lsa.neighbors.clone());
        }

        // Run Dijkstra's algorithm
        let mut distances: HashMap<String, usize> = HashMap::new();
        let mut previous: HashMap<String, String> = HashMap::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut heap = BinaryHeap::new();

        distances.insert(source.to_string(), 0);
        heap.push(DijkstraNode {
            node_id: source.to_string(),
            distance: 0,
        });

        while let Some(DijkstraNode { node_id, distance }) = heap.pop() {
            if visited.contains(&node_id) {
                continue;
            }

            if node_id == destination {
                // Reconstruct path
                let mut path = vec![];
                let mut current = destination.to_string();

                while current != source {
                    path.push(current.clone());
                    match previous.get(&current) {
                        Some(prev) => current = prev.clone(),
                        None => return None, // Path broken
                    }
                }

                path.reverse();
                return Some(path);
            }

            visited.insert(node_id.clone());

            // Check neighbors
            if let Some(neighbors) = graph.get(&node_id) {
                for neighbor in neighbors {
                    if visited.contains(neighbor) {
                        continue;
                    }

                    let new_distance = distance + 1;
                    let is_shorter = distances
                        .get(neighbor)
                        .map_or(true, |&current| new_distance < current);

                    if is_shorter {
                        distances.insert(neighbor.clone(), new_distance);
                        previous.insert(neighbor.clone(), node_id.clone());
                        heap.push(DijkstraNode {
                            node_id: neighbor.clone(),
                            distance: new_distance,
                        });
                    }
                }
            }
        }

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
