use botcoresecrets::SecretsManager;
use anyhow::Result;
use std::collections::HashMap;

pub fn install_protection() -> Result<()> {
    eprintln!("Error: This command requires root privileges.");
    eprintln!();
    eprintln!("Run with: sudo botserver install protection");
    Ok(())
}

pub fn remove_protection() -> Result<()> {
    eprintln!("Error: This command requires root privileges.");
    eprintln!();
    eprintln!("Run with: sudo botserver remove protection");
    Ok(())
}

pub fn verify_protection() -> String {
    "Protection verification requires botsecurity integration".to_string()
}

pub async fn vault_put(path: &str, kvs: &[&str]) -> Result<()> {
    let manager = SecretsManager::get()?.clone();
    if !manager.is_enabled() {
        return Err(anyhow::anyhow!("Vault not configured"));
    }

    let mut data: HashMap<String, String> = HashMap::new();

    if kvs.is_empty() {
        println!("\n=== Interactive Vault Store ===");
        println!("Path: {}", path);
        println!("Enter values (press Enter with empty key to finish):\n");

        loop {
            print!("Key: ");
            std::io::Write::flush(&mut std::io::stdout())?;
            let mut key = String::new();
            std::io::stdin().read_line(&mut key)?;
            let key = key.trim().to_string();

            if key.is_empty() {
                break;
            }

            print!("Value: ");
            std::io::Write::flush(&mut std::io::stdout())?;
            let mut value = String::new();
            std::io::stdin().read_line(&mut value)?;
            let value = value.trim().to_string();

            if value.is_empty() {
                eprintln!("Value cannot be empty, skipping '{}'", key);
                continue;
            }

            data.insert(key.clone(), value);
            println!("* Saved '{}'\n", key);
        }

        if data.is_empty() {
            return Err(anyhow::anyhow!("No values provided"));
        }
    } else {
        for kv in kvs {
            if let Some((k, v)) = kv.split_once('=') {
                data.insert(k.to_string(), v.to_string());
            } else {
                eprintln!("Invalid key=value pair: {}", kv);
            }
        }

        if data.is_empty() {
            return Err(anyhow::anyhow!("No valid key=value pairs provided"));
        }
    }

    manager.put_secret(path, data.clone()).await?;
    println!("\n✓ Stored {} key(s) at {}", data.len(), path);

    Ok(())
}

pub async fn vault_get(path: &str, key: Option<&str>) -> Result<()> {
    let manager = SecretsManager::get()?.clone();
    if !manager.is_enabled() {
        return Err(anyhow::anyhow!("Vault not configured"));
    }

    let secrets = manager.get_secret(path).await?;

    if let Some(k) = key {
        if let Some(v) = secrets.get(k) {
            println!("{}", v);
        } else {
            eprintln!("Key '{}' not found at {}", k, path);
        }
    } else {
        println!("Secrets at {}:", path);
        for (k, v) in &secrets {
            let masked = if k.contains("password")
                || k.contains("secret")
                || k.contains("key")
                || k.contains("token")
            {
                format!("{}...", v.chars().take(4).collect::<String>())
            } else {
                v.clone()
            };
            println!(" {}={}", k, masked);
        }
    }

    Ok(())
}

pub async fn vault_health() -> Result<()> {
    let manager = SecretsManager::get()?.clone();

    if !manager.is_enabled() {
        println!("x Vault not configured");
        println!(" Set VAULT_ADDR and VAULT_TOKEN environment variables");
        return Ok(());
    }

    match manager.health_check().await {
        Ok(true) => {
            println!("* Vault is healthy");
            println!(" Address: {}", std::env::var("VAULT_ADDR").unwrap_or_default());
        }
        Ok(false) => {
            println!("x Vault health check failed");
        }
        Err(e) => {
            println!("x Vault error: {}", e);
        }
    }

    Ok(())
}
