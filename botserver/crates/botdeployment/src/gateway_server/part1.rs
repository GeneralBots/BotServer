
/// Published-app site domain. Deployed vibe apps are reachable at
/// `{appname}.{domain}` (#1261). Prefer the platform domain
/// (`GB_PLATFORM_DOMAIN`, e.g. `generalbots.org`) — the same wildcard zone
/// the bots use — and fall back to `SITE_DOMAIN` / `gb.solutions` for
/// legacy self-hosted setups.
pub(crate) fn published_domain() -> String {
    std::env::var("GB_PLATFORM_DOMAIN")
        .ok()
        .filter(|d| !d.trim().is_empty())
        .or_else(|| std::env::var("SITE_DOMAIN").ok().filter(|d| !d.trim().is_empty()))
        .unwrap_or_else(|| "gb.solutions".to_string())
}
