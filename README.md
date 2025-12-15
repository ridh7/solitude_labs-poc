# Solitude Labs - Rust Learning & POCs

This repository documents my journey learning Rust, ranging from "Hello, World!" basics to building a secure, zero-trust mesh network gateway.

## Project Index

### 🟢 Beginner (The Book)
Foundational projects following [The Rust Programming Language](https://doc.rust-lang.org/book/) book.

- **`hello_world/`**: The minimal entry point. Pure `rustc` usage without Cargo.
- **`hello_cargo/`**: Introduction to Cargo, Rust's build system and package manager.
- **`guessing_game/`**: A classic interactive CLI game. Covers:
  - Standard input/output (`std::io`)
  - Random number generation (`rand` crate)
  - `match` statements and comparisons (`std::cmp::Ordering`)
  - Error handling with `Result`

### 🔵 Advanced (Capstone POC)
Real-world application concepts.

- **`mesh-gateway/`**: A Distributed Energy Resource (DER) mesh gateway network.
  - **Key Concepts**: Async Rust (`tokio`), Network Programming (`axum`, `reqwest`), Security (mTLS with `rustls`), and Configuration (`toml`).
  - **Goal**: Demonstrate a zero-trust architecture where gateways authenticate each other using mutual TLS certificates.

## Quick Reference

### Running Projects

**Run a cargo-based project:**
```bash
cd guessing_game
cargo run
```

**Run the Mesh Gateway (Advanced):**
```bash
cd mesh-gateway
# 1. Generate certs first
cargo run --bin gen_certs
# 2. Run the server
cargo run --bin mesh_gateway
```

### Common Commands

- `cargo check`: Quickly check code for compilation errors without building.
- `cargo build`: Build the project.
- `cargo build --release`: Build optimized binary for production.
- `cargo clippy`: Run the linter to catch common mistakes and improve idiomatic Rust.
- `cargo fmt`: Automatically format code.

## Resources

- [The Rust Book](https://doc.rust-lang.org/book/) - Primary learning resource.
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/) - Learn by reading code.
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial) - For async Rust (used in `mesh-gateway`).
