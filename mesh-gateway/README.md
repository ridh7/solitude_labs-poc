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
cargo run --bin mesh_gateway -- --node-id gateway-a --port 8001
```

You should see output like:
```
🚀 Starting Mesh Gateway: gateway-a
📁 Certificate: certs/gateway-a.crt
🔐 Private Key: certs/gateway-a.key
🏛️  CA Certificate: certs/ca.crt
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

#### 4. Run Multiple Gateways (Optional)

**Terminal 1 - Gateway A:**
```bash
cargo run --bin mesh_gateway -- --node-id gateway-a --port 8001
```

**Terminal 2 - Gateway B:**
```bash
cargo run --bin mesh_gateway -- --node-id gateway-b --port 8002
```

**Terminal 3 - Gateway C:**
```bash
cargo run --bin mesh_gateway -- --node-id gateway-c --port 8003
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

### Phase 2+: Mesh Routing (Coming Soon)

Features in development:
- Peer-to-peer message routing
- Multi-hop mesh communication
- Self-healing network topology
- Dynamic peer discovery

## Project Structure

```
mesh-gateway/
├── src/
│   ├── main.rs              # Entry point & CLI
│   ├── server.rs            # HTTPS server (Axum)
│   ├── client.rs            # HTTPS client (Reqwest)
│   ├── router.rs            # Mesh routing logic
│   ├── certs.rs             # Certificate handling
│   ├── config.rs            # Config file parsing
│   ├── types.rs             # Shared types
│   └── bin/
│       └── gen_certs.rs     # Certificate generation
├── certs/                   # Generated certificates
├── configs/                 # Gateway configurations
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
- ✅ Peer-to-peer communication
- ✅ Multi-hop message routing
- ✅ Self-healing network topology
- ✅ No single point of failure

## API Endpoints

Each gateway exposes:

- `GET /health` - Health check
- `GET /peer/info` - Gateway information
- `GET /peers` - List connected peers
- `POST /message/send` - Send message to another gateway

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
