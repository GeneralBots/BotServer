//! #1386 — Shared per-project database naming for Vibe projects.
//!
//! Every Vibe project owns TWO dedicated PostgreSQL databases so an
//! experiment on the dev twin can never touch production data:
//!
//! | env        | database name                |
//! |------------|------------------------------|
//! | production | `app_{branch}_{project}`     |
//! | test/dev   | `app_{branch}_{project}_dev` |
//!
//! Naming mirrors `botcore::bot_database::generate_database_name`
//! (`bot_{branch}_{bot}`) so bots and apps share one convention. The naming
//! lives in `botcore` (not `botvibe`) because the Database app backend
//! (`botdatabase`) must resolve the same names to contextualize its schema
//! browser on the selected project.
//!
//! Security: database names are built exclusively from alphanumeric +
//! underscore fragments and validated before reaching SQL, so identifier
//! injection is impossible. Connection URLs derived from these names are
//! injected as environment variables and never logged.

use uuid::Uuid;

/// Longest legal PostgreSQL identifier (NAMEDATALEN - 1).
pub const MAX_DB_NAME_LEN: usize = 63;

/// Sanitize a path component into a legal database-name fragment.
/// Mirrors `vm_lifecycle::sanitize_part` but keeps underscores (DB names).
fn sanitize_db_part(s: &str) -> String {
    let mut out = String::new();
    let mut last_underscore = false;
    for ch in s.chars().take(32) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_underscore = false;
        } else if (ch == '-' || ch == '_' || ch.is_whitespace()) && !out.is_empty() && !last_underscore {
            out.push('_');
            last_underscore = true;
        }
    }
    if out.is_empty() {
        "app".to_string()
    } else {
        out
    }
}

/// Branch fragment for database names: the first UUID group is stable and
/// short (`alm_org` convention), keeping generated names well under 63 chars.
fn branch_part(branch_id: Uuid) -> String {
    sanitize_db_part(branch_id.to_string().split('-').next().unwrap_or("default"))
}

/// `true` when `env` denotes the production environment; every other
/// spelling (`dev`, `test`, `development`, `staging`) resolves to the dev twin.
pub fn is_production_env(env: &str) -> bool {
    matches!(env.to_ascii_lowercase().as_str(), "production" | "prod")
}

/// Build the database name for a project in one environment.
///
/// - `production` → `app_{branch}_{name}`
/// - anything else (test/development/dev/staging) → `app_{branch}_{name}_dev`
pub fn project_database_name(branch_id: Uuid, project_name: &str, env: &str) -> String {
    let suffix = if is_production_env(env) { "" } else { "_dev" };
    let max_base = MAX_DB_NAME_LEN - suffix.len();
    let base = format!("app_{}_{}", branch_part(branch_id), sanitize_db_part(project_name));
    let base = if base.len() > max_base {
        base[..max_base].to_string()
    } else {
        base
    };
    format!("{base}{suffix}")
}

/// Validate a database name before it reaches SQL. Rejects anything that is
/// not `[a-z0-9_]` (the only characters we ever generate) or too long.
pub fn validate_db_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > MAX_DB_NAME_LEN {
        return Err(format!("invalid database name length: {} chars", name.len()));
    }
    if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
        return Err(format!("invalid characters in database name '{name}'"));
    }
    Ok(())
}

/// #1447 S1 — defense-in-depth identifier guard for SQL interpolation.
/// PostgreSQL cannot `$1`-bind identifiers, so every `format!` that places a
/// database name into DDL/`pg_database` text must pass this strict ASCII
/// allowlist check AT the interpolation site — even when the caller already
/// ran `validate_db_name` (a future caller passing raw `branch/project`
/// without it stays injectable otherwise).
pub fn assert_safe_db_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 63 {
        return Err(format!("unsafe database name length: {} chars", name.len()));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(format!("unsafe characters in database name '{name}'"));
    }
    Ok(())
}

/// Connection URL for a project database, derived from the botserver main
/// URL (credentials/host) with the database segment replaced. Callers must
/// never log the resulting URL.
pub fn database_url_for(db_name: &str) -> String {
    let base = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/botserver".to_string());
    match base.rfind('/') {
        Some(pos) => {
            let query = base[pos..].split_once('?').map(|(_, q)| format!("?{q}")).unwrap_or_default();
            format!("{}/{db_name}{query}", &base[..pos])
        }
        None => format!("{base}/{db_name}"),
    }
}

#[cfg(test)]
mod project_db_name_tests {
    use super::*;

    const BRANCH: Uuid = Uuid::from_u128(0x1ddcdef6_0000_0000_0000_000000000000);

    #[test]
    fn test_project_db_name_production_has_no_suffix() {
        let db = project_database_name(BRANCH, "my-store", "production");
        assert_eq!(db, "app_1ddcdef6_my_store");
        assert!(validate_db_name(&db).is_ok());
    }

    #[test]
    fn test_project_db_name_dev_twin_gets_suffix() {
        let db = project_database_name(BRANCH, "my-store", "dev");
        assert_eq!(db, "app_1ddcdef6_my_store_dev");
        assert!(validate_db_name(&db).is_ok());
    }

    #[test]
    fn test_project_db_name_dev_aliases_converge() {
        assert_eq!(
            project_database_name(BRANCH, "x", "development"),
            project_database_name(BRANCH, "x", "dev")
        );
        assert_eq!(
            project_database_name(BRANCH, "x", "test"),
            project_database_name(BRANCH, "x", "testing")
        );
    }

    #[test]
    fn test_project_db_name_rejects_injection() {
        let db = project_database_name(BRANCH, "x'; DROP DATABASE users;--", "production");
        assert!(validate_db_name(&db).is_ok(), "generated name must be sanitized, got {db}");
        assert!(!db.contains('\'') && !db.contains(';') && !db.contains(' '));
    }

    #[test]
    fn test_project_db_name_truncates_long_names() {
        let long = "a".repeat(100);
        let db = project_database_name(BRANCH, &long, "test");
        assert!(db.len() <= MAX_DB_NAME_LEN);
        assert!(db.ends_with("_dev"));
        assert!(validate_db_name(&db).is_ok());
    }
}
