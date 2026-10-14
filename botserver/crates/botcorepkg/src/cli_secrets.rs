use crate::cli::{generate_access_key, generate_password, generate_secret_key};
use botcoresecrets::{SecretPaths, SecretsManager};
use botlib::security::SafeCommand;
use anyhow::Result;
use std::collections::HashMap;

fn confirm_action(prompt: &str) -> bool {
    print!("{prompt}");
    let _ = std::io::Write::flush(&mut std::io::stdout());
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
    input.trim().to_lowercase() == "y"
}

fn confirm_exact(prompt: &str, expected: &str) -> bool {
    print!("{prompt}");
    let _ = std::io::Write::flush(&mut std::io::stdout());
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
    input.trim() == expected
}

pub async fn rotate_secret(component: &str) -> Result<()> {
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

            println!("⚠️ WARNING: You must update PostgreSQL with the new password!");
            println!();
            println!("Run this SQL command:");
            let default_username = "postgres".to_string();
            println!(
                " ALTER USER {} WITH PASSWORD '[REDACTED — see Vault secret/gbo/tables]';",
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

            if confirm_action("Save to Vault? [y/N]: ") {
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

            println!("⚠️ WARNING: You must update MinIO with the new credentials!");
            println!();
            println!("Run these commands:");
            println!(
                " mc admin user add myminio {} [REDACTED — see Vault secret/gbo/drive]",
                new_accesskey
            );
            println!(
                " mc admin policy attach myminio readwrite --user {}",
                new_accesskey
            );
            println!();
            println!("New access key: {}", new_accesskey);
            println!(
                "New secret key: {}...",
                new_secret.chars().take(8).collect::<String>()
            );

            if confirm_action("Save to Vault? [y/N]: ") {
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

            println!("⚠️ WARNING: You must update Valkey/Redis with the new password!");
            println!();
            println!("Run this command:");
            println!(" redis-cli CONFIG SET requirepass '[REDACTED — see Vault secret/gbo/cache]'");
            println!();
            println!(
                "New password: {}...",
                new_password.chars().take(4).collect::<String>()
            );

            if confirm_action("Save to Vault? [y/N]: ") {
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

            println!("⚠️ WARNING: You must update the mail server with the new password!");
            println!();
            println!(
                "New password: {}...",
                new_password.chars().take(4).collect::<String>()
            );

            if confirm_action("Save to Vault? [y/N]: ") {
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

            println!("⚠️ CRITICAL WARNING: Rotating encryption key will make existing encrypted data unreadable!");
            println!("⚠️ Make sure to re-encrypt all data with the new key!");
            println!();
            println!(
                "New master key: {}...",
                new_key.chars().take(8).collect::<String>()
            );

            if confirm_exact("Are you ABSOLUTELY sure? Type 'ROTATE' to confirm: ", "ROTATE") {
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

            println!("⚠️ WARNING: You must update Zitadel with the new client secret!");
            println!();
            println!(
                "New client secret: {}...",
                new_secret.chars().take(8).collect::<String>()
            );

            if confirm_action("Save to Vault? [y/N]: ") {
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

            println!("⚠️ JWT SECRET ROTATION");
            println!();
            println!("Current: JWT_SECRET in .env file");
            println!("Impact: ALL refresh tokens will become invalid immediately");
            println!("Access tokens (15 min) will expire naturally");
            println!();

            if !env_path.exists() {
                println!("✗ .env file not found at: {}", env_path.display());
                return Ok(());
            }

            let env_content = std::fs::read_to_string(&env_path)?;
            let current_jwt = env_content
                .lines()
                .find(|line| line.starts_with("JWT_SECRET="))
                .unwrap_or("JWT_SECRET=(not set)");

            println!("Current: {}", current_jwt.chars().take(40).collect::<String>());
            println!("New: {}... (64 chars)", new_secret.chars().take(8).collect::<String>());
            println!();

            let backup_path = format!("{}.backup.{}", env_path.display(), chrono::Utc::now().timestamp());
            std::fs::copy(&env_path, &backup_path)?;
            println!("✓ Backup created: {}", backup_path);
            println!();

            if confirm_action("Update JWT_SECRET in .env? [y/N]: ") {
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
                println!("⚠️ RESTART REQUIRED:");
                println!(" botserver restart");
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

pub async fn rotate_all_secrets() -> Result<()> {
    println!("🔐 ROTATING ALL SECRETS");
    println!("========================");
    println!();
    println!("⚠️ CRITICAL WARNING!");
    println!("This will generate new credentials for ALL components.");
    println!("You MUST update each service manually after rotation.");
    println!();

    if !confirm_exact("Type 'ROTATE ALL' to continue: ", "ROTATE ALL") {
        println!("✗ Aborted");
        return Ok(());
    }

    let manager = SecretsManager::get()?.clone();
    if !manager.is_enabled() {
        return Err(anyhow::anyhow!("Vault not configured"));
    }

    println!();
    println!("Generating new credentials...");
    println!();

    let tables_password = generate_password(32);
    let mut tables = manager
        .get_secret(SecretPaths::TABLES)
        .await
        .unwrap_or_default();
    tables.insert("password".to_string(), tables_password.clone());
    manager
        .put_secret(SecretPaths::TABLES, tables.clone())
        .await?;
    let default_username = "postgres".to_string();
    println!(
        "✓ tables: ALTER USER {} WITH PASSWORD '[REDACTED — see Vault secret/gbo/tables]';",
        tables.get("username").unwrap_or(&default_username)
    );

    let drive_accesskey = generate_access_key();
    let drive_secret = generate_secret_key();
    let mut drive = manager
        .get_secret(SecretPaths::DRIVE)
        .await
        .unwrap_or_default();
    drive.insert("accesskey".to_string(), drive_accesskey.clone());
    drive.insert("secret".to_string(), drive_secret.clone());
    manager.put_secret(SecretPaths::DRIVE, drive).await?;
    println!(
        "✓ drive: mc admin user add myminio {} [REDACTED]",
        drive_accesskey
    );

    let cache_password = generate_password(32);
    let mut cache: HashMap<String, String> = HashMap::new();
    cache.insert("password".to_string(), cache_password.clone());
    manager.put_secret(SecretPaths::CACHE, cache).await?;
    println!("✓ cache: redis-cli CONFIG SET requirepass '[REDACTED]'");

    let email_password = generate_password(24);
    let mut email = manager
        .get_secret(SecretPaths::EMAIL)
        .await
        .unwrap_or_default();
    email.insert("password".to_string(), email_password.clone());
    manager.put_secret(SecretPaths::EMAIL, email).await?;
    println!("✓ email: new password set ({} chars)", email_password.len());

    let directory_secret = generate_password(48);
    let mut directory = manager
        .get_secret(SecretPaths::DIRECTORY)
        .await
        .unwrap_or_default();
    directory.insert("client_secret".to_string(), directory_secret.clone());
    manager
        .put_secret(SecretPaths::DIRECTORY, directory)
        .await?;
    println!(
        "✓ directory: new client_secret = {}...",
        directory_secret.chars().take(12).collect::<String>()
    );

    println!();
    println!("========================");
    println!("✓ All secrets rotated and saved to Vault");
    println!();
    println!("⚠️ IMPORTANT: Run the commands above to update each service!");
    println!("⚠️ Then restart botserver: botserver restart");

    Ok(())
}

pub async fn verify_rotation(component: &str) -> Result<()> {
    println!();
    println!("Verifying {}...", component);

    match component {
        "tables" => {
            let manager = SecretsManager::get()?.clone();
            let secrets = manager.get_secret(SecretPaths::TABLES).await?;

            let host = secrets.get("host").cloned().unwrap_or_else(|| "localhost".to_string());
            let port = secrets.get("port").cloned().unwrap_or_else(|| "5432".to_string());
            let user = secrets.get("username").cloned().unwrap_or_else(|| "postgres".to_string());
            let pass = secrets.get("password").cloned().unwrap_or_default();
            let db = secrets.get("database").cloned().unwrap_or_else(|| "postgres".to_string());

            println!(" Testing connection to {}@{}:{}...", user, host, port);

            let mut cmd = SafeCommand::new("psql").map_err(|e| anyhow::anyhow!("{}", e))?;
            cmd = cmd.args(&[
                "-h", &host,
                "-p", &port,
                "-U", &user,
                "-d", &db,
                "-c", "SELECT 1;",
                "-t", "-q"
            ]).map_err(|e| anyhow::anyhow!("{}", e))?;
            cmd = cmd.env("PGPASSWORD", &pass).map_err(|e| anyhow::anyhow!("{}", e))?;
            let result = cmd.execute();

            match result {
                Ok(output) if output.status.success() => {
                    println!("✓ Database connection successful");
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    println!("✗ Database connection FAILED");
                    println!(" Error: {}", stderr.trim());
                    println!(" Hint: Run the SQL command provided by rotate-secret");
                }
                Err(_) => {
                    println!("⊘ Verification skipped (psql not available)");
                    println!(" Hint: Manually test with: psql -h {} -U {} -d {} -c 'SELECT 1'", host, user, db);
                }
            }
        }
        "jwt" => {
            println!(" Testing health endpoint...");

            let health_urls = vec![
                "/health",
                "/health",
                "/health",
            ];

            let mut success = false;
            for url in health_urls {
                match reqwest::get(url).await {
                    Ok(resp) if resp.status().is_success() => {
                        println!("✓ Service healthy at {}", url);
                        success = true;
                        break;
                    }
                    Ok(_) => {
                        continue;
                    }
                    Err(_) => {
                        continue;
                    }
                }
            }

            if !success {
                println!("⊘ Health endpoint not reachable");
                println!(" Hint: Restart botserver with: botserver restart");
                println!(" Then manually verify service is responding");
            }
        }
        _ => {
            println!("⊘ No automated verification available for {}", component);
            println!(" Hint: Manually verify the service is working after rotation");
        }
    }

    Ok(())
}
