use super::*;

impl DeviceInfo {
    pub fn from_user_agent(user_agent: &str) -> Self {
        let ua_lower = user_agent.to_lowercase();

        let device_type = if ua_lower.contains("mobile") || ua_lower.contains("android") {
            Some("Mobile".into())
        } else if ua_lower.contains("tablet") || ua_lower.contains("ipad") {
            Some("Tablet".into())
        } else {
            Some("Desktop".into())
        };

        let os = if ua_lower.contains("android") {
            Some("Android".into())
        } else if ua_lower.contains("iphone") || ua_lower.contains("ipad") {
            Some("iOS".into())
        } else if ua_lower.contains("windows") {
            Some("Windows".into())
        } else if ua_lower.contains("mac os") || ua_lower.contains("macos") {
            Some("macOS".into())
        } else if ua_lower.contains("linux") {
            Some("Linux".into())
        } else {
            None
        };

        let browser = if ua_lower.contains("firefox") {
            Some("Firefox".into())
        } else if ua_lower.contains("chrome") && !ua_lower.contains("edg") {
            Some("Chrome".into())
        } else if ua_lower.contains("safari") && !ua_lower.contains("chrome") {
            Some("Safari".into())
        } else if ua_lower.contains("edg") {
            Some("Edge".into())
        } else {
            None
        };

        Self {
            user_agent: Some(user_agent.to_string()),
            device_type,
            os,
            browser,
            fingerprint: None,
        }
    }

    pub fn with_fingerprint(mut self, fingerprint: String) -> Self {
        self.fingerprint = Some(fingerprint);
        self
    }
}
