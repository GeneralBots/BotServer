//! Brazilian tax validators (CNPJ, CPF, IE, IM, CEP, access key).

use crate::models::Address;

const CNPJ_WEIGHTS_DV1: [u8; 12] = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
const CNPJ_WEIGHTS_DV2: [u8; 13] = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
const CPF_WEIGHTS_DV1: [u8; 9] = [10, 9, 8, 7, 6, 5, 4, 3, 2];
const CPF_WEIGHTS_DV2: [u8; 10] = [11, 10, 9, 8, 7, 6, 5, 4, 3, 2];

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TaxError {
    #[error("invalid CNPJ: {0}")]
    InvalidCnpj(String),
    #[error("invalid CPF: {0}")]
    InvalidCpf(String),
    #[error("invalid state registration: {0}")]
    InvalidIe(String),
    #[error("invalid municipal registration: {0}")]
    InvalidIm(String),
    #[error("invalid CEP: {0}")]
    InvalidCep(String),
    #[error("invalid access key: {0}")]
    InvalidAccessKey(String),
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    #[error("document is not in a state that allows the operation: {0}")]
    InvalidState(String),
}

pub fn only_digits(input: &str) -> String {
    input.chars().filter(|c| c.is_ascii_digit()).collect()
}

fn compute_dv(digits: &[u8], weights: &[u8]) -> u8 {
    let sum: u32 = digits
        .iter()
        .zip(weights.iter())
        .map(|(d, w)| (*d as u32) * (*w as u32))
        .sum();
    let remainder = sum % 11;
    if remainder < 2 {
        0
    } else {
        (11 - remainder) as u8
    }
}

pub fn is_valid_cnpj(input: &str) -> bool {
    let digits = only_digits(input);
    if digits.len() != 14 {
        return false;
    }
    if digits.chars().all(|c| c == digits.chars().next().unwrap_or('0')) {
        return false;
    }
    let bytes: Vec<u8> = digits.bytes().map(|b| b - b'0').collect();
    let dv1 = compute_dv(&bytes[..12], &CNPJ_WEIGHTS_DV1);
    if dv1 != bytes[12] {
        return false;
    }
    let dv2 = compute_dv(&bytes[..13], &CNPJ_WEIGHTS_DV2);
    dv2 == bytes[13]
}

pub fn is_valid_cpf(input: &str) -> bool {
    let digits = only_digits(input);
    if digits.len() != 11 {
        return false;
    }
    if digits.chars().all(|c| c == digits.chars().next().unwrap_or('0')) {
        return false;
    }
    let bytes: Vec<u8> = digits.bytes().map(|b| b - b'0').collect();
    let dv1 = compute_dv(&bytes[..9], &CPF_WEIGHTS_DV1);
    if dv1 != bytes[9] {
        return false;
    }
    let dv2 = compute_dv(&bytes[..10], &CPF_WEIGHTS_DV2);
    dv2 == bytes[10]
}

pub fn is_valid_cep(input: &str) -> bool {
    let digits = only_digits(input);
    digits.len() == 8
}

pub fn is_valid_nfe_access_key(input: &str) -> bool {
    let digits = only_digits(input);
    if digits.len() != 44 {
        return false;
    }
    let bytes: Vec<u8> = digits.bytes().map(|b| b - b'0').collect();
    let weights: [u8; 43] = std::array::from_fn(|i| (43 - i) as u8);
    let sum: u32 = bytes[..43]
        .iter()
        .zip(weights.iter())
        .map(|(d, w)| (*d as u32) * (*w as u32))
        .sum();
    let remainder = sum % 11;
    let dv = if remainder < 2 { 0 } else { (11 - remainder) as u8 };
    dv == bytes[43]
}

pub fn format_cnpj(input: &str) -> String {
    let d = only_digits(input);
    if d.len() != 14 {
        return input.to_string();
    }
    format!("{}.{}.{}/{}-{}", &d[0..2], &d[2..5], &d[5..8], &d[8..12], &d[12..14])
}

pub fn format_cpf(input: &str) -> String {
    let d = only_digits(input);
    if d.len() != 11 {
        return input.to_string();
    }
    format!("{}.{}.{}-{}", &d[0..3], &d[3..6], &d[6..9], &d[9..11])
}

pub fn format_cep(input: &str) -> String {
    let d = only_digits(input);
    if d.len() != 8 {
        return input.to_string();
    }
    format!("{}-{}", &d[0..5], &d[5..8])
}

/// Heuristic state registration validator for the most common states. Returns
/// `Ok(())` when the registration shape is plausible for the given state. This
/// is not a substitute for a full SEFAZ check; it exists to catch obvious
/// formatting mistakes in the UI/API path.
pub fn is_ie_shape_valid(state: &str, registration: &str) -> bool {
    let d = only_digits(registration);
    match state.to_ascii_uppercase().as_str() {
        "AC" => d.len() == 13,
        "AL" => d.len() == 9,
        "AP" => d.len() == 9,
        "AM" => d.len() == 9,
        "BA" => d.len() == 8 || d.len() == 9,
        "CE" => d.len() == 9,
        "DF" => d.len() == 13,
        "ES" => d.len() == 9,
        "GO" => d.len() == 9,
        "MA" => d.len() == 9,
        "MT" => d.len() == 11,
        "MS" => d.len() == 9,
        "MG" => d.len() == 13,
        "PA" => d.len() == 9,
        "PB" => d.len() == 9,
        "PR" => d.len() == 10,
        "PE" => d.len() == 9 || d.len() == 14,
        "PI" => d.len() == 9,
        "RJ" => d.len() == 8,
        "RN" => d.len() == 9 || d.len() == 10,
        "RS" => d.len() == 10,
        "RO" => d.len() == 14,
        "RR" => d.len() == 9,
        "SC" => d.len() == 9,
        "SP" => d.len() == 12,
        "SE" => d.len() == 9,
        "TO" => d.len() == 9,
        _ => d.len() >= 8 && d.len() <= 14,
    }
}

pub fn validate_address(addr: &Address) -> Result<(), TaxError> {
    if !is_valid_cep(&addr.zip_code) {
        return Err(TaxError::InvalidCep(addr.zip_code.clone()));
    }
    if addr.state.is_empty() {
        return Err(TaxError::MissingField("state"));
    }
    if addr.city_code_ibge.is_empty() || addr.city_code_ibge.len() != 7 {
        return Err(TaxError::MissingField("city_code_ibge"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ibge::UF_SP;

    const VALID_CNPJ: &str = "11.222.333/0001-81";
    const INVALID_CNPJ: &str = "11.222.333/0001-82";
    const VALID_CPF: &str = "529.982.247-25";
    const INVALID_CPF: &str = "529.982.247-26";
    const VALID_CEP: &str = "01310-100";
    // Check digit (cDV) is the last position: modulo 11 over the preceding 43
    // digits with weights 43..1, yielding 3 for this body.
    const VALID_NFE_KEY: &str = "43250911122233300018155001000000010012345673";

    #[test]
    fn cnpj_validator() {
        assert!(is_valid_cnpj(VALID_CNPJ));
        assert!(!is_valid_cnpj(INVALID_CNPJ));
        assert!(!is_valid_cnpj("00000000000000"));
        assert!(!is_valid_cnpj("123"));
    }

    #[test]
    fn cpf_validator() {
        assert!(is_valid_cpf(VALID_CPF));
        assert!(!is_valid_cpf(INVALID_CPF));
        assert!(!is_valid_cpf("00000000000"));
    }

    #[test]
    fn cep_validator() {
        assert!(is_valid_cep(VALID_CEP));
        assert!(!is_valid_cep("123"));
    }

    #[test]
    fn nfe_access_key_validator() {
        assert!(is_valid_nfe_access_key(VALID_NFE_KEY));
        assert!(!is_valid_nfe_access_key("123"));
        // A tampered check digit must be rejected.
        assert!(!is_valid_nfe_access_key(&format!("{}8", &VALID_NFE_KEY[..43])));
        // A tampered body must invalidate the check digit as well.
        assert!(!is_valid_nfe_access_key(&format!(
            "{}1{}",
            &VALID_NFE_KEY[..5],
            &VALID_NFE_KEY[6..]
        )));
    }

    #[test]
    fn format_helpers_keep_digits() {
        assert_eq!(format_cnpj(VALID_CNPJ), VALID_CNPJ);
        assert_eq!(format_cpf(VALID_CPF), VALID_CPF);
        assert_eq!(format_cep(VALID_CEP), VALID_CEP);
    }

    #[test]
    fn ie_shape_per_state() {
        assert!(is_ie_shape_valid(UF_SP, "110042490114"));
        assert!(!is_ie_shape_valid(UF_SP, "123"));
        assert!(!is_ie_shape_valid("XX", "123456"));
    }
}
