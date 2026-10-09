use super::*;

pub(super) async fn create_default_organization(client: &ZitadelClient) -> Result<String> {
    let url = format!("{}/v2/organizations", client.api_url());

    let body = serde_json::json!({
        "name": DEFAULT_ORG_NAME
    });

    let response = client
        .http_post(url).await?
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create organization: {}", e))?;

    if response.status().is_success() {
        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;

        let org_id = data
            .get("organizationId")
            .or_else(|| data.get("id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("No organization ID in response"))?
            .to_string();

        Ok(org_id)
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(anyhow::anyhow!(
            "Failed to create organization: {}",
            error_text
        ))
    }
}
pub(super) fn save_setup_credentials(result: &BootstrapResult) {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let creds_path = format!("{}/.gb-setup-credentials", home);

    let content = format!(
        r#"# General Bots Initial Setup Credentials
# Created: {}

╔════════════════════════════════════════════════════════════╗
║           🔐 ADMIN LOGIN - READY TO USE                    ║
╠════════════════════════════════════════════════════════════╣
║                                                            ║
║  Username: {:<46}║
║  Password: {:<46}║
║  Email:    {:<46}║
║                                                            ║
║  🌐 LOGIN NOW: /suite/login           ║
║                                                            ║
╚════════════════════════════════════════════════════════════╝

✅ Login directly - no password change required on first access
⚠️  SECURITY: Change this password after login (Settings > Security)
🗑️  Delete this file after saving your new password
"#,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        result.username,
        result.initial_password,
        result.email
    );

    match fs::write(&creds_path, &content) {
        Ok(_) => {
            #[cfg(unix)]
            {
                if let Err(e) = botlib::os::fs::get_permissions_manager().set_readonly_owner(creds_path.as_ref()) {
                    warn!("Failed to set file permissions: {}", e);
                }
            }
            info!("Setup credentials saved to: {}", creds_path);
        }
        Err(e) => {
            log::error!("Failed to save setup credentials: {}", e);
        }
    }
}
pub(super) fn save_admin_pat_token(pat_token: &str) {
    // Create directory if it doesn't exist
    let stack = get_stack_path();
    let pat_dir = std::path::PathBuf::from(format!("{}/conf/directory", stack));
    if let Err(e) = fs::create_dir_all(&pat_dir) {
        log::error!("Failed to create PAT directory: {}", e);
        return;
    }

    let pat_path = pat_dir.join("admin-pat.txt");

    match fs::write(&pat_path, pat_token) {
        Ok(_) => {
            #[cfg(unix)]
            {
                if let Err(e) = botlib::os::fs::get_permissions_manager().set_readonly_owner(&pat_path) {
                    warn!("Failed to set PAT file permissions: {}", e);
                }
            }
            info!("Admin PAT token saved to: {}", pat_path.display());
        }
        Err(e) => {
            log::error!("Failed to save admin PAT token: {}", e);
        }
    }
}
pub(super) fn print_bootstrap_credentials(result: &BootstrapResult) {
    let separator = "═".repeat(60);

    println!();
    println!("╔{}╗", separator);
    println!("║{:^60}║", "");
    println!("║{:^60}║", "🤖 GENERAL BOTS - INITIAL SETUP");
    println!("║{:^60}║", "");
    println!("╠{}╣", separator);
    println!("║{:^60}║", "");
    println!("║  {:56}║", "Administrator account created!");
    println!("║{:^60}║", "");
    println!("╠{}╣", separator);
    println!("║{:^60}║", "");
    println!("║{:^60}║", "🔐 LOGIN CREDENTIALS (READY TO USE):");
    println!("║{:^60}║", "");
    println!("║  {:<58}║", format!("Username: {}", result.username));
    println!("║  {:<58}║", format!("Password: [set — {} chars]", result.initial_password.len()));
    println!("║  {:<58}║", format!("Email:    {}", result.email));
    println!("║{:^60}║", "");

    if let Some(ref org_id) = result.organization_id {
        println!(
            "║  {:<58}║",
            format!("Organization: {} ({})", DEFAULT_ORG_NAME, &org_id[..8.min(org_id.len())])
        );
        println!("║{:^60}║", "");
    }

    println!("╠{}╣", separator);
    println!("║{:^60}║", "");
    println!("║  {:56}║", "🌐 LOGIN NOW:");
    println!("║{:^60}║", "");
    println!("║  {:56}║", "/suite/login");
    println!("║{:^60}║", "");
    println!("╠{}╣", separator);
    println!("║{:^60}║", "");
    println!("║  ✅ {:<54}║", "Login directly - no password change required");
    println!("║{:^60}║", "");
    println!("║  {:<56}║", "⚠️  Change password after login (Settings > Security)");
    println!("║  {:<56}║", "📁 Credentials saved to: ~/.gb-setup-credentials");
    println!("║{:^60}║", "");
    println!("╚{}╝", separator);
    println!();

    info!(
        "Bootstrap complete: admin user '{}' created - ready for direct login",
        result.username
    );
}
pub fn print_existing_admin_notice() {
    println!();
    println!("ℹ️  Admin user already exists. Skipping bootstrap.");
    println!("   If you forgot your password, use Zitadel console to reset it.");
    println!();
}
