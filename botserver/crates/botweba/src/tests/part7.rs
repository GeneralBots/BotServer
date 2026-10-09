use super::*;

impl DashboardPage {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn url_pattern() -> &'static str {
        "/dashboard"
    }
}
