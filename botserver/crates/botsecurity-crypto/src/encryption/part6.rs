use super::*;

pub fn derive_key_from_password(password: &str, salt: &[u8]) -> Result<Vec<u8>> {
    if salt.len() < SALT_SIZE {
        return Err(anyhow!("Salt too short"));
    }

    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(salt);

    let mut result = hasher.finalize_reset();

    for _ in 0..PBKDF2_ITERATIONS {
        hasher.update(result);
        hasher.update(salt);
        result = hasher.finalize_reset();
    }

    Ok(result[..KEY_SIZE].to_vec())
}
pub fn generate_salt() -> Vec<u8> {
    let mut rng = rand::rng();
    (0..SALT_SIZE).map(|_| rng.random()).collect()
}
pub fn encrypt_field(plaintext: &str, key: &[u8]) -> Result<String> {
    let (nonce, ciphertext) = encrypt_aes_gcm(plaintext.as_bytes(), key)?;
    Ok(format!("1:{}:{}", nonce, ciphertext))
}
pub fn decrypt_field(encrypted: &str, key: &[u8]) -> Result<String> {
    let parts: Vec<&str> = encrypted.split(':').collect();
    if parts.len() != 3 {
        return Err(anyhow!("Invalid encrypted field format"));
    }

    let version: u32 = parts[0].parse().map_err(|_| anyhow!("Invalid version"))?;
    if version != 1 {
        return Err(anyhow!("Unsupported encryption version: {version}"));
    }

    let decrypted = decrypt_aes_gcm(parts[1], parts[2], key)?;
    String::from_utf8(decrypted).map_err(|e| anyhow!("Invalid UTF-8: {e}"))
}
pub fn hash_for_search(plaintext: &str, pepper: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(plaintext.as_bytes());
    hasher.update(pepper);
    hex::encode(hasher.finalize())
}
/// Deriva uma chave de escopo a partir da chave mestre + identificador de escopo (branch_id / bot_id).
/// Usa SHA-256(domain ":" master_key ":" scope_id) para isolamento criptográfico entre escopos.
pub fn derive_scope_key(master_key: &[u8], domain: &str, scope_id: &Uuid) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    hasher.update(b":");
    hasher.update(master_key);
    hasher.update(b":");
    hasher.update(scope_id.as_bytes());
    let result = hasher.finalize();
    result.to_vec()
}
