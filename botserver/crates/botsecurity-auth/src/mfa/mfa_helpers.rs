use rand::Rng;
use sha2::{Sha256, Digest};
use hex;
use anyhow::anyhow;
use hmac::{Hmac, Mac};
use sha2::digest::KeyInit;
use crate::mfa::mfa_types::MfaMethod;
use crate::mfa::mfa_types::TOTP_DIGITS;
use crate::mfa::mfa_types::TOTP_SECRET_LENGTH;
use crate::mfa::mfa_types::RECOVERY_CODE_LENGTH;

pub fn generate_totp_secret() -> String {
    let mut rng = rand::rng();
    let secret: Vec<u8> = (0..TOTP_SECRET_LENGTH).map(|_| rng.random()).collect();
    hex::encode(secret)
}

pub fn generate_challenge() -> Vec<u8> {
    let mut rng = rand::rng();
    let challenge: Vec<u8> = (0..32).map(|_| rng.random()).collect();
    challenge
}

pub fn generate_recovery_code() -> String {
    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::rng();

    let code: String = (0..RECOVERY_CODE_LENGTH)
        .map(|_| CHARS[rng.random_range(0..CHARS.len())] as char)
        .collect();

    format!("{}-{}", &code[..4], &code[4..])
}

pub fn generate_otp_code() -> String {
    let mut rng = rand::rng();
    format!("{:06}", rng.random_range(0..1_000_000u32))
}

pub fn hash_recovery_code(code: &str) -> String {
    let normalized = code.replace('-', "").to_uppercase();
    let hash = Sha256::digest(normalized.as_bytes());
    hex::encode(hash)
}

pub fn verify_recovery_code(code: &str, hash: &str) -> bool {
    let code_hash = hash_recovery_code(code);
    constant_time_compare(&code_hash, hash)
}

pub fn hash_otp_code(code: &str) -> String {
    let hash = Sha256::digest(code.as_bytes());
    hex::encode(hash)
}

pub fn verify_otp_code(code: &str, hash: &str) -> bool {
    let code_hash = hash_otp_code(code);
    constant_time_compare(&code_hash, hash)
}

pub fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }
    result == 0
}

pub fn base32_encode(data: &str) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let bytes = data.as_bytes();
    let mut result = String::new();

    let mut buffer: u64 = 0;
    let mut bits_left = 0;

    for &byte in bytes {
        buffer = (buffer << 8) | (byte as u64);
        bits_left += 8;

        while bits_left >= 5 {
            bits_left -= 5;
            let index = ((buffer >> bits_left) & 0x1F) as usize;
            result.push(ALPHABET[index] as char);
        }
    }

    if bits_left > 0 {
        let index = ((buffer << (5 - bits_left)) & 0x1F) as usize;
        result.push(ALPHABET[index] as char);
    }

    result
}

pub fn verify_totp(secret: &str, code: &str, period: u64) -> bool {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let counter = now / period;

    for offset in [-1i64, 0, 1] {
        let check_counter = (counter as i64 + offset) as u64;
        if let Ok(expected) = generate_totp_code(secret, check_counter) {
            if constant_time_compare(&expected, code) {
                return true;
            }
        }
    }

    false
}

pub fn generate_totp_code(secret: &str, counter: u64) -> anyhow::Result<String> {
    let secret_bytes = hex::decode(secret).map_err(|e| anyhow!("Invalid secret: {e}"))?;

    let mut mac = <HmacSha256 as KeyInit>::new_from_slice(&secret_bytes)
        .map_err(|e| anyhow!("HMAC error: {e}"))?;

    mac.update(&counter.to_be_bytes());
    let result = mac.finalize().into_bytes();

    let offset = (result[result.len() - 1] & 0x0F) as usize;
    if offset + 4 > result.len() {
        return Err(anyhow!("Invalid HMAC result length"));
    }
    let code = u32::from_be_bytes([
        result[offset] & 0x7F,
        result[offset + 1],
        result[offset + 2],
        result[offset + 3],
    ]);

    let otp = code % 10u32.pow(TOTP_DIGITS);
    Ok(format!("{:0width$}", otp, width = TOTP_DIGITS as usize))
}


pub fn mask_destination(destination: &str, method: MfaMethod) -> String {
    match method {
        MfaMethod::EmailOtp => {
            if let Some(at_pos) = destination.find('@') {
                let local = &destination[..at_pos];
                let domain = &destination[at_pos..];
                if local.len() <= 2 {
                    format!("{}***{}", &local[..1], domain)
                } else {
                    format!("{}***{}{}", &local[..2], &local[local.len()-1..], domain)
                }
            } else {
                "***@***".into()
            }
        }
        MfaMethod::SmsOtp => {
            let digits: String = destination.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() <= 4 {
                "***".into()
            } else {
                format!("***{}", &digits[digits.len()-4..])
            }
        }
        _ => "***".into(),
    }
}

type HmacSha256 = Hmac<Sha256>;
