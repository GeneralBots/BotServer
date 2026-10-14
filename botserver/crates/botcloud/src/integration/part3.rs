use super::*;

/// Recursively copy a directory from src to dst using std::fs.
pub(super) fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    // Containment barrier: every entry must canonicalize inside `src`, so a
    // crafted or symlinked entry name cannot escape the source directory.
    let canon_src = src.canonicalize()?;
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if let Ok(canon_entry) = src_path.canonicalize() {
            if !canon_entry.starts_with(&canon_src) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "copy_dir_all: entry escapes source directory",
                ));
            }
        }
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
/// Resolves the MinIO client binary. Bare `mc` is dangerous: GNU Midnight
/// Commander installs as /usr/bin/mc on some images, and invoking it instead
/// of the MinIO client silently broke bucket provisioning at signup (the
/// wrong binary printed its help text and no `.gborg` bucket was created).
/// Preference order: the configured path, then the standard stack locations.
pub fn resolve_mc_binary(configured: &str) -> String {
    for cand in [configured, "/opt/gbo/bin/mc", "/tmp/mc", "/usr/local/bin/mc"] {
        if Path::new(cand).exists() {
            return cand.to_string();
        }
    }
    configured.to_string()
}
/// Creates MinIO `.gborg` bucket for the org and uploads bot files.
/// If a `template` path is provided (e.g. "core/default.gbai"), copies the
/// corresponding directory from `config.templates_dir` into the bucket and
/// renames internal directories to match the new bot name.
/// Otherwise creates a generic `start.bas` with the bot name.
/// Structure: {org_slug}.gborg / {bot_slug}.gbai / {bot_slug}.gbdialog / ...
/// Requires mc alias to be pre-configured (e.g., via Vault credentials).
pub fn create_bot_bucket(
    config: &SaasConfig, org_slug: &str, bot_slug: &str, bot_name: &str,
    template: Option<&str>,
) -> Result<(), String> {
    let alias = &config.mc_alias;
    let mc = resolve_mc_binary(&config.mc_path);
    let org_bucket = format!("{alias}/{org_slug}.gborg");

    let mb = SafeCommand::new(&mc)
        .and_then(|c| c.arg("mb"))
        .and_then(|c| c.arg(&org_bucket))
        .and_then(|c| c.arg("--ignore-existing"))
        .map_err(|e| format!("mc mb guard: {e}"))?
        .execute()
        .map_err(|e| format!("mc mb failed: {e}"))?;

    if !mb.status.success() {
        let stderr = String::from_utf8_lossy(&mb.stderr);
        return Err(format!("Failed to create bucket {org_bucket}: {stderr}"));
    }

    if let Some(template_path) = template {
        let templates_dir = &config.templates_dir;
        // Path-injection guard: the template path arrives from the signup
        // request body. Only relative single-component segments under the
        // configured templates dir are allowed — no `..`, no absolute paths,
        // no separators beyond the single level of `sub/name.gbai`.
        if template_path.contains('\\')
            || template_path.contains('\0')
            || template_path.starts_with('/')
            || template_path.split('/').any(|seg| seg == ".." || seg.is_empty() || seg == ".")
        {
            return Err(format!(
                "invalid template path: {template_path} (must be relative segments under the templates dir)"
            ));
        }
        let src = Path::new(templates_dir).join(template_path);
        // Containment barrier: canonicalize and verify the resolved template
        // source stays inside the configured templates dir (symlink-aware).
        let canon_src = src
            .canonicalize()
            .map_err(|e| format!("template path {}: {e}", src.display()))?;
        let canon_base = Path::new(templates_dir)
            .canonicalize()
            .map_err(|e| format!("templates dir {}: {e}", templates_dir))?;
        if !canon_src.starts_with(&canon_base) {
            return Err(format!(
                "template path {} escapes templates dir {}",
                canon_src.display(),
                canon_base.display()
            ));
        }
        if src.is_dir() {
            let tmpdir = format!("/tmp/gbtemplate_{bot_slug}");
            let _ = std::fs::remove_dir_all(&tmpdir);
            if let Err(e) = copy_dir_all(&src, Path::new(&tmpdir)) {
                tracing::warn!("Failed to copy template {template_path}: {e}");
            } else {
                // Rename internal directories from original name to bot_slug
                let template_name = template_path
                    .split('/')
                    .next_back()
                    .and_then(|s| s.strip_suffix(".gbai"))
                    .unwrap_or("default");
                for ext in &["gbdialog", "gbot", "gbkb"] {
                    let old = format!("{tmpdir}/{template_name}.{ext}");
                    let new = format!("{tmpdir}/{bot_slug}.{ext}");
                    if Path::new(&old).exists() {
                        let _ = std::fs::rename(&old, &new);
                    }
                }
                // Upload recursively via mc cp
                let cp = SafeCommand::new(&mc)
                    .and_then(|c| c.arg("cp"))
                    .and_then(|c| c.arg("--recursive"))
                    .and_then(|c| c.arg(&tmpdir))
                    .and_then(|c| c.arg(format!("{org_bucket}/{bot_slug}.gbai/")))
                    .map_err(|e| format!("mc cp guard: {e}"))?
                    .execute()
                    .map_err(|e| format!("mc cp failed: {e}"))?;
                let _ = std::fs::remove_dir_all(&tmpdir);
                if cp.status.success() {
                    return Ok(());
                }
                let stderr = String::from_utf8_lossy(&cp.stderr);
                tracing::warn!("mc cp template failed, falling back to generic: {stderr}");
            }
        } else {
            tracing::warn!("Template {template_path} not found at {templates_dir}, using generic");
        }
    }

    // Fallback: create generic start.bas
    let start_bas_content = format!("TALK \"Olá! Sou o {bot_name}. Como posso ajudar?\"");
    let remote_path = format!("{org_bucket}/{bot_slug}.gbai/{bot_slug}.gbdialog/start.bas");

    let tmpfile = format!("/tmp/start_{bot_slug}.bas");
    let mut f = std::fs::File::create(&tmpfile)
        .map_err(|e| format!("Failed to create temp file: {e}"))?;
    f.write_all(start_bas_content.as_bytes())
        .map_err(|e| format!("Failed to write temp file: {e}"))?;
    drop(f);

    let cp = SafeCommand::new(&mc)
        .and_then(|c| c.arg("cp"))
        .and_then(|c| c.arg(&tmpfile))
        .and_then(|c| c.arg(&remote_path))
        .map_err(|e| format!("mc cp guard: {e}"))?
        .execute()
        .map_err(|e| format!("mc cp failed: {e}"))?;

    let _ = std::fs::remove_file(&tmpfile);

    if !cp.status.success() {
        let stderr = String::from_utf8_lossy(&cp.stderr);
        return Err(format!("Failed to upload start.bas to {remote_path}: {stderr}"));
    }

    Ok(())
}
/// Creates a trial subscription in billing_recurring for the given plan and trial days.
pub fn create_trial_subscription(
    pool: &DbPool,
    branch_id: Uuid,
    customer_name: &str,
    customer_email: &str,
    plan: &str,
    trial_days: i32,
) -> Result<Uuid, String> {
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;
    let sub_id = Uuid::new_v4();
    let now = Utc::now();
    let trial_end = now.date_naive() + Duration::days(trial_days as i64);

    diesel::sql_query(
        r#"INSERT INTO billing_recurring
           (id, branch_id, customer_name, customer_email, status, frequency, interval_count,
            amount, currency, description, next_invoice_date, start_date, last_invoice_id,
            invoices_generated, created_at, updated_at)
           VALUES ($1, $2, $3, $4, 'trialing', 'monthly', 1, 0.0, 'USD',
                   $5, $6, $7, NULL, 0, $8, $8)"#,
    )
    .bind::<DieselUuid, _>(sub_id)
    .bind::<DieselUuid, _>(branch_id)
    .bind::<Text, _>(customer_name)
    .bind::<Nullable<Text>, _>(Some(customer_email.to_string()))
    .bind::<Nullable<Text>, _>(Some(format!("{plan} - {trial_days} Day Trial")))
    .bind::<Date, _>(trial_end)
    .bind::<Date, _>(now.date_naive())
    .bind::<Timestamptz, _>(now)
    .execute(&mut conn)
    .map_err(|e| format!("Insert trial subscription: {e}"))?;

    Ok(sub_id)
}
/// Creates an active free subscription in billing_recurring.
/// No trial, no amount — starts immediately as 'active' with $0.00.
pub fn create_free_subscription(
    pool: &DbPool,
    branch_id: Uuid,
    customer_name: &str,
    customer_email: &str,
) -> Result<Uuid, String> {
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;
    let sub_id = Uuid::new_v4();
    let now = Utc::now();

    diesel::sql_query(
        r#"INSERT INTO billing_recurring
           (id, branch_id, customer_name, customer_email, status, frequency, interval_count,
            amount, currency, description, next_invoice_date, start_date, last_invoice_id,
            invoices_generated, created_at, updated_at)
           VALUES ($1, $2, $3, $4, 'active', 'monthly', 1, 0.0, 'USD',
                   'Free Plan', $7, $5, NULL, 0, $6, $6)"#,
    )
    .bind::<DieselUuid, _>(sub_id)
    .bind::<DieselUuid, _>(branch_id)
    .bind::<Text, _>(customer_name)
    .bind::<Nullable<Text>, _>(Some(customer_email.to_string()))
    .bind::<Date, _>(now.date_naive())
    .bind::<Date, _>(now.date_naive())
    .bind::<Timestamptz, _>(now)
    .execute(&mut conn)
    .map_err(|e| format!("Insert free subscription: {e}"))?;

    Ok(sub_id)
}
/// Creates a default cloud workspace for the organization.
pub fn create_cloud_workspace(pool: &DbPool, branch_id: Uuid, name: &str) -> Result<Uuid, String> {
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;
    let id = Uuid::new_v4();
    let now = Utc::now();

    diesel::sql_query(
        r#"INSERT INTO cloud_workspaces (id, branch_id, name, description, icon, created_at, updated_at)
           VALUES ($1, $2, $3, $4, 'default', $5, $5)"#,
    )
    .bind::<DieselUuid, _>(id)
    .bind::<DieselUuid, _>(branch_id)
    .bind::<Text, _>(name)
    .bind::<Nullable<Text>, _>(Some(format!("Default workspace for {}", name)))
    .bind::<Timestamptz, _>(now)
    .execute(&mut conn)
    .map_err(|e| format!("Insert cloud_workspace: {e}"))?;

    Ok(id)
}
