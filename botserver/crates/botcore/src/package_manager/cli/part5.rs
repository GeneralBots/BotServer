use super::*;

pub(super) async fn rotate_secret(component: &str) -> Result<()> {
    let manager = SecretsManager::get()?.clone();
    if !manager.is_enabled() {
        return Err(anyhow::anyhow!(
            "Vault not configured. Set VAULT_ADDR and VAULT_TOKEN"
        ));
    }

    println!("Rotating credentials for: {}", component);
    println!();

    match component {
        "tables" => {
            let new_password = generate_password(32);
            let mut secrets = manager
                .get_secret(SecretPaths::TABLES)
                .await
                .unwrap_or_default();
            let old_password = secrets.get("password").cloned().unwrap_or_default();
            secrets.insert("password".to_string(), new_password.clone());

            println!("⚠️  WARNING: You must update PostgreSQL with the new password!");
            println!();
            println!("Run this SQL command:");
            let default_username = "postgres".to_string();
            println!(
                "  ALTER USER {} WITH PASSWORD '[REDACTED — see Vault secret/gbo/tables]';",
                secrets.get("username").unwrap_or(&default_username)
            );
            println!();
            println!(
                "Old password: {}...",
                old_password.chars().take(4).collect::<String>()
            );
            println!(
                "New password: {}...",
                new_password.chars().take(4).collect::<String>()
            );

            print!("Save to Vault? [y/N]: ");
            std::io::Write::flush(&mut std::io::stdout())?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if input.trim().to_lowercase() == "y" {
                manager.put_secret(SecretPaths::TABLES, secrets).await?;
                println!("✓ Credentials saved to Vault");
                verify_rotation(component).await?;
            } else {
                println!("✗ Aborted");
            }
        }
        "drive" => {
            let new_accesskey = generate_access_key();
            let new_secret = generate_secret_key();
            let mut secrets = manager
                .get_secret(SecretPaths::DRIVE)
                .await
                .unwrap_or_default();

            println!("⚠️  WARNING: You must update MinIO with the new credentials!");
            println!();
            println!("Run these commands:");
            println!(
                "  mc admin user add myminio {} [REDACTED — see Vault secret/gbo/drive]",
                new_accesskey
            );
            println!(
                "  mc admin policy attach myminio readwrite --user {}",
                new_accesskey
            );
            println!();
            println!("New access key: {}", new_accesskey);
            println!(
                "New secret key: {}...",
                new_secret.chars().take(8).collect::<String>()
            );

            print!("Save to Vault? [y/N]: ");
            std::io::Write::flush(&mut std::io::stdout())?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if input.trim().to_lowercase() == "y" {
                secrets.insert("accesskey".to_string(), new_accesskey);
                secrets.insert("secret".to_string(), new_secret);
                manager.put_secret(SecretPaths::DRIVE, secrets).await?;
                println!("✓ Credentials saved to Vault");
            } else {
                println!("✗ Aborted");
            }
        }
        "cache" => {
            let new_password = generate_password(32);
            let mut secrets: HashMap<String, String> = HashMap::new();

            println!("⚠️  WARNING: You must update Valkey/Redis with the new password!");
            println!();
            println!("Run this command:");
            println!("  redis-cli CONFIG SET requirepass '[REDACTED — see Vault secret/gbo/cache]'");
            println!();
            println!(
                "New password: {}...",
                new_password.chars().take(4).collect::<String>()
            );

            print!("Save to Vault? [y/N]: ");
            std::io::Write::flush(&mut std::io::stdout())?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if input.trim().to_lowercase() == "y" {
                secrets.insert("password".to_string(), new_password);
                manager.put_secret(SecretPaths::CACHE, secrets).await?;
                println!("✓ Credentials saved to Vault");
            } else {
                println!("✗ Aborted");
            }
        }
        "email" => {
            let new_password = generate_password(24);
            let mut secrets = manager
                .get_secret(SecretPaths::EMAIL)
                .await
                .unwrap_or_default();

            println!("⚠️  WARNING: You must update the mail server with the new password!");
            println!();
            println!(
                "New password: {}...",
                new_password.chars().take(4).collect::<String>()
            );

            print!("Save to Vault? [y/N]: ");
            std::io::Write::flush(&mut std::io::stdout())?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if input.trim().to_lowercase() == "y" {
                secrets.insert("password".to_string(), new_password);
                manager.put_secret(SecretPaths::EMAIL, secrets).await?;
                println!("✓ Credentials saved to Vault");
            } else {
                println!("✗ Aborted");
            }
        }
        "encryption" => {
            let new_key = generate_password(64);
            let mut secrets: HashMap<String, String> = HashMap::new();

            println!("⚠️  CRITICAL WARNING: Rotating encryption key will make existing encrypted data unreadable!");
            println!("⚠️  Make sure to re-encrypt all data with the new key!");
            println!();
            println!(
                "New master key: {}...",
                new_key.chars().take(8).collect::<String>()
            );

            print!("Are you ABSOLUTELY sure? Type 'ROTATE' to confirm: ");
            std::io::Write::flush(&mut std::io::stdout())?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if input.trim() == "ROTATE" {
                secrets.insert("master_key".to_string(), new_key);
                manager.put_secret(SecretPaths::ENCRYPTION, secrets).await?;
                println!("✓ Encryption key saved to Vault");
            } else {
                println!("✗ Aborted");
            }
        }
        "directory" => {
            let new_secret = generate_password(48);
            let mut secrets = manager
                .get_secret(SecretPaths::DIRECTORY)
                .await
                .unwrap_or_default();

            println!("⚠️  WARNING: You must update Zitadel with the new client secret!");
            println!();
            println!(
                "New client secret: {}...",
                new_secret.chars().take(8).collect::<String>()
            );

            print!("Save to Vault? [y/N]: ");
            std::io::Write::flush(&mut std::io::stdout())?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if input.trim().to_lowercase() == "y" {
                secrets.insert("client_secret".to_string(), new_secret);
                manager.put_secret(SecretPaths::DIRECTORY, secrets).await?;
                println!("✓ Credentials saved to Vault");
            } else {
                println!("✗ Aborted");
            }
        }
        "jwt" => {
            let new_secret = generate_password(64);
            let env_path = std::env::current_dir()?.join(".env");

            println!("⚠️  JWT SECRET ROTATION");
            println!();
            println!("Current: JWT_SECRET in .env file");
            println!("Impact: ALL refresh tokens will become invalid immediately");
            println!("Access tokens (15 min) will expire naturally");
            println!();

            // Check if .env exists
            if !env_path.exists() {
                println!("✗ .env file not found at: {}", env_path.display());
                return Ok(());
            }

            // Read current JWT_SECRET for display
            let env_content = std::fs::read_to_string(&env_path)?;
            let current_jwt = env_content
                .lines()
                .find(|line| line.starts_with("JWT_SECRET="))
                .unwrap_or("JWT_SECRET=(not set)");

            println!("Current: {}", current_jwt.chars().take(40).collect::<String>());
            println!("New: {}... (64 chars)", new_secret.chars().take(8).collect::<String>());
            println!();

            // Backup .env
            let backup_path = format!("{}.backup.{}", env_path.display(), chrono::Utc::now().timestamp());
            std::fs::copy(&env_path, &backup_path)?;
            println!("✓ Backup created: {}", backup_path);
            println!();

            print!("Update JWT_SECRET in .env? [y/N]: ");
            std::io::Write::flush(&mut std::io::stdout())?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            if input.trim().to_lowercase() == "y" {
                // Read, update, write .env atomically
                let content = std::fs::read_to_string(&env_path)?;
                let new_content = content
                    .lines()
                    .map(|line| {
                        if line.starts_with("JWT_SECRET=") {
                            format!("JWT_SECRET={}", new_secret)
                        } else {
                            line.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                let temp_path = format!("{}.new", env_path.display());
                std::fs::write(&temp_path, new_content)?;
                std::fs::rename(&temp_path, &env_path)?;

                println!("✓ JWT_SECRET updated in .env");
                println!();
                println!("⚠️  RESTART REQUIRED:");
                println!("   botserver restart");
                println!();
                println!("All users must re-login after restart (refresh tokens invalid)");
                println!("Access tokens will expire naturally within 15 minutes");

                verify_rotation(component).await?;
            } else {
                println!("✗ Aborted");
                println!("Backup preserved at: {}", backup_path);
            }
        }
        _ => {
            eprintln!("Unknown component: {}", component);
            eprintln!("Valid components: tables, drive, cache, email, directory, encryption, jwt");
        }
    }

    Ok(())
}
