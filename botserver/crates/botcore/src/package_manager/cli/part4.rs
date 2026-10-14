use super::*;

pub(super) async fn vault_put(path: &str, kvs: &[&str]) -> Result<()> {
    let manager = SecretsManager::get()?.clone();
    if !manager.is_enabled() {
        return Err(anyhow::anyhow!("Vault not configured"));
    }

    let mut data: HashMap<String, String> = HashMap::new();

    // If no key=value provided, enter interactive mode
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
        // Command line mode
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
pub(super) async fn vault_get(path: &str, key: Option<&str>) -> Result<()> {
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
            println!("  {}={}", k, masked);
        }
    }

    Ok(())
}
pub(super) fn vault_list() -> Result<()> {
    println!("Configured secret paths:");
    println!("  {} - Database credentials", SecretPaths::TABLES);
    println!("  {} - S3/MinIO credentials", SecretPaths::DRIVE);
    println!("  {} - Redis credentials", SecretPaths::CACHE);
    println!("  {} - SMTP credentials", SecretPaths::EMAIL);
    println!("  {} - Zitadel credentials", SecretPaths::DIRECTORY);
    println!("  {} - AI API keys", SecretPaths::LLM);
    println!("  {} - Encryption keys", SecretPaths::ENCRYPTION);
    println!("  {} - LiveKit credentials", SecretPaths::MEET);
    println!("  {} - Forgejo credentials", SecretPaths::ALM);
    println!("  {} - Qdrant credentials", SecretPaths::VECTORDB);
    println!("  {} - InfluxDB credentials", SecretPaths::OBSERVABILITY);
    println!("  gbo/stripe - Payment credentials");
    println!("  gbo/custom - Custom database");

    Ok(())
}
pub(super) async fn print_version(show_all: bool) -> Result<()> {
    println!("botserver {}", VERSION);

    if show_all {
        println!();
        println!("Build Information:");
        println!("  rustc: {}", rustc_version());
        println!("  target: {}", std::env::consts::ARCH);
        println!("  os: {}", std::env::consts::OS);
        println!();

        let mode = InstallMode::Local;
        if let Ok(pm) = PackageManager::new(mode, None) {
            println!("Installed Components:");
            let components = pm.list();
            let mut installed_count = 0;
            for component in &components {
                if pm.is_installed(component) {
                    println!("  * {} (installed)", component);
                    installed_count += 1;
                }
            }
            if installed_count == 0 {
                println!("  (none)");
            }
            println!();
            println!("Available Components: {}", components.len());
        }

        println!();
        println!("Secrets:");
        if let Ok(manager) = SecretsManager::get() {
            if manager.is_enabled() {
                match manager.health_check().await {
                    Ok(true) => println!("  Vault: connected"),
                    _ => println!("  Vault: not reachable"),
                }
            } else {
                println!("  Vault: not configured");
            }
        } else {
            println!("  Vault: not configured");
        }
    }

    Ok(())
}
pub(super) fn rustc_version() -> String {
    SafeCommand::new("rustc")
        .and_then(|c| c.arg("--version"))
        .ok()
        .and_then(|cmd| cmd.execute().ok())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
pub(super) fn generate_password(length: usize) -> String {
    crate::package_manager::generate_random_string(length)
}
pub(super) fn generate_access_key() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::rng();
    (0..20)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}
pub(super) fn generate_secret_key() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut rng = rand::rng();
    (0..40)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}
