use super::*;

impl Default for PathGuardConfig {
    fn default() -> Self {
        Self {
            allowed_roots: vec![],
            allow_symlinks: false,
            allow_hidden_files: false,
            allowed_extensions: None,
            denied_extensions: vec![
                "exe".to_string(),
                "bat".to_string(),
                "cmd".to_string(),
                "sh".to_string(),
                "ps1".to_string(),
                "vbs".to_string(),
                "js".to_string(),
                "jar".to_string(),
                "msi".to_string(),
                "dll".to_string(),
                "so".to_string(),
            ],
            max_path_length: 4096,
            max_depth: 20,
        }
    }
}
impl PathGuardConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn permissive() -> Self {
        Self {
            allowed_roots: vec![],
            allow_symlinks: true,
            allow_hidden_files: true,
            allowed_extensions: None,
            denied_extensions: vec![],
            max_path_length: 8192,
            max_depth: 50,
        }
    }

    pub fn strict() -> Self {
        Self {
            allowed_roots: vec![],
            allow_symlinks: false,
            allow_hidden_files: false,
            allowed_extensions: Some(vec![
                "txt".to_string(),
                "pdf".to_string(),
                "doc".to_string(),
                "docx".to_string(),
                "xls".to_string(),
                "xlsx".to_string(),
                "csv".to_string(),
                "json".to_string(),
                "xml".to_string(),
                "png".to_string(),
                "jpg".to_string(),
                "jpeg".to_string(),
                "gif".to_string(),
                "svg".to_string(),
                "mp3".to_string(),
                "mp4".to_string(),
                "wav".to_string(),
                "zip".to_string(),
            ]),
            denied_extensions: vec![],
            max_path_length: 2048,
            max_depth: 10,
        }
    }

    pub fn with_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.allowed_roots.push(root.into());
        self
    }

    pub fn with_roots(mut self, roots: Vec<PathBuf>) -> Self {
        self.allowed_roots = roots;
        self
    }

    pub fn allow_symlinks(mut self, allow: bool) -> Self {
        self.allow_symlinks = allow;
        self
    }

    pub fn allow_hidden(mut self, allow: bool) -> Self {
        self.allow_hidden_files = allow;
        self
    }

    pub fn with_allowed_extensions(mut self, extensions: Vec<String>) -> Self {
        self.allowed_extensions = Some(extensions);
        self
    }

    pub fn with_denied_extensions(mut self, extensions: Vec<String>) -> Self {
        self.denied_extensions = extensions;
        self
    }

    pub fn with_max_length(mut self, length: usize) -> Self {
        self.max_path_length = length;
        self
    }

    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }
}
