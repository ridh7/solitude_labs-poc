use anyhow::{Context, Result};
use reqwest::{Client, Identity};
use std::fs;
use std::path::Path;

/// Creates an HTTPS client configured for mTLS
pub fn create_mtls_client(
    cert_path: impl AsRef<Path>,
    key_path: impl AsRef<Path>,
    ca_cert_path: impl AsRef<Path>,
) -> Result<Client> {
    // Load client certificate and key
    let cert_pem = fs::read(cert_path.as_ref())
        .context(format!("Failed to read certificate: {:?}", cert_path.as_ref()))?;

    let key_pem = fs::read(key_path.as_ref())
        .context(format!("Failed to read private key: {:?}", key_path.as_ref()))?;

    // Combine cert and key for reqwest Identity
    let mut pem = cert_pem;
    pem.extend_from_slice(&key_pem);

    let identity = Identity::from_pem(&pem)
        .context("Failed to create identity from certificate and key")?;

    // Load CA certificate
    let ca_cert = fs::read(ca_cert_path.as_ref())
        .context(format!("Failed to read CA certificate: {:?}", ca_cert_path.as_ref()))?;

    let ca_cert = reqwest::Certificate::from_pem(&ca_cert)
        .context("Failed to parse CA certificate")?;

    // Build the client with mTLS configuration
    let client = Client::builder()
        .identity(identity)
        .add_root_certificate(ca_cert)
        .use_rustls_tls()
        .build()
        .context("Failed to build HTTPS client")?;

    Ok(client)
}

/// Makes a GET request to the specified URL
pub async fn get(client: &Client, url: &str) -> Result<String> {
    let response = client
        .get(url)
        .send()
        .await
        .context(format!("Failed to send GET request to {}", url))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .context("Failed to read response body")?;

    if !status.is_success() {
        anyhow::bail!("Request failed with status {}: {}", status, body);
    }

    Ok(body)
}

/// Makes a POST request with JSON body
pub async fn post_json(client: &Client, url: &str, json_body: &str) -> Result<String> {
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(json_body.to_string())
        .send()
        .await
        .context(format!("Failed to send POST request to {}", url))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .context("Failed to read response body")?;

    if !status.is_success() {
        anyhow::bail!("Request failed with status {}: {}", status, body);
    }

    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_client() {
        let client = create_mtls_client(
            "certs/gateway-a.crt",
            "certs/gateway-a.key",
            "certs/ca.crt",
        );
        assert!(client.is_ok(), "Failed to create mTLS client");
    }
}
