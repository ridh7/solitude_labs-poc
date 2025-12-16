# Mesh Gateway Network

A proof-of-concept mesh network of DER (Distributed Energy Resource) gateways demonstrating zero-trust security with mutual TLS authentication.

## Quick Start

### Phase 1: Basic mTLS Communication (Current)

#### 1. Generate Certificates

```bash
cargo run --bin gen_certs
```

This generates:
- `certs/ca.crt` and `certs/ca.key` (Root CA)
- `certs/gateway-a.crt` and `certs/gateway-a.key`
- `certs/gateway-b.crt` and `certs/gateway-b.key`
- `certs/gateway-c.crt` and `certs/gateway-c.key`

**Note**: Private keys (`*.key`) are gitignored and not committed. You must regenerate them on each machine.

#### 2. Run a Gateway Server

**Terminal 1 - Start Gateway A:**
```bash
cargo run -- --config configs/gateway-a.toml
```

You should see output like:
```
📄 Loading configuration from: configs/gateway-a.toml
🚀 Starting Mesh Gateway: gateway-a
📁 Certificate: certs/gateway-a.crt
🔐 Private Key: certs/gateway-a.key
🏛️  CA Certificate: certs/ca.crt
👥 Configured peers: 2
🗺️  Routing table initialized with 2 peers
INFO mesh_gateway::server: Starting HTTPS server on 127.0.0.1:8001
INFO mesh_gateway::server: TLS configured for node: gateway-a
INFO mesh_gateway::server: Listening on https://127.0.0.1:8001
```

#### 3. Test mTLS Connection

**In another terminal, test the health endpoint:**

Using Gateway A's certificate:
```bash
curl --cacert certs/ca.crt \
     --cert certs/gateway-a.crt \
     --key certs/gateway-a.key \
     https://localhost:8001/health
```

Expected response:
```json
{"status":"healthy","node_id":"gateway-a","uptime_seconds":0}
```

**Test cross-gateway authentication** (Gateway B connecting to Gateway A):
```bash
curl --cacert certs/ca.crt \
     --cert certs/gateway-b.crt \
     --key certs/gateway-b.key \
     https://localhost:8001/health
```

This proves mutual TLS is working - Gateway B can authenticate to Gateway A using its own certificate!

**Test all API endpoints:**

Get gateway information:
```bash
curl --cacert certs/ca.crt \
     --cert certs/gateway-a.crt \
     --key certs/gateway-a.key \
     https://localhost:8001/peer/info
```

Expected response:
```json
{"node_id":"gateway-a","listen_addr":"127.0.0.1:8001","peers":["gateway-b","gateway-c"],"version":"0.1.0"}
```

List peers:
```bash
curl --cacert certs/ca.crt \
     --cert certs/gateway-a.crt \
     --key certs/gateway-a.key \
     https://localhost:8001/peers
```

Expected response:
```json
{
  "peers": [
    {
      "node_id": "gateway-b",
      "address": "127.0.0.1:8002",
      "status": "connected",
      "last_seen": "2024-12-14T18:30:25Z"
    },
    {
      "node_id": "gateway-c",
      "address": "127.0.0.1:8003",
      "status": "connected",
      "last_seen": "2024-12-14T18:30:25Z"
    }
  ]
}
```

**Note:** All peers are currently marked as `connected` on startup (even if they're not actually running) to enable message forwarding testing. The `last_seen` timestamp is set to the gateway's startup time. In Phase 4, health checks will properly verify peer availability and update status dynamically.

Send a message:
```bash
curl --cacert certs/ca.crt \
     --cert certs/gateway-a.crt \
     --key certs/gateway-a.key \
     -X POST https://localhost:8001/message/send \
     -H "Content-Type: application/json" \
     -d '{"to":"gateway-b","content":"Hello!"}'
```

Expected response (if gateway-b is not running or not reachable):
```json
{"status":"failed","route":["gateway-a"]}
```

Expected response (if gateway-b is running and reachable):
```json
{"status":"delivered","route":["gateway-a","gateway-b"]}
```

**Note:** For message forwarding to work, you need to have the destination gateway running. All peers are temporarily marked as `connected` on startup to enable testing. In Phase 4, health checks will properly manage peer status.

#### 4. Run Multiple Gateways (Optional)

**Terminal 1 - Gateway A:**
```bash
cargo run -- --config configs/gateway-a.toml
```

**Terminal 2 - Gateway B:**
```bash
cargo run -- --config configs/gateway-b.toml
```

**Terminal 3 - Gateway C:**
```bash
cargo run -- --config configs/gateway-c.toml
```

Test each gateway:
```bash
# Test Gateway A
curl --cacert certs/ca.crt --cert certs/gateway-a.crt --key certs/gateway-a.key https://localhost:8001/health

# Test Gateway B
curl --cacert certs/ca.crt --cert certs/gateway-b.crt --key certs/gateway-b.key https://localhost:8002/health

# Test Gateway C
curl --cacert certs/ca.crt --cert certs/gateway-c.crt --key certs/gateway-c.key https://localhost:8003/health
```

#### 5. Test Peer-to-Peer Message Forwarding

With multiple gateways running, you can now test actual message forwarding between them:

**Send a message from Gateway A to Gateway B:**
```bash
curl --cacert certs/ca.crt \
     --cert certs/gateway-a.crt \
     --key certs/gateway-a.key \
     -X POST https://localhost:8001/message/send \
     -H "Content-Type: application/json" \
     -d '{"to":"gateway-b","content":"Hello from A to B!"}'
```

Expected response:
```json
{"status":"delivered","route":["gateway-a","gateway-b"]}
```

Check the logs in Terminal 2 (gateway-b) - you should see:
```
INFO mesh_gateway::server: Received forwarded message from gateway-a to gateway-b: Hello from A to B!
INFO mesh_gateway::server: Message delivered to final destination: Hello from A to B!
```

**Send a message from Gateway B to Gateway C:**
```bash
curl --cacert certs/ca.crt \
     --cert certs/gateway-b.crt \
     --key certs/gateway-b.key \
     -X POST https://localhost:8002/message/send \
     -H "Content-Type: application/json" \
     -d '{"to":"gateway-c","content":"Hello from B to C!"}'
```

This demonstrates that the mesh network can route messages peer-to-peer using mTLS authentication!

#### 6. Test Multi-Hop Routing with Link-State Protocol

The mesh network now implements **link-state routing** (similar to OSPF) for automatic route discovery:

**How it works:**
1. Every 30 seconds, each gateway broadcasts a **Link State Advertisement (LSA)** to all its peers
2. LSAs contain: node_id, list of neighbors, sequence number
3. Gateways forward LSAs they receive to their other peers
4. Each gateway builds a complete network topology from all LSAs
5. **Dijkstra's algorithm** finds shortest paths through the network

**Network Topology (Linear):**
```
gateway-a ←→ gateway-b ←→ gateway-c
```

**Start gateways with linear topology:**

**Terminal 1 - Gateway A:**
```bash
cargo run -- --config configs/gateway-a-linear.toml
```

**Terminal 2 - Gateway B (intermediary):**
```bash
cargo run -- --config configs/gateway-b-linear.toml
```

**Terminal 3 - Gateway C:**
```bash
cargo run -- --config configs/gateway-c-linear.toml
```

**Wait 35 seconds** for LSAs to propagate (5s initial delay + 30s for first broadcast).

Check the logs - you should see:
```
INFO mesh_gateway::server: Received LSA from gateway-b (seq: 1, neighbors: ["gateway-a", "gateway-c"])
```

**Now send a multi-hop message from A to C:**

```bash
curl --cacert certs/ca.crt \
     --cert certs/gateway-a.crt \
     --key certs/gateway-a.key \
     -X POST https://localhost:8001/message/send \
     -H "Content-Type: application/json" \
     -d '{"to":"gateway-c","content":"Multi-hop from A to C!"}'
```

**Expected response:**
```json
{"status":"delivered","route":["gateway-a","gateway-b","gateway-c"]}
```

**What happened:**
1. Gateway A doesn't know Gateway C directly
2. Gateway A received LSA from Gateway B saying it knows Gateway C
3. Gateway A built a topology graph and ran Dijkstra's algorithm
4. Route found: A → B → C
5. Message sent to B, B forwarded to C, full route tracked!

This demonstrates true mesh routing with **automatic topology discovery** and **shortest path routing**!

### Phase 3: Mesh Routing (Complete!)

**Completed:**
- ✅ Configuration file parser (load peers from TOML)
- ✅ Routing table structure (thread-safe, with peer tracking)
- ✅ Peer discovery from config on startup
- ✅ Updated `/peers` endpoint with real routing table data
- ✅ Route finding for direct peers
- ✅ mTLS HTTP client for peer communication
- ✅ `/message/receive` endpoint for accepting forwarded messages
- ✅ Peer-to-peer message forwarding (direct connections)
- ✅ Multi-hop message forwarding infrastructure (intermediate nodes can relay)
- ✅ Loop prevention in multi-hop forwarding
- ✅ Route tracking across multiple hops
- ✅ Config validation (peer address format)
- ✅ Timestamp serialization (RFC3339 format)
- ✅ **Link-State Routing Protocol (OSPF-like)**
  - Link State Advertisements (LSAs) with sequence numbers
  - Periodic LSA broadcasting (30s interval)
  - LSA database for topology storage
  - **Dijkstra's shortest path algorithm**
  - Automatic route discovery through unknown nodes
  - Complete network topology awareness

**Phase 4: Self-Healing (Coming Soon)**
- Periodic health checks to update peer status
- Automatic peer failure detection
- Route recalculation when topology changes
- Network resilience testing

## Project Structure

```
mesh-gateway/
├── src/
│   ├── main.rs              # Entry point & CLI
│   ├── server.rs            # HTTPS server (Axum)
│   ├── client.rs            # HTTPS client (Reqwest)
│   ├── routing.rs           # Routing table & mesh logic
│   ├── certs.rs             # Certificate handling
│   ├── config.rs            # Config file parsing (TOML)
│   ├── types.rs             # Shared types
│   └── bin/
│       └── gen_certs.rs     # Certificate generation
├── certs/                   # Generated certificates
├── configs/                 # Gateway configurations (TOML)
└── scripts/                 # Helper scripts
```

## What This Demonstrates

### Zero-Trust Security (NIST SP 800-207)
- ✅ Mutual TLS authentication
- ✅ Certificate-based identity verification
- ✅ No implicit trust based on network location
- ✅ Each connection verified independently

### IEEE 2030.5 Alignment
- ✅ HTTPS communication
- ✅ Certificate-based authentication (PKI)
- ✅ RESTful API design
- ✅ TLS 1.2+ encryption

### Mesh Networking
- ✅ Peer-to-peer communication (mTLS)
- ✅ Configuration-driven peer discovery
- ✅ Thread-safe routing table with LSA database
- ✅ Link-state routing protocol (OSPF-like)
- ✅ Dijkstra's shortest path algorithm
- ✅ Automatic topology discovery via LSA exchange
- ✅ Multi-hop message forwarding (relay capability)
- ✅ Multi-hop route discovery (finding paths through unknown nodes)
- ✅ Loop prevention in message routing
- ✅ Route tracking across multiple hops
- 🚧 Self-healing network topology (Phase 4)
- ✅ No single point of failure (decentralized architecture)

## API Endpoints

Each gateway exposes these HTTPS endpoints (require mTLS authentication):

### GET /health
Returns health status and uptime.

**Response:**
```json
{
  "status": "healthy",
  "node_id": "gateway-a",
  "uptime_seconds": 123
}
```

### GET /peer/info
Returns information about this gateway, including the list of configured peer node IDs.

**Response:**
```json
{
  "node_id": "gateway-a",
  "listen_addr": "127.0.0.1:8001",
  "peers": ["gateway-b", "gateway-c"],
  "version": "0.1.0"
}
```

### GET /peers
Lists all peer gateways from the routing table.

**Response:**
```json
{
  "peers": [
    {
      "node_id": "gateway-b",
      "address": "127.0.0.1:8002",
      "status": "connected",
      "last_seen": "2024-12-14T18:30:25Z"
    },
    {
      "node_id": "gateway-c",
      "address": "127.0.0.1:8003",
      "status": "connected",
      "last_seen": "2024-12-14T18:30:25Z"
    }
  ]
}
```
**Note:** Peers are populated from the config file. All peers are marked as `connected` on startup (regardless of actual availability) to enable message forwarding testing. The `last_seen` timestamp shows when the gateway started. In Phase 4, health checks will properly verify peer availability and update status/timestamps dynamically.

### POST /message/send
Send a message to another gateway. Uses the routing table to find a path and forwards the message via mTLS.

**Request:**
```json
{
  "to": "gateway-b",
  "content": "Hello from gateway-a!"
}
```

**Response when no route is available:**
```json
{
  "status": "no_route",
  "route": ["gateway-a"]
}
```

**Response when message delivery fails:**
```json
{
  "status": "failed",
  "route": ["gateway-a"]
}
```

**Response when message is successfully delivered:**
```json
{
  "status": "delivered",
  "route": ["gateway-a", "gateway-b"]
}
```

**Note:** Messages are actually forwarded to the destination gateway using mTLS. The destination must be running and reachable for delivery to succeed.

### POST /message/receive
Internal endpoint used by gateways to receive forwarded messages from peers. Handles both final delivery and multi-hop forwarding.

**Request:**
```json
{
  "from": "gateway-a",
  "to": "gateway-c",
  "content": "Hello!",
  "route": ["gateway-a", "gateway-b"]
}
```

**Response (delivered to final destination):**
```json
{
  "status": "delivered",
  "route": ["gateway-a", "gateway-b", "gateway-c"]
}
```

**Response (loop detected):**
```json
{
  "status": "loop_detected",
  "route": ["gateway-a", "gateway-b", "gateway-a"]
}
```

**Response (no route for multi-hop):**
```json
{
  "status": "no_route",
  "route": ["gateway-a", "gateway-b"]
}
```

**Behavior:**
- If message is for this gateway: delivers and responds with "delivered"
- If message is for another gateway: forwards to next hop (multi-hop routing)
- If loop detected: drops message and responds with "loop_detected"
- If no route available: responds with "no_route"

**Note:** This endpoint is for internal use between gateways. Direct user access is not typically needed.

### POST /topology/lsa
Receives Link State Advertisements (LSAs) from peer gateways. Part of the link-state routing protocol.

**Request:**
```json
{
  "node_id": "gateway-b",
  "neighbors": ["gateway-a", "gateway-c"],
  "sequence": 1,
  "timestamp": "2024-12-14T18:30:25Z"
}
```

**Response (accepted):**
```json
{
  "status": "accepted",
  "message": "LSA from gateway-b accepted and processed"
}
```

**Response (ignored - duplicate or old):**
```json
{
  "status": "ignored",
  "message": "LSA from gateway-b already known or outdated"
}
```

**Behavior:**
- LSAs are automatically broadcasted every 30 seconds
- Sequence numbers prevent processing old LSAs
- LSA database is used to build complete network topology
- Dijkstra's algorithm uses topology for route finding

**Note:** This endpoint is called automatically by the link-state routing protocol. Manual testing not typically needed.

## Certificate Trust Chain

```
Root CA (self-signed)
    │
    ├─ signs → Gateway A Certificate
    ├─ signs → Gateway B Certificate
    └─ signs → Gateway C Certificate
```

Each gateway:
- Trusts the Root CA
- Has its own certificate signed by the CA
- Validates peer certificates against the CA
- Rejects connections with invalid certificates

## Development

### Build
```bash
cargo build
```

### Run tests
```bash
cargo test
```

### Run with logging
```bash
RUST_LOG=debug cargo run -- --config configs/gateway-a.toml
```

### Check code with clippy
```bash
cargo clippy
```

### Format code
```bash
cargo fmt
```

## Technologies Used

- **Rust 2024 edition**
- **Axum** - HTTP server framework
- **Reqwest** - HTTP client
- **Tokio** - Async runtime
- **Rustls** - Pure Rust TLS implementation
- **rcgen** - Certificate generation

## References

- [IEEE 2030.5 Security Requirements](https://archerint.com/ieee-2030-5-ibr-security/)
- [NIST SP 800-207 - Zero Trust Architecture](https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-207.pdf)
- [NREL DER Cybersecurity Standards](https://docs.nrel.gov/docs/fy18osti/70454.pdf)

## License

This is a proof-of-concept for learning purposes.
