use super::*;

pub fn compute_spki_fingerprint(cert_der: &[u8]) -> Result<Vec<u8>> {
    let (_, cert) = X509Certificate::from_der(cert_der)
        .map_err(|e| anyhow!("Failed to parse X.509 certificate: {}", e))?;

    let spki = cert.public_key().raw;

    let hash = digest(&SHA256, spki);

    Ok(hash.as_ref().to_vec())
}
pub fn compute_cert_fingerprint(cert_der: &[u8]) -> Vec<u8> {
    let hash = digest(&SHA256, cert_der);
    hash.as_ref().to_vec()
}
pub fn pem_to_der(pem_data: &[u8]) -> Result<Vec<u8>> {
    let pem_str = std::str::from_utf8(pem_data).context("Invalid UTF-8 in PEM data")?;

    let start_marker = "-----BEGIN CERTIFICATE-----";
    let end_marker = "-----END CERTIFICATE-----";

    let start = pem_str
        .find(start_marker)
        .ok_or_else(|| anyhow!("No certificate found in PEM data"))?;

    let end = pem_str
        .find(end_marker)
        .ok_or_else(|| anyhow!("Invalid PEM: missing end marker"))?;

    let base64_data = &pem_str[start + start_marker.len()..end];
    let cleaned: String = base64_data.chars().filter(|c| !c.is_whitespace()).collect();

    BASE64
        .decode(&cleaned)
        .context("Failed to decode base64 certificate data")
}
pub fn format_fingerprint(hash: &[u8]) -> String {
    hash.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(":")
}
pub fn parse_fingerprint(formatted: &str) -> Result<Vec<u8>> {
    if let Some(base64_part) = formatted.strip_prefix("sha256//") {
        return BASE64
            .decode(base64_part)
            .context("Failed to decode base64 fingerprint");
    }

    if formatted.contains(':') {
        let bytes: Result<Vec<u8>, _> = formatted
            .split(':')
            .map(|hex| u8::from_str_radix(hex, 16))
            .collect();

        return bytes.context("Failed to parse hex fingerprint");
    }

    let bytes: Result<Vec<u8>, _> = (0..formatted.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&formatted[i..i + 2], 16))
        .collect();

    bytes.context("Failed to parse fingerprint")
}
