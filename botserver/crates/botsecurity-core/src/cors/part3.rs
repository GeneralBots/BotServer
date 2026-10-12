use super::*;

impl Default for OriginValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl OriginValidator {
    pub fn new() -> Self {
        Self {
            allowed_origins: HashSet::new(),
            allow_localhost: false,
            allowed_patterns: Vec::new(),
        }
    }

    pub fn allow_origin(mut self, origin: impl Into<String>) -> Self {
        self.allowed_origins.insert(origin.into());
        self
    }

    pub fn allow_localhost(mut self, allow: bool) -> Self {
        self.allow_localhost = allow;
        self
    }

    pub fn allow_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.allowed_patterns.push(pattern.into());
        self
    }

    pub fn from_config(origins: Vec<String>, patterns: Vec<String>, allow_localhost: bool) -> Self {
        let mut validator = Self::new();

        for origin in origins {
            if !origin.is_empty() {
                validator.allowed_origins.insert(origin);
            }
        }

        for pattern in patterns {
            if !pattern.is_empty() {
                validator.allowed_patterns.push(pattern);
            }
        }

        validator.allow_localhost = allow_localhost;
        validator
    }

    pub fn from_allowed_origins(origins: Vec<String>) -> Self {
        Self::from_config(origins, Vec::new(), false)
    }

    pub fn is_allowed(&self, origin: &str) -> bool {
        // A request without an `Origin` header is same-origin, never a cross
        // origin request, so it is allowed regardless of the allowlist.
        if origin.is_empty() {
            return true;
        }

        if self.allowed_origins.contains(origin) {
            return true;
        }

        if self.allow_localhost && is_localhost_origin(origin) {
            return true;
        }

        for pattern in &self.allowed_patterns {
            if matches_pattern(origin, pattern) {
                return true;
            }
        }

        false
    }
}
pub(super) fn is_localhost_origin(origin: &str) -> bool {
    // No `Origin` header means the request did not cross an origin boundary, so
    // it is treated as same-origin and is always acceptable.
    if origin.is_empty() {
        return true;
    }

    let localhost_patterns = [
        "http://localhost",
        "https://localhost",
        "http://127.0.0.1",
        "https://127.0.0.1",
        "http://[::1]",
        "https://[::1]",
    ];

    for pattern in &localhost_patterns {
        if origin.starts_with(pattern) {
            return true;
        }
    }

    false
}
pub(super) fn matches_pattern(origin: &str, pattern: &str) -> bool {
    if pattern.starts_with("*.") {
        let suffix = &pattern[1..];
        if let Some(host) = extract_host(origin) {
            return host.ends_with(suffix) || host == &suffix[1..];
        }
    }

    if let Some(prefix) = pattern.strip_suffix("*") {
        return origin.starts_with(prefix);
    }

    origin == pattern
}
pub(super) fn extract_host(origin: &str) -> Option<&str> {
    // Same-origin requests carry no `Origin` header; report the local host so
    // callers comparing against an allowlist still match.
    if origin.is_empty() {
        return Some("localhost");
    }

    let without_scheme = origin
        .strip_prefix("https://")
        .or_else(|| origin.strip_prefix("http://"))?;

    Some(without_scheme.split(':').next().unwrap_or(without_scheme))
}
