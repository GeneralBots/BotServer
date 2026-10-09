use super::*;

pub(super) fn generate_api_key() -> (String, String) {
    let mut rng = rand::rng();

    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

    let random_part: String = (0..API_KEY_LENGTH)
        .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
        .collect();

    let secret = format!("{API_KEY_PREFIX}{random_part}");
    let hash = hash_api_key(&secret);

    (secret, hash)
}
pub(super) fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());

    for _ in 0..API_KEY_HASH_ITERATIONS {
        let result = hasher.finalize_reset();
        hasher.update(result);
    }

    let result = hasher.finalize();
    hex::encode(result)
}
pub(super) fn extract_prefix(key: &str) -> String {
    let stripped = key.strip_prefix(API_KEY_PREFIX).unwrap_or(key);
    format!("{API_KEY_PREFIX}{}...", &stripped[..8.min(stripped.len())])
}
pub(super) fn matches_cidr(ip: &str, cidr: &str) -> bool {
    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 {
        return false;
    }

    let network = parts[0];
    let prefix_len: u8 = match parts[1].parse() {
        Ok(p) => p,
        Err(_) => return false,
    };

    let ip_octets: Vec<u8> = match ip
        .split('.')
        .map(|s| s.parse())
        .collect::<Result<Vec<u8>, _>>()
    {
        Ok(o) if o.len() == 4 => o,
        _ => return false,
    };

    let network_octets: Vec<u8> = match network
        .split('.')
        .map(|s| s.parse())
        .collect::<Result<Vec<u8>, _>>()
    {
        Ok(o) if o.len() == 4 => o,
        _ => return false,
    };

    let ip_u32 = u32::from_be_bytes([ip_octets[0], ip_octets[1], ip_octets[2], ip_octets[3]]);
    let network_u32 = u32::from_be_bytes([
        network_octets[0],
        network_octets[1],
        network_octets[2],
        network_octets[3],
    ]);

    let mask = if prefix_len >= 32 {
        u32::MAX
    } else {
        u32::MAX << (32 - prefix_len)
    };

    (ip_u32 & mask) == (network_u32 & mask)
}
pub fn extract_api_key_from_header(header_value: &str) -> Option<&str> {
    if let Some(key) = header_value.strip_prefix("Bearer ") {
        return Some(key);
    }
    if let Some(key) = header_value.strip_prefix("bearer ") {
        return Some(key);
    }
    if let Some(key) = header_value.strip_prefix("ApiKey ") {
        return Some(key);
    }
    if let Some(key) = header_value.strip_prefix("api-key ") {
        return Some(key);
    }
    if header_value.starts_with(API_KEY_PREFIX) {
        return Some(header_value);
    }
    None
}
