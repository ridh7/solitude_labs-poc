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
Language: Rust (2024 edition)

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
      "status": "disconnected",
      "last_seen": "2025-12-14T18:28:10Z"
    }
  ]
}
```

#### 5. Receive Message (Internal)
```http
POST /message/receive
Headers:
  Content-Type: application/json

Body:
{
  "from": "gateway-a",
  "to": "gateway-c",
  "content": "Hello!",
  "route": ["gateway-a", "gateway-b"]
}

Response: 200 OK
{
  "status": "delivered",
  "route": ["gateway-a", "gateway-b", "gateway-c"]
}
```

#### 6. Link State Advertisement (Internal)
```http
POST /topology/lsa
Headers:
  Content-Type: application/json

Body:
{
  "node_id": "gateway-b",
  "neighbors": ["gateway-a", "gateway-c"],
  "sequence": 1,
  "timestamp": "2025-12-14T18:30:25Z"
}

Response: 200 OK
{
  "status": "accepted",
  "message": "LSA from gateway-b accepted and flooded"
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

### Routing Table & Link-State Protocol

Each gateway maintains:
1. **Routing table** - Direct peer information
2. **LSA database** - Complete network topology from Link State Advertisements
3. **Dijkstra's algorithm** - Computes shortest paths from topology

```rust
struct RoutingTable {
    // Direct neighbors from config
    peers: HashMap<NodeId, PeerInfo>,

    // Link-state database (OSPF-like)
    lsa_database: HashMap<NodeId, LinkStateAdvertisement>,

    // LSA sequence counter
    own_lsa_sequence: u64,
}
```

**Link-State Routing (OSPF-like):**
- Every 30s, each gateway broadcasts LSA containing its neighbors
- LSAs are flooded to all connected peers immediately
- Dijkstra's algorithm computes shortest paths from topology graph
- Automatic route discovery through unknown intermediate nodes

### Self-Healing

**Scenario: Gateway B fails**

```
Before:
  A ↔ B ↔ C

After B fails:
  A   X   C

Self-healing:
1. A detects B is down (health check timeout)
2. A marks B as disconnected in routing table
3. Dijkstra excludes disconnected peers from graph
4. Routes automatically recalculate using only healthy peers
```

**Implemented:**
- Periodic health checks (every 15 seconds)
- 5-second timeout per health check request
- Peers start as `unknown`, transition to `connected`/`disconnected` dynamically
- Only `connected` peers included in routing and LSA broadcasts
- Automatic peer recovery when failed nodes return

---

## Implementation Plan

### Phase 1: Basic HTTPS with mTLS ✅ Complete

**Goals:**
- Generate certificates (Root CA + 3 gateway certs)
- Create basic HTTPS server (Axum)
- Create basic HTTPS client (Reqwest)
- Implement mTLS authentication
- Test peer-to-peer connection

**Implemented:**
- Certificate generation utility (Rust binary)
- HTTPS server with mTLS validation
- HTTPS client with certificate-based auth
- Cross-gateway authentication working

### Phase 2: RESTful API ✅ Complete

**Implemented:**
- `/health`, `/peer/info`, `/peers`, `/message/send` endpoints
- `/message/receive` (internal) for multi-hop forwarding
- `/topology/lsa` (internal) for link-state routing
- JSON serialization with RFC3339 timestamps
- Structured logging with tracing

### Phase 3: Mesh Routing ✅ Complete

**Implemented:**
- Link-state routing protocol (OSPF-like)
- LSA database with sequence numbers
- Dijkstra's shortest path algorithm
- Automatic topology discovery via LSA flooding
- Multi-hop message forwarding with route tracking
- Loop prevention and error handling
- TOML configuration with validation

### Phase 4: Self-Healing ✅ Complete

**Implemented:**
- Periodic health checks (15s interval, 5s timeout)
- Automatic peer failure detection
- Dynamic status management (Unknown/Connected/Disconnected)
- Route recalculation using only healthy peers
- Automatic peer recovery when nodes return
- Network continues operating through failures

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
│   ├── server.rs          # HTTPS server (Axum) + LSA/health tasks
│   ├── client.rs          # HTTPS client (Reqwest)
│   ├── routing.rs         # Routing table, LSA database, Dijkstra
│   ├── certs.rs           # Certificate handling
│   ├── config.rs          # TOML config parsing & validation
│   ├── types.rs           # Shared types & serialization
│   └── bin/
│       └── gen_certs.rs   # Certificate generation utility
│
└── configs/
    ├── gateway-a.toml
    ├── gateway-b.toml
    ├── gateway-c.toml
    ├── gateway-a-linear.toml  # Linear topology testing
    ├── gateway-b-linear.toml
    └── gateway-c-linear.toml
```

---

## Running the Demo

### Step 1: Generate Certificates
```bash
cd mesh-gateway
cargo run --bin gen_certs
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
9. [OSPF - Open Shortest Path First](https://en.wikipedia.org/wiki/Open_Shortest_Path_First)

---

## Contributors

- Ridh ([@ridh7](https://github.com/ridh7)) - Primary developer
- Claude Code - Design & implementation assistance

---

**Last Updated:** 2024-12-21
**Version:** 0.1.0
**Status:** ✅ Fully Implemented - All Phases Complete
