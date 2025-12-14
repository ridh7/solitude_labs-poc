use anyhow::{Context, Result};
use rustls::{Certificate, PrivateKey, RootCertStore};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Loads a certificate from a PEM file
pub fn load_cert(path: impl AsRef<Path>) -> Result<Vec<Certificate>> {
    let file = File::open(path.as_ref())
        .context(format!("Failed to open certificate file: {:?}", path.as_ref()))?;

    let mut reader = BufReader::new(file);

    let certs: Vec<Certificate> = certs(&mut reader)?
        .into_iter()
        .map(Certificate)
        .collect();

    if certs.is_empty() {
        anyhow::bail!("No certificates found in file: {:?}", path.as_ref());
    }

    Ok(certs)
}

/// Loads a private key from a PEM file
pub fn load_private_key(path: impl AsRef<Path>) -> Result<PrivateKey> {
    let file = File::open(path.as_ref())
        .context(format!("Failed to open private key file: {:?}", path.as_ref()))?;

    let mut reader = BufReader::new(file);

    let mut keys = pkcs8_private_keys(&mut reader)?;

    if keys.is_empty() {
        anyhow::bail!("No private keys found in file: {:?}", path.as_ref());
    }

    if keys.len() > 1 {
        tracing::warn!("Multiple keys found in file, using first one: {:?}", path.as_ref());
    }

    Ok(PrivateKey(keys.remove(0)))
}

/// Loads the Root CA certificate into a RootCertStore
pub fn load_ca_cert(path: impl AsRef<Path>) -> Result<RootCertStore> {
    let file = File::open(path.as_ref())
        .context(format!("Failed to open CA certificate file: {:?}", path.as_ref()))?;

    let mut reader = BufReader::new(file);

    let mut root_store = RootCertStore::empty();

    let certs: Vec<Certificate> = certs(&mut reader)?
        .into_iter()
        .map(Certificate)
        .collect();

    if certs.is_empty() {
        anyhow::bail!("No CA certificates found in file: {:?}", path.as_ref());
    }

    for cert in certs {
        root_store.add(&cert)
            .context("Failed to add CA certificate to root store")?;
    }

    Ok(root_store)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_ca_cert() {
        let ca_cert = load_ca_cert("certs/ca.crt");
        assert!(ca_cert.is_ok(), "Failed to load CA certificate");
    }

    #[test]
    fn test_load_gateway_cert() {
        let cert = load_cert("certs/gateway-a.crt");
        assert!(cert.is_ok(), "Failed to load gateway certificate");
        assert!(!cert.unwrap().is_empty(), "Certificate list is empty");
    }

    #[test]
    fn test_load_private_key() {
        let key = load_private_key("certs/gateway-a.key");
        assert!(key.is_ok(), "Failed to load private key");
    }
}
