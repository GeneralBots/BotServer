use super::*;

impl CspBuilder {
    pub fn new() -> Self {
        Self {
            directives: HashMap::new(),
        }
    }

    pub fn default_src(mut self, sources: &[&str]) -> Self {
        self.directives.insert(
            "default-src".to_string(),
            sources.iter().map(|s| (*s).to_string()).collect(),
        );
        self
    }

    pub fn script_src(mut self, sources: &[&str]) -> Self {
        self.directives.insert(
            "script-src".to_string(),
            sources.iter().map(|s| (*s).to_string()).collect(),
        );
        self
    }

    pub fn style_src(mut self, sources: &[&str]) -> Self {
        self.directives.insert(
            "style-src".to_string(),
            sources.iter().map(|s| (*s).to_string()).collect(),
        );
        self
    }

    pub fn img_src(mut self, sources: &[&str]) -> Self {
        self.directives.insert(
            "img-src".to_string(),
            sources.iter().map(|s| (*s).to_string()).collect(),
        );
        self
    }

    pub fn font_src(mut self, sources: &[&str]) -> Self {
        self.directives.insert(
            "font-src".to_string(),
            sources.iter().map(|s| (*s).to_string()).collect(),
        );
        self
    }

    pub fn connect_src(mut self, sources: &[&str]) -> Self {
        self.directives.insert(
            "connect-src".to_string(),
            sources.iter().map(|s| (*s).to_string()).collect(),
        );
        self
    }

    pub fn frame_src(mut self, sources: &[&str]) -> Self {
        self.directives.insert(
            "frame-src".to_string(),
            sources.iter().map(|s| (*s).to_string()).collect(),
        );
        self
    }

    pub fn frame_ancestors(mut self, sources: &[&str]) -> Self {
        self.directives.insert(
            "frame-ancestors".to_string(),
            sources.iter().map(|s| (*s).to_string()).collect(),
        );
        self
    }

    pub fn base_uri(mut self, sources: &[&str]) -> Self {
        self.directives.insert(
            "base-uri".to_string(),
            sources.iter().map(|s| (*s).to_string()).collect(),
        );
        self
    }

    pub fn form_action(mut self, sources: &[&str]) -> Self {
        self.directives.insert(
            "form-action".to_string(),
            sources.iter().map(|s| (*s).to_string()).collect(),
        );
        self
    }

    pub fn object_src(mut self, sources: &[&str]) -> Self {
        self.directives.insert(
            "object-src".to_string(),
            sources.iter().map(|s| (*s).to_string()).collect(),
        );
        self
    }

    pub fn media_src(mut self, sources: &[&str]) -> Self {
        self.directives.insert(
            "media-src".to_string(),
            sources.iter().map(|s| (*s).to_string()).collect(),
        );
        self
    }

    pub fn worker_src(mut self, sources: &[&str]) -> Self {
        self.directives.insert(
            "worker-src".to_string(),
            sources.iter().map(|s| (*s).to_string()).collect(),
        );
        self
    }

    pub fn upgrade_insecure_requests(mut self) -> Self {
        self.directives
            .insert("upgrade-insecure-requests".to_string(), vec![]);
        self
    }

    pub fn block_all_mixed_content(mut self) -> Self {
        self.directives
            .insert("block-all-mixed-content".to_string(), vec![]);
        self
    }

    pub fn build(self) -> String {
        self.directives
            .iter()
            .map(|(directive, sources)| {
                if sources.is_empty() {
                    directive.clone()
                } else {
                    format!("{} {}", directive, sources.join(" "))
                }
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}
impl Default for CspBuilder {
    fn default() -> Self {
        Self::new()
    }
}
