use super::*;

pub(super) fn base64_url_decode(input: &str) -> Result<Vec<u8>, String> {
    let input = input.replace('-', "+").replace('_', "/");

    let padding = match input.len() % 4 {
        0 => "",
        2 => "==",
        3 => "=",
        _ => return Err("Invalid base64 length".to_string()),
    };

    let padded = format!("{}{}", input, padding);

    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(&padded)
        .map_err(|e| format!("Base64 decode error: {}", e))
}
