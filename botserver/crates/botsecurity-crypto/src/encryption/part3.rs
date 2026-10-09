use super::*;

impl EncryptedData {
    pub fn to_string_compact(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.version,
            self.algorithm,
            self.key_id.as_deref().unwrap_or("default"),
            self.nonce,
            self.ciphertext
        )
    }

    pub fn from_string_compact(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() < 5 {
            return Err(anyhow!("Invalid encrypted data format"));
        }

        let version: u32 = parts[0].parse().map_err(|_| anyhow!("Invalid version"))?;
        let algorithm = parts[1].to_string();
        let key_id = if parts[2] == "default" {
            None
        } else {
            Some(parts[2].to_string())
        };
        let nonce = parts[3].to_string();
        let ciphertext = parts[4..].join(":");

        Ok(Self {
            algorithm,
            nonce,
            ciphertext,
            key_id,
            version,
        })
    }
}
