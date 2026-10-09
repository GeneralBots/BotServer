use super::*;

impl Default for ZitadelAuthConfig {
    fn default() -> Self {
        Self {
            issuer_url: "".to_string(),
            api_url: "".to_string(),
            client_id: String::new(),
            client_secret: String::new(),
            project_id: String::new(),
            cache_ttl_secs: 300,
            introspect_tokens: true,
        }
    }
}
impl ZitadelAuthConfig {
    /// Rebuilds the API base from validated URL components (scheme + host +
    /// port), dropping userinfo/path/query so service tokens can never be
    /// redirected to a tampered host.
    pub fn sanitized_api_base(&self) -> Option<String> {
        let parsed = url::Url::parse(&self.api_url).ok()?;
        let host = parsed.host_str()?.to_string();
        // Service tokens and user data travel to this host: require https,
        // allowing plain http only for loopback (never leaves the host).
        let loopback = host == "localhost" || host.starts_with("127.") || host == "[::1]";
        if parsed.scheme() != "https" && !(parsed.scheme() == "http" && loopback) {
            return None;
        }
        let mut base = format!("{}://{}", parsed.scheme(), host);
        if let Some(port) = parsed.port() {
            base.push_str(&format!(":{port}"));
        }
        Some(base)
    }

    pub fn new(issuer_url: &str, api_url: &str, client_id: &str, client_secret: &str) -> Self {
        Self {
            issuer_url: issuer_url.to_string(),
            api_url: api_url.to_string(),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            project_id: String::new(),
            cache_ttl_secs: 300,
            introspect_tokens: true,
        }
    }

    pub async fn from_vault(secrets: &dyn VaultConfigProvider) -> Result<Self> {
        let (url, project_id, client_id, client_secret) = secrets.get_directory_config().map_err(|e| anyhow::anyhow!(e))?;

        info!("Loaded Zitadel configuration from Vault");

        Ok(Self {
            issuer_url: url.clone(),
            api_url: url,
            client_id,
            client_secret,
            project_id,
            cache_ttl_secs: 300,
            introspect_tokens: true,
        })
    }

    pub async fn from_vault_or_default(secrets: &dyn VaultConfigProvider) -> Self {
        match Self::from_vault(secrets).await {
            Ok(config) => config,
            Err(e) => {
                warn!("Failed to load Zitadel config from Vault: {}. Using defaults.", e);
                Self::default()
            }
        }
    }

    pub fn is_configured(&self) -> bool {
        !self.client_id.is_empty() && !self.client_secret.is_empty()
    }

    pub fn with_project_id(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = project_id.into();
        self
    }

    pub fn with_cache_ttl(mut self, ttl_secs: u64) -> Self {
        self.cache_ttl_secs = ttl_secs;
        self
    }

    pub fn without_introspection(mut self) -> Self {
        self.introspect_tokens = false;
        self
    }
}
