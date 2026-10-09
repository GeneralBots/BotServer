use super::*;

/// Bucket that hosts an org's workspaces: `{base}.gborg`.
///
/// `organizations.slug` carries two conventions: signup stores the plain name
/// (`beiner`), while Drive discovery's legacy naming appends `-org`
/// (`cristo-org`) — `ensure_tenant_and_org` treats both as aliases of the same
/// org (#779). The physical bucket is always named after the plain tenant
/// (`cristo.gborg`), so the legacy suffix is stripped before building the name.
pub fn org_drive_bucket(org_slug: &str) -> String {
    let slug = org_slug.trim();
    let stripped = slug.strip_suffix(LEGACY_ORG_SUFFIX).unwrap_or(slug);
    let base = if stripped.is_empty() { slug } else { stripped };
    format!("{base}{GBORG_SUFFIX}")
}
