# Mesh Gateway Network - Design Document

## Project Overview

### Goal
Build a proof-of-concept mesh network of DER (Distributed Energy Resource) gateways demonstrating:
- Zero-trust security architecture
- Mutual TLS (mTLS) authentication
- Peer-to-peer mesh communication
- Self-healing network topology

### Why This Project?
This demonstrates core concepts used by Solitude Labs and the energy grid industry:
- IEEE 2030.5-style HTTPS communication
- NIST 800-207 zero-trust principles
- Distributed security without single points of failure

### Learning Objectives
- Implement HTTPS servers/clients in Rust
- Work with TLS certificates and PKI
- Build mesh network routing
- Understand zero-trust authentication flows

---

## Architecture

### High-Level Design

```
┌─────────────────────────────────────────────────────┐
│                  Mesh Network                        │
│                                                      │
│     ┌──────────┐         ┌──────────┐              │
│     │ Gateway  │ ←─────→ │ Gateway  │              │
│     │    A     │         │    B     │              │
│     │ :8001    │         │ :8002    │              │
│     └────┬─────┘         └────┬─────┘              │
│          │                    │                     │
│          │      ┌──────────┐  │                     │
│          └─────→│ Gateway  │←─┘                     │
│                 │    C     │                        │
│                 │ :8003    │                        │
│                 └──────────┘                        │
│                                                      │
│  All connections use HTTPS with mutual TLS          │
│  Each gateway can route through any neighbor        │
└─────────────────────────────────────────────────────┘
```

### Component Architecture

Each gateway is a **single Rust binary** that acts as:

1. **HTTPS Server** - Listens for incoming peer connections
2. **HTTPS Client** - Connects to peer gateways
3. **Mesh Router** - Routes messages through the network
4. **Certificate Authority** - Validates peer certificates

```
┌─────────────────────────────────────────────────────┐
│              Gateway Application                     │
├─────────────────────────────────────────────────────┤
│                                                      │
│  ┌─────────────────┐      ┌─────────────────┐      │
│  │  HTTPS Server   │      │  HTTPS Client   │      │
│  │  (Axum)         │      │  (Reqwest)      │      │
│  │                 │      │                 │      │
│  │ - Accept conns  │      │ - Connect peers │      │
│  │ - Verify certs  │      │ - Send requests │      │
│  │ - Handle APIs   │      │ - Verify certs  │      │
│  └────────┬────────┘      └────────┬────────┘      │
│           │                        │               │
│           └────────┬───────────────┘               │
│                    ▼                               │
│         ┌─────────────────────┐                    │
│         │   Mesh Router       │                    │
│         │                     │                    │
│         │ - Routing table     │                    │
│         │ - Peer discovery    │                    │
│         │ - Health checks     │                    │
│         └──────────┬──────────┘                    │
│                    ▼                               │
│         ┌─────────────────────┐                    │
│         │  Certificate Store  │                    │
│         │                     │                    │
│         │ - Own cert/key      │                    │
│         │ - Root CA cert      │                    │
│         │ - Trusted peers     │                    │
│         └─────────────────────┘                    │
└─────────────────────────────────────────────────────┘
```

---

## Technology Stack

### Core Technologies

```yaml
Language: Rust (2021 edition)

HTTP Framework:
  Server: axum (async web framework)
  Client: reqwest (HTTP client)
  Runtime: tokio (async runtime)

TLS/Security:
  rustls: Pure Rust TLS implementation
  rcgen: Certificate generation
  x509-parser: Certificate validation

Serialization:
  serde: Serialization framework
  serde_json: JSON format

CLI:
  clap: Command-line argument parsing

Logging:
  tracing: Structured logging
  tracing-subscriber: Log output
```

### Why These Choices?

- **Axum**: Modern, ergonomic, built on tokio, excellent for APIs
- **Reqwest**: Industry standard HTTP client with TLS support
- **Rustls**: Memory-safe TLS (vs OpenSSL which is C-based)
- **Tokio**: De-facto async runtime for Rust

### Real-World Alignment

This stack aligns with IEEE 2030.5 requirements:
- ✅ HTTPS/TLS 1.2+ (via rustls)
- ✅ Certificate-based authentication (PKI)
- ✅ Mutual TLS (mTLS)
- ✅ RESTful API design

**References:**
- [IEEE 2030.5 Security Requirements](https://archerint.com/ieee-2030-5-ibr-security/)
- [NIST SP 800-207 Zero Trust Architecture](https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-207.pdf)

---

## Security Model

### Zero-Trust Principles (NIST 800-207)

Following NIST zero-trust architecture:

1. **Never Trust, Always Verify**
   - Every connection requires certificate validation
   - No implicit trust based on network location

2. **Least Privilege**
   - Each gateway only exposes necessary APIs

3. **Assume Breach**
   - Each gateway validates independently
   - No single point of failure

### Mutual TLS (mTLS) Flow

```
Gateway A wants to connect to Gateway B:

┌──────────┐                           ┌──────────┐
│Gateway A │                           │Gateway B │
└────┬─────┘                           └────┬─────┘
     │                                      │
     │ 1. TLS ClientHello                   │
     ├─────────────────────────────────────>│
     │    (Supported ciphers)               │
     │                                      │
     │ 2. TLS ServerHello + Certificate     │
     │<─────────────────────────────────────┤
     │    (B's certificate)                 │
     │                                      │
     │ 3. Certificate Request               │
     │<─────────────────────────────────────┤
     │    (B requests A's certificate)      │
     │                                      │
     │ 4. Client Certificate + Finished     │
     ├─────────────────────────────────────>│
     │    (A's certificate)                 │
     │                                      │
     │    [A verifies B's cert]             │
     │    [B verifies A's cert]             │
     │                                      │
     │ 5. TLS Handshake Complete            │
     │<────────────────────────────────────>│
     │                                      │
     │ 6. Encrypted HTTPS requests          │
     │<────────────────────────────────────>│
     │                                      │
```

**Certificate Validation:**
- Verify signature using Root CA public key
- Check certificate not expired
- Verify common name matches expected peer ID

### Certificate Structure

**Root CA (Self-Signed):**
```
Subject: CN=MeshNet Root CA
Issuer: CN=MeshNet Root CA (self-signed)
Purpose: Sign gateway certificates
Private Key: Kept secure, used to sign gateway certs
```

**Gateway Certificate:**
```
Subject: CN=gateway-A
Issuer: CN=MeshNet Root CA
Purpose: Identify and authenticate gateway
Signed by: Root CA private key
Contains: Gateway's public key
```

**Trust Model:**
```
Root CA (trusted by all)
    │
    ├─ signs ──> Gateway A Certificate
    ├─ signs ──> Gateway B Certificate
    └─ signs ──> Gateway C Certificate

Each gateway:
  - Has Root CA certificate (public)
  - Has own certificate (signed by Root CA)
  - Has own private key
  - Trusts any cert signed by Root CA
```

---

## API Design

### RESTful Endpoints

Each gateway exposes these HTTPS endpoints:

#### 1. Health Check
```http
GET /health
Response: 200 OK
{
  "status": "healthy",
  "node_id": "gateway-a",
  "uptime_seconds": 123
}
```

#### 2. Peer Info
```http
GET /peer/info
Response: 200 OK
{
  "node_id": "gateway-a",
  "listen_addr": "127.0.0.1:8001",
  "peers": ["gateway-b", "gateway-c"],
  "version": "0.1.0"
}
```

#### 3. Send Message
```http
POST /message/send
Headers:
  Content-Type: application/json

Body:
{
  "to": "gateway-c",
  "content": "Hello from A"
}

Response: 200 OK
{
  "status": "delivered",
  "route": ["gateway-a", "gateway-b", "gateway-c"]
}
```

#### 4. List Peers
```http
GET /peers
Response: 200 OK
{
  "peers": [
    {
      "node_id": "gateway-b",
      "address": "127.0.0.1:8002",
      "status": "connected",
      "last_seen": "2025-12-14T18:30:00Z"
    },
    {
      "node_id": "gateway-c",
      "address": "127.0.0.1:8003",
      "status": "connected",
      "last_seen": "2025-12-14T18:30:01Z"
    }
  ]
}
```

### Message Routing

**Direct Connection:**
```
Gateway A → Gateway B
(A and B are neighbors)

1. A sends HTTPS POST to B
2. B receives and processes
```

**Mesh Routing:**
```
Gateway A → Gateway C (no direct connection)
(A connects through B)

1. A determines route: A → B → C
2. A sends HTTPS POST to B with final destination = C
3. B forwards HTTPS POST to C
4. C processes and sends response back through B
```

---

## Mesh Networking

### Peer Discovery

**Static Configuration:**
```toml
# gateway-a.toml
node_id = "gateway-a"
listen_port = 8001

[[peers]]
node_id = "gateway-b"
address = "127.0.0.1:8002"

[[peers]]
node_id = "gateway-c"
address = "127.0.0.1:8003"
```

### Routing Table

Each gateway maintains a routing table:

```rust
struct RoutingTable {
    // Direct neighbors
    neighbors: HashMap<NodeId, PeerInfo>,

    // Routes to non-neighbors
    routes: HashMap<NodeId, NodeId>, // destination -> next_hop
}
```

**Example:**
```
Gateway A's routing table:
┌─────────────┬────────────┬─────────────┐
│ Destination │ Next Hop   │ Hop Count   │
├─────────────┼────────────┼─────────────┤
│ gateway-b   │ gateway-b  │ 1 (direct)  │
│ gateway-c   │ gateway-c  │ 1 (direct)  │
└─────────────┴────────────┴─────────────┘

If Gateway B connects to Gateway D:
┌─────────────┬────────────┬─────────────┐
│ gateway-d   │ gateway-b  │ 2 (via B)   │
└─────────────┴────────────┴─────────────┘
```

### Self-Healing

**Scenario: Gateway B fails**

```
Before:
  A ↔ B ↔ C

After B fails:
  A   X   C

Self-healing:
1. A detects B is down (health check timeout)
2. A removes B from routing table
3. A tries alternative route to C (if exists)
4. Network reconfigures automatically
```

Implementation:
- Periodic health checks (every 10 seconds)
- Timeout after 3 failed checks
- Auto-remove failed peers
- Rebuild routing table

---

## Implementation Plan

### Phase 1: Basic HTTPS with mTLS

**Goals:**
- Generate certificates (Root CA + 3 gateway certs)
- Create basic HTTPS server (Axum)
- Create basic HTTPS client (Reqwest)
- Implement mTLS authentication
- Test peer-to-peer connection

**Deliverables:**
- Certificate generation script
- Basic gateway binary
- Can establish mTLS connection between 2 gateways

**Success Criteria:**
- Gateway A can connect to Gateway B with mTLS
- Certificate validation works
- Reject connections with invalid certs

### Phase 2: RESTful API

**Goals:**
- Implement `/health` endpoint
- Implement `/peer/info` endpoint
- Implement `/message/send` endpoint
- Add structured logging

**Deliverables:**
- Full REST API
- JSON serialization
- Request/response handling

**Success Criteria:**
- Can query peer info via API
- Can send messages between gateways
- Logs show authentication flow

### Phase 3: Mesh Routing

**Goals:**
- Build routing table
- Implement message forwarding
- Add peer discovery from config
- Multi-hop message routing

**Deliverables:**
- Routing algorithm
- Configuration file parsing
- 3-gateway mesh network

**Success Criteria:**
- Gateway A can send message to C via B
- Routing table updates correctly
- Messages delivered through mesh

### Phase 4: Self-Healing

**Goals:**
- Health check mechanism
- Automatic peer removal on failure
- Route recalculation
- Network resilience

**Deliverables:**
- Health check background task
- Failure detection
- Auto-recovery

**Success Criteria:**
- Detect when peer goes offline
- Remove failed peer from routing
- Network continues operating

---

## Project Structure

```
mesh-gateway/
├── Cargo.toml
├── README.md
├── DESIGN_DOC.md (this file)
│
├── certs/                  # Generated certificates
│   ├── ca.key              # Root CA private key
│   ├── ca.crt              # Root CA certificate
│   ├── gateway-a.key       # Gateway A private key
│   ├── gateway-a.crt       # Gateway A certificate
│   ├── gateway-b.key
│   ├── gateway-b.crt
│   ├── gateway-c.key
│   └── gateway-c.crt
│
├── configs/                # Gateway configurations
│   ├── gateway-a.toml
│   ├── gateway-b.toml
│   └── gateway-c.toml
│
├── src/
│   ├── main.rs            # Entry point + CLI
│   ├── server.rs          # HTTPS server (Axum)
│   ├── client.rs          # HTTPS client (Reqwest)
│   ├── router.rs          # Mesh routing logic
│   ├── certs.rs           # Certificate handling
│   ├── config.rs          # Config file parsing
│   └── types.rs           # Shared types
│
└── scripts/
    └── gen_certs.sh       # Certificate generation script
```

---

## Running the Demo

### Step 1: Generate Certificates
```bash
cd mesh-gateway
./scripts/gen_certs.sh
```

### Step 2: Start Gateways (3 terminals)

**Terminal 1:**
```bash
cargo run -- --config configs/gateway-a.toml
```

**Terminal 2:**
```bash
cargo run -- --config configs/gateway-b.toml
```

**Terminal 3:**
```bash
cargo run -- --config configs/gateway-c.toml
```

### Step 3: Test Communication

**Send message from A to B:**
```bash
curl --cacert certs/ca.crt \
     --cert certs/gateway-a.crt \
     --key certs/gateway-a.key \
     -X POST https://localhost:8002/message/send \
     -H "Content-Type: application/json" \
     -d '{"to": "gateway-b", "content": "Hello!"}'
```

**Send message from A to C via B (mesh routing):**
```bash
curl --cacert certs/ca.crt \
     --cert certs/gateway-a.crt \
     --key certs/gateway-a.key \
     -X POST https://localhost:8001/message/send \
     -H "Content-Type: application/json" \
     -d '{"to": "gateway-c", "content": "Hello via mesh!"}'
```

---

## Success Metrics

### Functional Requirements
- ✅ 3 gateways running simultaneously
- ✅ Mutual TLS authentication working
- ✅ Messages can be sent between any two gateways
- ✅ Mesh routing through intermediate gateways
- ✅ Self-healing when gateway fails

### Security Requirements
- ✅ All connections use TLS 1.2+
- ✅ Invalid certificates are rejected
- ✅ Each gateway validates peer certificates
- ✅ No plaintext communication

### Observable Behaviors
- ✅ Logs show TLS handshake details
- ✅ Logs show message routing path
- ✅ Logs show peer health checks
- ✅ Can query peer status via API

---

## References

### Industry Standards
1. [IEEE 2030.5 Security Requirements](https://archerint.com/ieee-2030-5-ibr-security/)
2. [NIST SP 800-207 - Zero Trust Architecture](https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-207.pdf)
3. [GridOS Zero Trust Security](https://www.gevernova.com/software/blog/how-gridos-software-has-zero-trust-security-built-in)
4. [NREL DER Cybersecurity Standards](https://docs.nrel.gov/docs/fy18osti/70454.pdf)

### Technical Resources
5. [Axum Web Framework](https://github.com/tokio-rs/axum)
6. [Reqwest HTTP Client](https://github.com/seanmonstar/reqwest)
7. [Rustls TLS Library](https://github.com/rustls/rustls)
8. [Tokio Async Runtime](https://tokio.rs/)

---

## Contributors

- Ridh ([@ridh7](https://github.com/ridh7)) - Primary developer
- Claude Code - Design & implementation assistance

---

**Last Updated:** 2025-12-14
**Version:** 0.1.0
**Status:** Design Phase
