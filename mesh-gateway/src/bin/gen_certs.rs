use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, PKCS_ECDSA_P256_SHA256};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 Generating certificates for mesh gateway network...\n");

    // Create certs directory if it doesn't exist
    fs::create_dir_all("certs")?;

    // Generate Root CA
    println!("1. Generating Root CA...");
    let ca = generate_ca()?;

    // Save CA certificate and key
    fs::write("certs/ca.crt", ca.serialize_pem()?)?;
    fs::write("certs/ca.key", ca.serialize_private_key_pem())?;
    println!("   ✓ Saved certs/ca.crt");
    println!("   ✓ Saved certs/ca.key\n");

    // Generate gateway certificates
    let gateways = vec!["gateway-a", "gateway-b", "gateway-c"];

    for gateway_id in gateways {
        println!("2. Generating certificate for {}...", gateway_id);
        let cert = generate_gateway_cert(gateway_id, &ca)?;

        let cert_file = format!("certs/{}.crt", gateway_id);
        let key_file = format!("certs/{}.key", gateway_id);

        // Serialize certificate signed by CA
        fs::write(&cert_file, cert.serialize_pem_with_signer(&ca)?)?;
        fs::write(&key_file, cert.serialize_private_key_pem())?;

        println!("   ✓ Saved {}", cert_file);
        println!("   ✓ Saved {}\n", key_file);
    }

    println!("✅ All certificates generated successfully!");
    println!("\n📁 Certificate files created in ./certs/");
    println!("   - ca.crt, ca.key (Root CA)");
    println!("   - gateway-a.crt, gateway-a.key");
    println!("   - gateway-b.crt, gateway-b.key");
    println!("   - gateway-c.crt, gateway-c.key");

    Ok(())
}

/// Generate a self-signed Root CA certificate
fn generate_ca() -> Result<rcgen::Certificate, Box<dyn std::error::Error>> {
    let mut params = CertificateParams::default();

    // Set CA-specific parameters
    params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    params.key_usages = vec![
        rcgen::KeyUsagePurpose::KeyCertSign,
        rcgen::KeyUsagePurpose::CrlSign,
    ];

    // Set distinguished name
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "MeshNet Root CA");
    dn.push(DnType::OrganizationName, "Solitude Labs POC");
    dn.push(DnType::CountryName, "US");
    params.distinguished_name = dn;

    // Set validity period (1 year)
    params.not_before = time::OffsetDateTime::now_utc();
    params.not_after = params.not_before + time::Duration::days(365);

    // Generate key pair
    params.alg = &PKCS_ECDSA_P256_SHA256;
    let _key_pair = KeyPair::generate(&PKCS_ECDSA_P256_SHA256)?;

    // Generate certificate
    let cert = rcgen::Certificate::from_params(params)?;

    Ok(cert)
}

/// Generate a gateway certificate signed by the CA
fn generate_gateway_cert(
    gateway_id: &str,
    _ca_cert: &rcgen::Certificate,
) -> Result<rcgen::Certificate, Box<dyn std::error::Error>> {
    let mut params = CertificateParams::default();

    // Set distinguished name
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, gateway_id);
    dn.push(DnType::OrganizationName, "Solitude Labs POC");
    dn.push(DnType::OrganizationalUnitName, "Mesh Gateway");
    params.distinguished_name = dn;

    // Add subject alternative names (for TLS)
    params.subject_alt_names = vec![
        rcgen::SanType::DnsName(gateway_id.to_string()),
        rcgen::SanType::DnsName("localhost".to_string()),
        rcgen::SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
    ];

    // Set key usage
    params.key_usages = vec![
        rcgen::KeyUsagePurpose::DigitalSignature,
        rcgen::KeyUsagePurpose::KeyEncipherment,
    ];

    // Extended key usage (TLS server and client)
    params.extended_key_usages = vec![
        rcgen::ExtendedKeyUsagePurpose::ServerAuth,
        rcgen::ExtendedKeyUsagePurpose::ClientAuth,
    ];

    // Set validity period (1 year)
    params.not_before = time::OffsetDateTime::now_utc();
    params.not_after = params.not_before + time::Duration::days(365);

    // Generate key pair and set algorithm
    params.alg = &PKCS_ECDSA_P256_SHA256;

    // Generate certificate signed by CA
    let cert = rcgen::Certificate::from_params(params)?;

    Ok(cert)
}
