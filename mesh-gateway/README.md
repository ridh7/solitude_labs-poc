# Mesh Gateway Network

A proof-of-concept mesh network of DER (Distributed Energy Resource) gateways demonstrating zero-trust security with mutual TLS authentication, link-state routing, and self-healing capabilities.

## Demo

https://github.com/user-attachments/assets/e7a987d9-7155-43b2-a222-58e3e37e0bbd

## Features

- ✅ **Zero-Trust Security**: Mutual TLS (mTLS) authentication with certificate-based identity
- ✅ **Link-State Routing**: OSPF-like protocol with Dijkstra's shortest path algorithm
- ✅ **Multi-Hop Routing**: Automatic route discovery through intermediate nodes
- ✅ **Self-Healing Network**: Automatic peer failure detection and recovery
- ✅ **Decentralized Architecture**: No single point of failure
- ✅ **Dynamic Topology**: Real-time route recalculation on network changes

## Quick Start

### 1. Generate Certificates

```bash
cargo run --bin gen_certs
```

This generates:
- `certs/ca.crt` and `certs/ca.key` (Root CA)
- `certs/gateway-a.crt` and `certs/gateway-a.key`
- `certs/gateway-b.crt` and `certs/gateway-b.key`
- `certs/gateway-c.crt` and `certs/gateway-c.key`

**Note**: Private keys (`*.key`) are gitignored and must be regenerated on each machine.

### 2. Start a Gateway

```bash
cargo run -- --config configs/gateway-a.toml
```

Expected output:
```
📄 Loading configuration from: configs/gateway-a.toml
🚀 Starting Mesh Gateway: gateway-a
📁 Certificate: certs/gateway-a.crt
🔐 Private Key: certs/gateway-a.key
🏛️  CA Certificate: certs/ca.crt
👥 Configured peers: 2
🗺️  Routing table initialized with 2 peers
🏥 Starting peer health monitoring...
✓ Health check task started (15s interval)
🔄 Starting link-state routing protocol...
✓ LSA broadcast task started (30s interval)
INFO mesh_gateway::server: Starting HTTPS server on 127.0.0.1:8001
INFO mesh_gateway::server: TLS configured for node: gateway-a
INFO mesh_gateway::server: Listening on https://127.0.0.1:8001
```

### 3. Test mTLS Authentication

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

**Test cross-gateway authentication** (Gateway B authenticating to Gateway A):
```bash
curl --cacert certs/ca.crt \
     --cert certs/gateway-b.crt \
     --key certs/gateway-b.key \
     https://localhost:8001/health
```

This proves mutual TLS is working - any gateway can authenticate to any other using its own certificate!

## Testing Features

### Basic API Endpoints

**Get gateway information:**
```bash
curl --cacert certs/ca.crt \
     --cert certs/gateway-a.crt \
     --key certs/gateway-a.key \
     https://localhost:8001/peer/info
```

**List peers:**
```bash
curl --cacert certs/ca.crt \
     --cert certs/gateway-a.crt \
     --key certs/gateway-a.key \
     https://localhost:8001/peers
```

Peer status is dynamically updated via health checks every 15 seconds. Peers start as `unknown` and transition to `connected` once healthy.

### Peer-to-Peer Messaging

**Start multiple gateways:**

```bash
# Terminal 1
cargo run -- --config configs/gateway-a.toml

# Terminal 2
cargo run -- --config configs/gateway-b.toml

# Terminal 3
cargo run -- --config configs/gateway-c.toml
```

Wait 10-15 seconds for health checks, then send a message:

```bash
curl --cacert certs/ca.crt \
     --cert certs/gateway-a.crt \
     --key certs/gateway-a.key \
     -X POST https://localhost:8001/message/send \
     -H "Content-Type: application/json" \
     -d '{"to":"gateway-b","content":"Hello from A!"}'
```

Expected response:
```json
{"status":"delivered","route":["gateway-a","gateway-b"]}
```

Check Gateway B's logs:
```
INFO mesh_gateway::server: Received forwarded message from gateway-a to gateway-b: Hello from A!
INFO mesh_gateway::server: Message delivered to final destination: Hello from A!
```

### Multi-Hop Routing with Link-State Protocol

The network implements **OSPF-like link-state routing** with automatic topology discovery:

**How it works:**
1. Every 30 seconds, each gateway broadcasts a **Link State Advertisement (LSA)** containing its neighbors
2. Gateways flood LSAs to all peers (not just neighbors of the originator)
3. Each gateway builds a complete network topology graph from all LSAs
4. **Dijkstra's algorithm** computes shortest paths through the network
5. Routes automatically include intermediate hops that aren't direct peers

**Test with linear topology:**

Network topology: `gateway-a ←→ gateway-b ←→ gateway-c`

```bash
# Terminal 1 - Gateway A (only knows B)
cargo run -- --config configs/gateway-a-linear.toml

# Terminal 2 - Gateway B (knows both A and C)
cargo run -- --config configs/gateway-b-linear.toml

# Terminal 3 - Gateway C (only knows B)
cargo run -- --config configs/gateway-c-linear.toml
```

**Wait 35 seconds** for LSAs to propagate (5s initial delay + 30s for first broadcast).

Check logs for LSA exchange:
```
INFO mesh_gateway::server: Received LSA from gateway-b (seq: 1, neighbors: ["gateway-a", "gateway-c"])
```

**Send multi-hop message from A to C:**

```bash
curl --cacert certs/ca.crt \
     --cert certs/gateway-a.crt \
     --key certs/gateway-a.key \
     -X POST https://localhost:8001/message/send \
     -H "Content-Type: application/json" \
     -d '{"to":"gateway-c","content":"Multi-hop from A to C!"}'
```

Expected response:
```json
{"status":"delivered","route":["gateway-a","gateway-b","gateway-c"]}
```

**What happened:**
1. Gateway A doesn't have Gateway C configured as a direct peer
2. Gateway A received LSA from Gateway B advertising that B can reach C
3. Gateway A built a topology graph and ran Dijkstra's algorithm
4. Route computed: A → B → C
5. Message forwarded through B to reach C, with full route tracking!

### Self-Healing Network

The network automatically detects peer failures and recovers when they return.

**Start all three gateways:**

```bash
# Terminal 1
cargo run -- --config configs/gateway-a.toml

# Terminal 2
cargo run -- --config configs/gateway-b.toml

# Terminal 3
cargo run -- --config configs/gateway-c.toml
```

**Observe health check process** (wait ~10-15 seconds after startup):

Logs show peers becoming reachable:
```
INFO mesh_gateway::server: Peer gateway-b is now reachable
INFO mesh_gateway::server: Peer gateway-c is now reachable
```

**Check peer status:**
```bash
curl --cacert certs/ca.crt \
     --cert certs/gateway-a.crt \
     --key certs/gateway-a.key \
     https://localhost:8001/peers | jq
```

Peers show `"status": "connected"` with recent `last_seen` timestamps.

**Test failure detection - Stop Gateway B (Ctrl+C in Terminal 2)**

Wait 15-20 seconds, then check peers again:
```bash
curl --cacert certs/ca.crt \
     --cert certs/gateway-a.crt \
     --key certs/gateway-a.key \
     https://localhost:8001/peers | jq
```

Gateway B now shows `"status": "disconnected"`.

**Verify routing adapts to failure:**
```bash
curl --cacert certs/ca.crt \
     --cert certs/gateway-a.crt \
     --key certs/gateway-a.key \
     -X POST https://localhost:8001/message/send \
     -H "Content-Type: application/json" \
     -d '{"to":"gateway-b","content":"Are you there?"}'
```

Expected response:
```json
{"status":"no_route","route":["gateway-a"]}
```

The message fails because Gateway B is detected as disconnected and excluded from routing.

**Test self-healing - Restart Gateway B:**
```bash
# Terminal 2
cargo run -- --config configs/gateway-b.toml
```

Wait 15-20 seconds for health checks to detect recovery. Logs show:
```
INFO mesh_gateway::server: Peer gateway-b is now reachable
```

**Verify automatic recovery:**
```bash
curl --cacert certs/ca.crt \
     --cert certs/gateway-a.crt \
     --key certs/gateway-a.key \
     -X POST https://localhost:8001/message/send \
     -H "Content-Type: application/json" \
     -d '{"to":"gateway-b","content":"Welcome back!"}'
```

Expected response:
```json
{"status":"delivered","route":["gateway-a","gateway-b"]}
```

The network has **automatically healed** - Gateway B is back in the routing table and messages flow again!

**What you demonstrated:**
- ✅ Automatic peer failure detection (15s health check interval, 5s timeout)
- ✅ Dynamic status updates (Unknown → Connected → Disconnected → Connected)
- ✅ Route recalculation on topology changes
- ✅ Network resilience and self-healing without manual intervention

## Project Structure

```
mesh-gateway/
├── src/
│   ├── main.rs              # Entry point & CLI
│   ├── server.rs            # HTTPS server with Axum
│   ├── client.rs            # mTLS HTTP client
│   ├── routing.rs           # Routing table, LSA database, Dijkstra
│   ├── certs.rs             # Certificate loading
│   ├── config.rs            # TOML config parsing & validation
│   ├── types.rs             # Shared types & serialization
│   └── bin/
│       └── gen_certs.rs     # Certificate generation utility
├── certs/                   # Generated certificates (gitignored .key files)
├── configs/                 # Gateway TOML configurations
│   ├── gateway-a.toml       # Full mesh topology
│   ├── gateway-b.toml
│   ├── gateway-c.toml
│   ├── gateway-a-linear.toml  # Linear topology for multi-hop testing
│   ├── gateway-b-linear.toml
│   └── gateway-c-linear.toml
└── scripts/                 # Helper scripts
```

## Architecture & Capabilities

### Zero-Trust Security (NIST SP 800-207)
- ✅ Mutual TLS (mTLS) authentication for all connections
- ✅ Certificate-based identity verification (PKI)
- ✅ No implicit trust based on network location
- ✅ Each connection independently verified
- ✅ Root CA signs all gateway certificates

### IEEE 2030.5 Alignment
- ✅ HTTPS communication (TLS 1.2+)
- ✅ Certificate-based authentication
- ✅ RESTful API design
- ✅ Encrypted peer-to-peer communication

### Mesh Networking Features

**Link-State Routing Protocol (OSPF-like):**
- ✅ Link State Advertisements (LSAs) with sequence numbers
- ✅ Periodic LSA broadcasting (30s interval)
- ✅ LSA flooding to all neighbors for rapid propagation
- ✅ LSA database for complete topology storage
- ✅ Dijkstra's shortest path algorithm
- ✅ Automatic route discovery through unknown intermediate nodes

**Multi-Hop Routing:**
- ✅ Messages forwarded through multiple intermediary gateways
- ✅ Loop prevention via route tracking
- ✅ Complete route visibility from source to destination
- ✅ Automatic path computation from topology graph

**Self-Healing Network:**
- ✅ Periodic health checks (15s interval, 5s timeout per peer)
- ✅ Automatic peer failure detection
- ✅ Dynamic peer status management (Unknown/Connected/Disconnected)
- ✅ Automatic route recalculation when topology changes
- ✅ Peer recovery detection when failed nodes return

**Resilience:**
- ✅ Thread-safe routing table with Arc<RwLock<>>
- ✅ Decentralized architecture (no single point of failure)
- ✅ Concurrent LSA processing and health checks
- ✅ Configuration-driven peer discovery

## API Reference

All endpoints require mTLS authentication with valid gateway certificates.

### GET /health

Returns gateway health status and uptime.

**Response:**
```json
{
  "status": "healthy",
  "node_id": "gateway-a",
  "uptime_seconds": 123
}
```

### GET /peer/info

Returns this gateway's information and configured peer list.

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

Lists all peers with current status and last-seen timestamps.

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
      "status": "disconnected",
      "last_seen": "2024-12-14T18:28:10Z"
    }
  ]
}
```

Status values: `unknown` (no health check yet), `connected` (healthy), `disconnected` (failed/timeout).

### POST /message/send

Send a message to another gateway. Automatically routes via shortest path.

**Request:**
```json
{
  "to": "gateway-c",
  "content": "Hello!"
}
```

**Response - Success:**
```json
{
  "status": "delivered",
  "route": ["gateway-a", "gateway-b", "gateway-c"]
}
```

**Response - No Route:**
```json
{
  "status": "no_route",
  "route": ["gateway-a"]
}
```

**Response - Delivery Failed:**
```json
{
  "status": "failed",
  "route": ["gateway-a"]
}
```

### POST /message/receive

Internal endpoint for receiving forwarded messages. Handles both final delivery and multi-hop relay.

**Request:**
```json
{
  "from": "gateway-a",
  "to": "gateway-c",
  "content": "Hello!",
  "route": ["gateway-a", "gateway-b"]
}
```

**Response - Delivered:**
```json
{
  "status": "delivered",
  "route": ["gateway-a", "gateway-b", "gateway-c"]
}
```

**Response - Loop Detected:**
```json
{
  "status": "loop_detected",
  "route": ["gateway-a", "gateway-b", "gateway-a"]
}
```

**Behavior:**
- If `to` matches this gateway: delivers and responds with "delivered"
- If `to` is another gateway: computes next hop and forwards (multi-hop relay)
- If this gateway already in route: drops message with "loop_detected"
- If no route to destination: responds with "no_route"

### POST /topology/lsa

Receives Link State Advertisements from peers. Part of the link-state routing protocol.

**Request:**
```json
{
  "node_id": "gateway-b",
  "neighbors": ["gateway-a", "gateway-c"],
  "sequence": 1,
  "timestamp": "2024-12-14T18:30:25Z"
}
```

**Response - Accepted (new/newer LSA):**
```json
{
  "status": "accepted",
  "message": "LSA from gateway-b accepted and flooded"
}
```

**Response - Ignored (old/duplicate):**
```json
{
  "status": "ignored",
  "message": "LSA from gateway-b already known or outdated"
}
```

**Behavior:**
- New LSAs are stored in the LSA database and immediately flooded to all connected peers
- Sequence numbers prevent processing old/duplicate LSAs
- LSA database builds complete network topology
- Dijkstra's algorithm uses topology for route computation

**Note:** This endpoint is called automatically by the protocol. Manual testing not typically needed.

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

### Run with debug logging
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
- **Reqwest** - HTTP client with mTLS support
- **Tokio** - Async runtime for concurrent operations
- **Rustls** - Pure Rust TLS implementation
- **rcgen** - Certificate generation utility
- **serde** - Serialization/deserialization
- **toml** - Configuration file parsing

## References

- [IEEE 2030.5 Security Requirements](https://archerint.com/ieee-2030-5-ibr-security/)
- [NIST SP 800-207 - Zero Trust Architecture](https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-207.pdf)
- [NREL DER Cybersecurity Standards](https://docs.nrel.gov/docs/fy18osti/70454.pdf)
- [OSPF - Open Shortest Path First](https://en.wikipedia.org/wiki/Open_Shortest_Path_First)

## License

This is a proof-of-concept for learning purposes.
