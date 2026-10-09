use super::*;

pub(super) async fn rotate_all_secrets() -> Result<()> {
    println!("🔐 ROTATING ALL SECRETS");
    println!("========================");
    println!();
    println!("⚠️  CRITICAL WARNING!");
    println!("This will generate new credentials for ALL components.");
    println!("You MUST update each service manually after rotation.");
    println!();
    print!("Type 'ROTATE ALL' to continue: ");
    std::io::Write::flush(&mut std::io::stdout())?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if input.trim() != "ROTATE ALL" {
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
        &directory_secret.chars().take(12).collect::<String>()
    );

    println!();
    println!("========================");
    println!("✓ All secrets rotated and saved to Vault");
    println!();
    println!("⚠️  IMPORTANT: Run the commands above to update each service!");
    println!("⚠️  Then restart botserver: botserver restart");

    Ok(())
}
pub(super) async fn verify_rotation(component: &str) -> Result<()> {
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

            println!("  Testing connection to {}@{}:{}...", user, host, port);

            // Use psql to test connection
            let mut cmd = SafeCommand::new("psql").map_err(|e| anyhow::anyhow!("{}", e))?;
            cmd = cmd.args(&[
                "-h", &host,
                "-p", &port,
                "-U", &user,
                "-d", &db,
                "-c", "SELECT 1;",
                "-t", "-q"  // Tuples only, quiet mode
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
                    println!("  Error: {}", stderr.trim());
                    println!("  Hint: Run the SQL command provided by rotate-secret");
                }
                Err(_e) => {
                    println!("⊘ Verification skipped (psql not available)");
                    println!("  Hint: Manually test with: psql -h {} -U {} -d {} -c 'SELECT 1'", host, user, db);
                }
            }
        }
        "jwt" => {
            println!("  Testing health endpoint...");

            // Try to determine the health endpoint
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
                    Ok(_resp) => {
                        // Try next URL
                        continue;
                    }
                    Err(_e) => {
                        // Try next URL
                        continue;
                    }
                }
            }

            if !success {
                println!("⊘ Health endpoint not reachable");
                println!("  Hint: Restart botserver with: botserver restart");
                println!("  Then manually verify service is responding");
            }
        }
        _ => {
            println!("⊘ No automated verification available for {}", component);
            println!("  Hint: Manually verify the service is working after rotation");
        }
    }

    Ok(())
}
pub(super) async fn vault_health() -> Result<()> {
    let manager = SecretsManager::get()?.clone();

    if !manager.is_enabled() {
        println!("x Vault not configured");
        println!("  Set VAULT_ADDR and VAULT_TOKEN environment variables");
        return Ok(());
    }

    match manager.health_check().await {
        Ok(true) => {
            println!("* Vault is healthy");
            println!("  Address: {}", env::var("VAULT_ADDR").unwrap_or_default());
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
