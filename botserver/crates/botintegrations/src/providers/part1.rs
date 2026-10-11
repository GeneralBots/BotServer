

pub(super) fn outcome_key_is_sensitive(key: &str) -> bool {
    let lower = key.to_lowercase();
    [
        "access_key",
        "secret",
        "session_token",
        "token",
        "api_key",
        "password",
        "authorization",
    ]
    .iter()
    .any(|fragment| lower.contains(fragment))
}
