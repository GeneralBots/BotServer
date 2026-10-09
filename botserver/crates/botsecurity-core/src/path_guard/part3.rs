use super::*;

impl Default for PathGuard {
    fn default() -> Self {
        Self::new(PathGuardConfig::default())
    }
}
impl PathGuard {
    pub fn new(config: PathGuardConfig) -> Self {
        Self { config }
    }

    pub fn validate(&self, path: &Path) -> Result<PathBuf, PathGuardError> {
        let path_str = path.to_string_lossy();
        if path_str.contains('\0') {
            warn!(path = %path_str, "Path contains null byte");
            return Err(PathGuardError::NullByte);
        }

        if path_str.is_empty() {
            return Err(PathGuardError::EmptyPath);
        }

        if path_str.len() > self.config.max_path_length {
            warn!(path_len = path_str.len(), max = self.config.max_path_length, "Path too long");
            return Err(PathGuardError::PathTooLong);
        }

        if path.is_absolute() && !self.config.allowed_roots.is_empty() {
            let is_within_root = self.config.allowed_roots.iter().any(|root| {
                path.starts_with(root)
            });
            if !is_within_root {
                warn!(path = %path_str, "Absolute path outside allowed roots");
                return Err(PathGuardError::AbsolutePath);
            }
        }

        let mut depth: usize = 0;
        let mut normalized = PathBuf::new();

        for component in path.components() {
            match component {
                Component::ParentDir => {
                    if normalized.pop() {
                        depth = depth.saturating_sub(1);
                    } else {
                        warn!(path = %path_str, "Path traversal attempt detected");
                        return Err(PathGuardError::PathTraversal);
                    }
                }
                Component::Normal(name) => {
                    let name_str = name.to_string_lossy();

                    if !self.config.allow_hidden_files && name_str.starts_with('.') {
                        warn!(path = %path_str, component = %name_str, "Hidden file not allowed");
                        return Err(PathGuardError::HiddenFileNotAllowed);
                    }

                    if has_dangerous_patterns(&name_str) {
                        warn!(path = %path_str, component = %name_str, "Invalid path component");
                        return Err(PathGuardError::InvalidComponent);
                    }

                    normalized.push(name);
                    depth += 1;

                    if depth > self.config.max_depth {
                        warn!(path = %path_str, depth = depth, max = self.config.max_depth, "Path depth exceeded");
                        return Err(PathGuardError::PathTooLong);
                    }
                }
                Component::RootDir => {
                    normalized.push(Component::RootDir);
                }
                Component::Prefix(prefix) => {
                    normalized.push(prefix.as_os_str());
                }
                Component::CurDir => {}
            }
        }

        if let Some(ext) = normalized.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();

            if let Some(ref allowed) = self.config.allowed_extensions {
                if !allowed.iter().any(|e| e.to_lowercase() == ext_str) {
                    warn!(path = %path_str, extension = %ext_str, "Extension not in allowed list");
                    return Err(PathGuardError::InvalidExtension);
                }
            }

            if self.config.denied_extensions.iter().any(|e| e.to_lowercase() == ext_str) {
                warn!(path = %path_str, extension = %ext_str, "Extension is denied");
                return Err(PathGuardError::InvalidExtension);
            }
        }

        Ok(normalized)
    }

    pub fn validate_and_resolve(&self, base: &Path, path: &Path) -> Result<PathBuf, PathGuardError> {
        let validated = self.validate(path)?;
        let full_path = base.join(&validated);

        if !self.config.allowed_roots.is_empty() {
            let is_within_root = self.config.allowed_roots.iter().any(|root| {
                full_path.starts_with(root)
            });
            if !is_within_root {
                warn!(
                    path = %full_path.display(),
                    "Resolved path outside allowed roots"
                );
                return Err(PathGuardError::OutsideAllowedRoot);
            }
        }

        Ok(full_path)
    }

    pub fn validate_existing(&self, path: &Path) -> Result<PathBuf, PathGuardError> {
        let validated = self.validate(path)?;

        if !self.config.allow_symlinks && validated.is_symlink() {
            warn!(path = %validated.display(), "Symlink not allowed");
            return Err(PathGuardError::SymlinkNotAllowed);
        }

        if let Ok(canonical) = validated.canonicalize() {
            if !self.config.allowed_roots.is_empty() {
                let is_within_root = self.config.allowed_roots.iter().any(|root| {
                    if let Ok(root_canonical) = root.canonicalize() {
                        canonical.starts_with(&root_canonical)
                    } else {
                        canonical.starts_with(root)
                    }
                });
                if !is_within_root {
                    warn!(
                        path = %canonical.display(),
                        "Canonical path outside allowed roots"
                    );
                    return Err(PathGuardError::OutsideAllowedRoot);
                }
            }
            Ok(canonical)
        } else {
            Ok(validated)
        }
    }
}
pub(super) fn has_dangerous_patterns(name: &str) -> bool {
    let dangerous = [
        "..",
        "...",
        "~",
        "$",
        "`",
        "|",
        ";",
        "&",
        "<",
        ">",
        "\\",
        "%00",
        "%2e",
        "%2f",
        "%5c",
        "\r",
        "\n",
        "\t",
    ];

    for pattern in &dangerous {
        if name.contains(pattern) {
            return true;
        }
    }

    if name.chars().any(|c| c.is_control()) {
        return true;
    }

    false
}
pub fn sanitize_filename(name: &str) -> String {
    let dangerous_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|', '\0'];

    let sanitized: String = name
        .chars()
        .map(|c| {
            if dangerous_chars.contains(&c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();

    let sanitized = sanitized.trim_matches(|c| c == '.' || c == ' ');

    if sanitized.is_empty() {
        return "unnamed".to_string();
    }

    let reserved = [
        "CON", "PRN", "AUX", "NUL",
        "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    let upper = sanitized.to_uppercase();
    let base_name = upper.split('.').next().unwrap_or("");
    if reserved.contains(&base_name) {
        return format!("_{}", sanitized);
    }

    if sanitized.len() > 255 {
        sanitized[..255].to_string()
    } else {
        sanitized.to_string()
    }
}
pub fn sanitize_path_component(component: &str) -> String {
    sanitize_filename(component)
}
pub fn is_safe_path(path: &Path) -> bool {
    PathGuard::default().validate(path).is_ok()
}
pub fn join_safe(base: &Path, relative: &Path) -> Result<PathBuf, PathGuardError> {
    let guard = PathGuard::new(PathGuardConfig::default().with_root(base.to_path_buf()));
    guard.validate_and_resolve(base, relative)
}
pub fn canonicalize_safe(path: &Path, allowed_root: &Path) -> Result<PathBuf, PathGuardError> {
    let guard = PathGuard::new(PathGuardConfig::default().with_root(allowed_root.to_path_buf()));
    guard.validate_existing(path)
}
