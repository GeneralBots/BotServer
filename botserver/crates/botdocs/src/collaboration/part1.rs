

pub(super) fn extract_user_from_token(token: &str) -> Option<(String, String)> {
    if token.is_empty() {
        return None;
    }
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload_b64 = parts[1].replace('-', "+").replace('_', "/");
    let padding = (4 - payload_b64.len() % 4) % 4;
    let padded = format!("{}{}", payload_b64, "=".repeat(padding));
    let chars: Vec<u8> = padded.bytes().collect();
    let mut out = Vec::with_capacity(chars.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for &c in &chars {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            _ => return None,
        };
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1u32 << bits) - 1;
        }
    }
    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&out) {
        let sub = v.get("sub").and_then(|x| x.as_str())
            .or_else(|| v.get("user_id").and_then(|x| x.as_str()))
            .or_else(|| v.get("email").and_then(|x| x.as_str()));
        let name = v.get("name").and_then(|x| x.as_str())
            .or_else(|| v.get("display_name").and_then(|x| x.as_str()))
            .or_else(|| v.get("preferred_username").and_then(|x| x.as_str()))
            .or_else(|| v.get("email").and_then(|x| x.as_str()));
        if let (Some(s), Some(n)) = (sub, name) {
            return Some((s.to_string(), n.to_string()));
        }
        if let Some(s) = sub {
            return Some((s.to_string(), s.to_string()));
        }
    }
    None
}
