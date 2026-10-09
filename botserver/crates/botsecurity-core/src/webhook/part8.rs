use super::*;

pub(super) fn generate_webhook_secret() -> String {
    let mut rng = rand::rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.random()).collect();
    format!("whsec_{}", BASE64.encode(&bytes))
}
pub(super) fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }
    result == 0
}
pub fn parse_signature_header(header: &str) -> Option<(&str, &str)> {
    let parts: Vec<&str> = header.split('=').collect();
    if parts.len() == 2 {
        Some((parts[0], parts[1]))
    } else {
        None
    }
}
