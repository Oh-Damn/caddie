use rcgen::{
    CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose, SanType,
};
use sha2::{Digest, Sha256};
use std::net::IpAddr;
use std::path::Path;
use time::{Duration, OffsetDateTime};

const VALID_DAYS: i64 = 364;

const TLS_VERSION: u32 = 3;

#[derive(Clone)]
pub struct TlsMaterial {
    pub cert_pem: Vec<u8>,
    pub key_pem: Vec<u8>,
    pub cert_fingerprint: String,
}

pub fn load_or_create(
    data_dir: &Path,
    lan_ip: &str,
    host_fqdn: &str,
) -> Result<TlsMaterial, Box<dyn std::error::Error>> {
    let cert_path = data_dir.join("tls-cert.pem");
    let key_path = data_dir.join("tls-key.pem");
    let host_path = data_dir.join("tls-host.txt");

    let stamp = format!("{TLS_VERSION}\n{host_fqdn}");
    let host_ok = cert_path.exists()
        && key_path.exists()
        && std::fs::read_to_string(&host_path)
            .map(|stored| stored.trim() == stamp)
            .unwrap_or(false);
    if host_ok {
        let cert_pem = std::fs::read(&cert_path)?;
        let key_pem = std::fs::read(&key_path)?;
        return Ok(TlsMaterial {
            cert_fingerprint: fingerprint_pem(&cert_pem),
            cert_pem,
            key_pem,
        });
    }
    let material = generate(lan_ip, host_fqdn)?;
    std::fs::write(&cert_path, &material.cert_pem)?;
    std::fs::write(&key_path, &material.key_pem)?;
    std::fs::write(&host_path, &stamp)?;
    Ok(material)
}

fn generate(lan_ip: &str, host_fqdn: &str) -> Result<TlsMaterial, Box<dyn std::error::Error>> {
    let mut params = CertificateParams::default();
    params
        .distinguished_name
        .push(DnType::CommonName, host_fqdn.trim_end_matches('.'));
    for name in [host_fqdn, crate::protocol::LAN_HOST] {
        let dns = name.trim_end_matches('.');
        if dns.is_empty() {
            continue;
        }
        params
            .subject_alt_names
            .push(SanType::DnsName(dns.try_into()?));
    }
    if let Ok(ip) = lan_ip.parse::<IpAddr>() {
        params.subject_alt_names.push(SanType::IpAddress(ip));
    }

    params.is_ca = IsCa::NoCa;
    params.key_usages.extend([
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ]);
    params
        .extended_key_usages
        .push(ExtendedKeyUsagePurpose::ServerAuth);
    let now = OffsetDateTime::now_utc();
    params.not_before = now - Duration::days(1);
    params.not_after = now + Duration::days(VALID_DAYS);
    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;
    let cert_pem = cert.pem().into_bytes();
    let key_pem = key_pair.serialize_pem().into_bytes();
    Ok(TlsMaterial {
        cert_fingerprint: fingerprint_pem(&cert_pem),
        cert_pem,
        key_pem,
    })
}

fn fingerprint_pem(pem: &[u8]) -> String {
    hex::encode(&Sha256::digest(pem)[..4])
}
