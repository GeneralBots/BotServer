use super::*;

impl DlpPolicy {
    pub fn new(name: &str, data_types: Vec<SensitiveDataType>, action: DlpAction) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: None,
            enabled: true,
            data_types,
            action,
            severity_threshold: DlpSeverity::Low,
            exceptions: Vec::new(),
            applies_to: PolicyScope::All,
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    pub fn with_severity_threshold(mut self, threshold: DlpSeverity) -> Self {
        self.severity_threshold = threshold;
        self
    }

    pub fn with_scope(mut self, scope: PolicyScope) -> Self {
        self.applies_to = scope;
        self
    }

    pub fn add_exception(&mut self, exception: DlpException) {
        self.exceptions.push(exception);
    }

    pub fn matches_data_type(&self, data_type: SensitiveDataType) -> bool {
        self.data_types.contains(&data_type)
    }

    pub fn is_exception(&self, content: &str, user_id: Option<Uuid>) -> bool {
        for exception in &self.exceptions {
            match exception {
                DlpException::User(uid) => {
                    if user_id == Some(*uid) {
                        return true;
                    }
                }
                DlpException::Pattern(pattern) => {
                    if let Some(re) = crate::re_cache::dynamic(pattern) {
                        if re.is_match(content) {
                            return true;
                        }
                    }
                }
                DlpException::Domain(domain) => {
                    if content.contains(domain) {
                        return true;
                    }
                }
            }
        }
        false
    }
}
