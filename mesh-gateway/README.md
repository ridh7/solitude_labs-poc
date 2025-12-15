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
      "status": "unknown",
      "last_seen": null
    },
    {
      "node_id": "gateway-c",
      "address": "127.0.0.1:8003",
      "status": "unknown",
      "last_seen": null
    }
  ]
}
```

**Note:** Peer status is `unknown` until health checks are implemented in Phase 4.

Send a message:
```bash
curl --cacert certs/ca.crt \
     --cert certs/gateway-a.crt \
     --key certs/gateway-a.key \
     -X POST https://localhost:8001/message/send \
     -H "Content-Type: application/json" \
     -d '{"to":"gateway-b","content":"Hello!"}'
```

Expected response:
```json
{"status":"no_route","route":["gateway-a"]}
```

**Note:** Messages return `no_route` because peers are in `unknown` status. Peer-to-peer message forwarding will be implemented in Phase 3 continuation, and peer status will be set to `connected` via health checks in Phase 4.

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

### Phase 3: Mesh Routing (Partially Complete)

**Completed:**
- ✅ Configuration file parser (load peers from TOML)
- ✅ Routing table structure (thread-safe, with peer tracking)
- ✅ Peer discovery from config on startup
- ✅ Updated `/peers` endpoint with real routing table data
- ✅ Route finding for direct peers

**In Progress:**
- 🚧 Peer-to-peer message forwarding (direct connections)
- 🚧 Multi-hop mesh communication (routing through intermediary nodes)

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
- ✅ Thread-safe routing table
- ✅ Route finding for direct connections
- 🚧 Multi-hop message routing (in progress)
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
      "status": "unknown",
      "last_seen": null
    },
    {
      "node_id": "gateway-c",
      "address": "127.0.0.1:8003",
      "status": "unknown",
      "last_seen": null
    }
  ]
}
```
**Note:** Peers are populated from the config file. Status is `unknown` until health checks are implemented in Phase 4. Once connected, status will change to `connected` and `last_seen` will be populated.

### POST /message/send
Send a message to another gateway. Uses the routing table to find a path to the destination.

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

**Response when route is found (after Phase 4 health checks):**
```json
{
  "status": "queued",
  "route": ["gateway-a", "gateway-b"]
}
```

**Note:** Currently returns `no_route` because peers are in `unknown` status. Once Phase 4 health checks mark peers as `connected`, the routing will work and messages will be forwarded.

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
