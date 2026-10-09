use super::*;

pub(super) fn bootstrap_state_path() -> std::path::PathBuf {
    let stack = get_stack_path();
    std::path::PathBuf::from(format!("{}/conf/directory/bootstrap-state.json", stack))
}
pub(super) fn load_bootstrap_state() -> Option<BootstrapState> {
    let path = bootstrap_state_path();
    if !path.exists() {
        return None;
    }
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).ok(),
        Err(e) => {
            warn!("Failed to read bootstrap state: {}", e);
            None
        }
    }
}
pub(super) fn save_bootstrap_state(state: &BootstrapState) {
    let path = bootstrap_state_path();
    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            log::error!("Failed to create bootstrap state directory: {}", e);
            return;
        }
    }
    match fs::write(&path, serde_json::to_string_pretty(state).unwrap_or_default()) {
        Ok(_) => info!("Bootstrap state saved to {}", path.display()),
        Err(e) => log::error!("Failed to save bootstrap state: {}", e),
    }
}
pub async fn check_and_bootstrap_admin(client: &ZitadelClient) -> Result<Option<BootstrapResult>> {
    info!("Checking if bootstrap is needed...");

    if let Some(state) = load_bootstrap_state() {
        info!(
            "Bootstrap already completed (admin: {}, org: {}). Skipping.",
            state.user_id,
            state.organization_id.as_deref().unwrap_or("none")
        );
        return Ok(None);
    }

    match client.list_users(10, 0).await {
        Ok(users) => {
            if !users.is_empty() {
                let has_admin = users.iter().any(|u| {
                    let username = u
                        .get("userName")
                        .or_else(|| u.get("username"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    let has_admin_role = u
                        .get("roles")
                        .and_then(|r| r.as_array())
                        .map(|roles| {
                            roles.iter().any(|r| {
                                r.as_str()
                                    .map(|s| s.to_lowercase().contains("admin"))
                                    .unwrap_or(false)
                            })
                        })
                        .unwrap_or(false);

                    username == ADMIN_USERNAME || has_admin_role
                });

                if has_admin {
                    info!("Admin user already exists, skipping bootstrap");
                    return Ok(None);
                }
            }
        }
        Err(e) => {
            info!(
                "Could not check existing users yet (may be first run): {}",
                e
            );
        }
    }

    info!("No admin user found, bootstrapping initial admin account...");

    let result = create_bootstrap_admin(client).await?;

    print_bootstrap_credentials(&result);

    Ok(Some(result))
}
pub async fn ensure_default_organization(client: &ZitadelClient) -> Result<String> {
    let orgs = client.list_organizations(100, 0).await?;
    if let Some(existing) = orgs.first() {
        let org_id = existing
            .get("id")
            .or_else(|| existing.get("organizationId"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("No organization ID found"))?;
        info!("Default organization already exists: {}", org_id);
        return Ok(org_id.to_string());
    }

    let org_id = create_default_organization(client).await?;
    info!("Created default organization: {}", org_id);
    Ok(org_id)
}
pub async fn ensure_admin_user(client: &ZitadelClient) -> Result<String> {
    let users = client.list_users(100, 0).await?;
    if let Some(existing) = users.iter().find(|u| {
        u.get("userName")
            .or_else(|| u.get("username"))
            .and_then(|v| v.as_str())
            .unwrap_or("") == ADMIN_USERNAME
    }) {
        let user_id = existing
            .get("id")
            .or_else(|| existing.get("userId"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("No user ID found"))?;
        info!("Admin user already exists: {}", user_id);
        return Ok(user_id.to_string());
    }

    let email = format!("{}@localhost", ADMIN_USERNAME);
    let initial_password = generate_secure_password();

    let user_id = client
        .create_user_with_password(&email, "System", "Administrator", Some(ADMIN_USERNAME), Some(&initial_password))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create admin user: {}", e))?;

    info!("Created admin user with ID: {}", user_id);

    if let Err(e) = client.set_user_password(&user_id, &initial_password, false).await {
        info!("Failed to set initial password via API (non-critical): {}. User may use password reset flow.", e);
    }

    Ok(user_id)
}
pub(super) fn generate_secure_password() -> String {
    let mut rng = rand::rng();

    let lowercase: Vec<char> = (b'a'..=b'z').map(|c| c as char).collect();
    let uppercase: Vec<char> = (b'A'..=b'Z').map(|c| c as char).collect();
    let digits: Vec<char> = (b'0'..=b'9').map(|c| c as char).collect();
    let special: Vec<char> = "!@#$%&*".chars().collect();

    let mut password = Vec::with_capacity(16);

    password.push(lowercase[rng.random_range(0..lowercase.len())]);
    password.push(uppercase[rng.random_range(0..uppercase.len())]);
    password.push(digits[rng.random_range(0..digits.len())]);
    password.push(special[rng.random_range(0..special.len())]);

    let all_chars: Vec<char> = lowercase
        .iter()
        .chain(uppercase.iter())
        .chain(digits.iter())
        .chain(special.iter())
        .copied()
        .collect();

    for _ in 0..12 {
        password.push(all_chars[rng.random_range(0..all_chars.len())]);
    }

    for i in (1..password.len()).rev() {
        let j = rng.random_range(0..=i);
        password.swap(i, j);
    }

    password.into_iter().collect()
}
pub(super) async fn create_bootstrap_admin(client: &ZitadelClient) -> Result<BootstrapResult> {
    let email = format!("{}@localhost", ADMIN_USERNAME);
    let initial_password = generate_secure_password();

    let user_id = client
        .create_user_with_password(&email, "System", "Administrator", Some(ADMIN_USERNAME), Some(&initial_password))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create admin user: {}", e))?;

    info!("Created admin user with ID: {}", user_id);

    if let Err(e) = client.set_user_password(&user_id, &initial_password, false).await {
        info!("Failed to set initial password via API (non-critical): {}. User may need to use password reset flow.", e);
    } else {
        info!("Initial password set for admin user");
    }

    let org_id = match create_default_organization(client).await {
        Ok(id) => {
            info!("Created default organization with ID: {}", id);

            let admin_roles = vec![
                "admin".to_string(),
                "org_owner".to_string(),
                "user_manager".to_string(),
            ];
            if let Err(e) = client.add_org_member(&id, &user_id, admin_roles).await {
                warn!("Failed to add admin to organization: {}", e);
            }

            Some(id)
        }
        Err(e) => {
            info!("Failed to create default organization (non-critical): {}. Will retry on next restart.", e);
            None
        }
    };

    // Create a PAT token for the admin user for API access
    // Note: This requires OAuth client credentials. If not available, PAT creation
    // will be skipped, but admin user can still login with username/password.
    info!("Creating admin PAT token for API access...");
    match client.create_pat(&user_id, "Admin API Token", None).await {
        Ok(pat_token) => {
            info!("Admin PAT token created successfully");
            save_admin_pat_token(&pat_token);
        }
        Err(e) => {
            info!("Admin PAT token creation skipped (non-critical): {}. User can login with username/password.", e);
        }
    }

    let base_url = client.api_url();
    let setup_url = format!("{}/ui/login", base_url);

    let result = BootstrapResult {
        user_id: user_id.clone(),
        organization_id: org_id.clone(),
        username: ADMIN_USERNAME.to_string(),
        email: email.clone(),
        initial_password: initial_password.clone(),
        setup_url: setup_url.clone(),
    };

    save_setup_credentials(&result);

    create_password_change_reminder(&user_id);

    let bootstrap_state = BootstrapState {
        user_id: user_id.clone(),
        organization_id: org_id.clone(),
        username: ADMIN_USERNAME.to_string(),
        email: email.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    save_bootstrap_state(&bootstrap_state);

    Ok(result)
}
pub(super) fn create_password_change_reminder(user_id: &str) {
    let reminder_file = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join(".gb-admin-reminders.json");

    let reminder = serde_json::json!({
        "id": Uuid::new_v4().to_string(),
        "type": "security",
        "priority": "high",
        "title": "Change initial admin password",
        "description": "The admin account was created with an auto-generated password. Please change it to a secure password of your choice.",
        "user_id": user_id,
        "created_at": chrono::Utc::now().to_rfc3339(),
        "status": "pending",
        "action_url": "/settings/security"
    });

    let reminders = if reminder_file.exists() {
        let content = fs::read_to_string(&reminder_file).unwrap_or_default();
        let mut arr: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap_or_default();
        arr.push(reminder);
        arr
    } else {
        vec![reminder]
    };

    if let Err(e) = fs::write(&reminder_file, serde_json::to_string_pretty(&reminders).unwrap_or_default()) {
        warn!("Failed to save password change reminder: {}", e);
    } else {
        info!("Created security reminder: Change initial admin password");
    }
}
