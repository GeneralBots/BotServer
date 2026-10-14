const B64_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn encode_standard(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64_ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(B64_ALPHABET[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            B64_ALPHABET[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64_ALPHABET[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

pub fn decode_flexible(input: &str) -> Option<Vec<u8>> {
    let mut normalized = String::with_capacity(input.len());
    for c in input.trim().chars() {
        match c {
            '-' => normalized.push('+'),
            '_' => normalized.push('/'),
            c if !c.is_whitespace() => normalized.push(c),
            _ => {}
        }
    }
    if normalized.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(normalized.len() / 4 * 3);
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    for b in normalized.bytes() {
        if b == b'=' {
            break;
        }
        let v = match b {
            b'A'..=b'Z' => (b - b'A') as u32,
            b'a'..=b'z' => (b - b'a' + 26) as u32,
            b'0'..=b'9' => (b - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xFF) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_standard() {
        for sample in ["", "gbskill", "General Bots marketplace bundle \u{1F600}"] {
            let encoded = encode_standard(sample.as_bytes());
            assert_eq!(decode_flexible(&encoded), Some(sample.as_bytes().to_vec()));
        }
    }

    #[test]
    fn decode_urlsafe_matches_standard() {
        let encoded = encode_standard(b"payload-with-symbols??");
        let urlsafe = encoded.replace('+', "-").replace('/', "_");
        assert_eq!(decode_flexible(&urlsafe), Some(b"payload-with-symbols??".to_vec()));
    }

    #[test]
    fn rejects_invalid_length_and_chars() {
        assert!(decode_flexible("abcde").is_none());
        assert!(decode_flexible("**").is_none());
    }
}
