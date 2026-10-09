use super::*;

/// Project-name → bot slug (single source; the REST path reuses this).
pub fn bot_slug(name: &str) -> String {
    name.to_lowercase()
        .replace(' ', "-")
        .replace('_', "-")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect()
}

/// ALM (Forgejo) org for a branch **slug** — the readable "org-branch
/// organization" of the reform (#1503): every project of the branch lives in
/// this one org, repos named after the bot/web/app.
pub fn alm_org_from_slug(branch_slug: &str) -> String {
    let cleaned: String = branch_slug
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    let cleaned = cleaned.trim_matches('-');
    if cleaned.is_empty() {
        crate::vm_lifecycle::VmLifecycle::alm_org(Uuid::nil())
    } else {
        cleaned.to_string()
    }
}
