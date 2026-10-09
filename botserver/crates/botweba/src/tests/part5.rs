use super::*;

impl Cookie {
    pub fn new(name: &str, value: &str) -> Self {
        Self {
            name: name.to_string(),
            value: value.to_string(),
            domain: None,
            path: None,
            secure: None,
            http_only: None,
        }
    }

    pub fn with_domain(mut self, domain: &str) -> Self {
        self.domain = Some(domain.to_string());
        self
    }

    pub fn with_path(mut self, path: &str) -> Self {
        self.path = Some(path.to_string());
        self
    }

    pub const fn secure(mut self) -> Self {
        self.secure = Some(true);
        self
    }

    pub const fn http_only(mut self) -> Self {
        self.http_only = Some(true);
        self
    }
}
