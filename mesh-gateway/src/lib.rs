// This file creates a library crate (in addition to the binary crate from main.rs)
// This allows us to:
// 1. Run unit tests in modules (cargo test)
// 2. Organize code separately from the CLI entry point
// 3. Reuse modules across multiple binaries (e.g., main.rs and gen_certs.rs)

pub mod certs;
pub mod client;
pub mod config;
pub mod routing;
pub mod server;
pub mod types;
