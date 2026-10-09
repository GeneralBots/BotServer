pub(crate) use std::fmt;

#[derive(Default)]
pub struct SecretString {
    inner: String,
}

#[derive(Default)]
pub struct SecretBytes {
    inner: Vec<u8>,
}

#[derive(Clone)]
pub struct ApiKey {
    key: SecretString,
    provider: String,
}

#[derive(Clone)]
pub struct DatabaseCredentials {
    username: String,
    password: SecretString,
    host: String,
    port: u16,
    database: String,
}

#[derive(Clone)]
pub struct JwtSecret {
    secret: SecretBytes,
    algorithm: String,
}

#[derive(Clone, Default)]
pub struct SecretsStore {
    api_keys: std::collections::HashMap<String, ApiKey>,
    database_credentials: Option<DatabaseCredentials>,
    jwt_secret: Option<JwtSecret>,
    custom_secrets: std::collections::HashMap<String, SecretString>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_string_redaction() -> Result<(), Box<dyn std::error::Error>> {
        let secret = SecretString::new("my-super-secret-password".to_string());
        assert_eq!(format!("{:?}", secret), "[REDACTED]");
        assert_eq!(format!("{}", secret), "[REDACTED]");
        Ok(())
    }

    #[test]
    fn test_secret_string_expose() {
        let secret = SecretString::new("my-password".to_string());
        assert_eq!(secret.expose_secret(), "my-password");
    }

    #[test]
    fn test_secret_bytes_redaction() {
        let secret = SecretBytes::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(format!("{:?}", secret), "[REDACTED BYTES]");
    }

    #[test]
    fn test_api_key_masked() {
        let key = ApiKey::new("sk-1234567890abcdef1234567890abcdef", "openai");
        let masked = key.masked();
        assert!(masked.starts_with("sk-1"));
        assert!(masked.ends_with("cdef"));
        assert!(masked.contains("..."));
    }

    #[test]
    fn test_api_key_short() {
        let key = ApiKey::new("short", "test");
        let masked = key.masked();
        assert_eq!(masked, "*****");
    }

    #[test]
    fn test_database_credentials_from_url() -> Result<(), Box<dyn std::error::Error>> {
        let url = "postgres://user:pass@localhost:5432/mydb";
        let creds = DatabaseCredentials::from_url(url).ok_or("Failed to parse URL")?;
        assert_eq!(creds.username(), "user");
        assert_eq!(creds.expose_password(), "pass");
        assert_eq!(creds.host(), "localhost");
        assert_eq!(creds.port(), 5432);
        assert_eq!(creds.database(), "mydb");
        Ok(())
    }

    #[test]
    fn test_database_credentials_safe_string() {
        let db_pass = String::from_utf8_lossy(b"secret").to_string();
        let creds = DatabaseCredentials::new("user", db_pass, "localhost", 5432, "db");
        let safe = creds.to_safe_string();
        assert!(!safe.contains("secret"));
        assert!(safe.contains("****"));
    }

    #[test]
    fn test_redact_sensitive_data() {
        let text = "password=secret123 and api_key=abc123";
        let redacted = redact_sensitive_data(text);
        assert!(!redacted.contains("secret123"));
        assert!(!redacted.contains("abc123"));
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn test_is_sensitive_key() {
        assert!(is_sensitive_key("password"));
        assert!(is_sensitive_key("API_KEY"));
        assert!(is_sensitive_key("secret_token"));
        assert!(is_sensitive_key("db_password"));
        assert!(!is_sensitive_key("username"));
        assert!(!is_sensitive_key("email"));
    }

    #[test]
    fn test_secrets_store() {
        let mut store = SecretsStore::new();

        store.add_api_key("openai", ApiKey::new("sk-test", "openai"));
        assert!(store.get_api_key("openai").is_some());
        assert!(store.get_api_key("nonexistent").is_none());

        store.add_custom_secret("my_secret", "value");
        assert!(store.get_custom_secret("my_secret").is_some());

        store.clear();
        assert!(store.get_api_key("openai").is_none());
    }

    #[test]
    fn test_secret_string_default() {
        let secret: SecretString = Default::default();
        assert!(secret.is_empty());
        assert_eq!(secret.len(), 0);
    }

    #[test]
    fn test_secret_bytes_from() {
        let bytes: SecretBytes = vec![1, 2, 3].into();
        assert_eq!(bytes.len(), 3);

        let bytes2: SecretBytes = [4u8, 5, 6].as_slice().into();
        assert_eq!(bytes2.len(), 3);
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;

pub use part6::*;
