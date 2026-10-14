use std::path::PathBuf;

use crate::os_abstraction::{OsAbstraction, Platform};

/// macOS-specific operating system abstraction.
pub struct MacOsAbstraction;

impl OsAbstraction for MacOsAbstraction {
    fn platform(&self) -> Platform {
        Platform::MacOS
    }

    fn is_admin(&self) -> bool {
        #[cfg(unix)]
        {
            std::process::Command::new("id")
                .arg("-u")
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        String::from_utf8_lossy(&o.stdout)
                            .trim()
                            .parse::<u32>()
                            .ok()
                    } else {
                        None
                    }
                })
                .map(|uid| uid == 0)
                .unwrap_or(false)
        }
        #[cfg(not(unix))]
        {
            false
        }
    }

    fn set_permissions(&self, path: &std::path::Path, mode: u32) -> Result<(), String> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(path).map_err(|e| format!("metadata: {e}"))?;
            let mut perms = metadata.permissions();
            perms.set_mode(mode);
            std::fs::set_permissions(path, perms).map_err(|e| format!("set_permissions: {e}"))
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            let _ = mode;
            Err("set_permissions not supported on this platform".to_string())
        }
    }

    fn set_executable(&self, path: &std::path::Path) -> Result<(), String> {
        #[cfg(unix)]
        {
            self.set_permissions(path, 0o755)
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            Err("set_executable not supported on this platform".to_string())
        }
    }

    fn set_readonly_owner(&self, path: &std::path::Path) -> Result<(), String> {
        #[cfg(unix)]
        {
            self.set_permissions(path, 0o400)
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            Err("set_readonly_owner not supported on this platform".to_string())
        }
    }

    fn default_data_dir(&self) -> PathBuf {
        #[cfg(target_os = "macos")]
        {
            dirs::data_dir()
                .map(|p| p.join("GeneralBots"))
                .unwrap_or_else(|| {
                    let home = std::env::var("HOME")
                        .unwrap_or_else(|_| "/opt/gbo".to_string());
                    PathBuf::from(home).join("Library/Application Support/GeneralBots")
                })
        }
        #[cfg(not(target_os = "macos"))]
        {
            PathBuf::from("/opt/gbo/data")
        }
    }

    fn default_config_dir(&self) -> PathBuf {
        #[cfg(target_os = "macos")]
        {
            dirs::config_dir()
                .map(|p| p.join("GeneralBots"))
                .unwrap_or_else(|| {
                    let home = std::env::var("HOME")
                        .unwrap_or_else(|_| "/opt/gbo".to_string());
                    PathBuf::from(home).join("Library/Preferences/GeneralBots")
                })
        }
        #[cfg(not(target_os = "macos"))]
        {
            PathBuf::from("/opt/gbo/conf")
        }
    }

    fn shell_command(&self) -> (&'static str, &'static str) {
        ("sh", "-c")
    }

    fn process_grep_command(&self) -> &'static str {
        "pgrep"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::os_abstraction::get_abstraction;

    #[test]
    fn test_macos_platform() {
        let abs = MacOsAbstraction;
        assert_eq!(abs.platform(), Platform::MacOS);
    }

    #[test]
    fn test_macos_default_dirs() {
        let abs = MacOsAbstraction;
        let data = abs.default_data_dir();
        let config = abs.default_config_dir();
        assert!(!data.as_os_str().is_empty());
        assert!(!config.as_os_str().is_empty());
    }

    #[test]
    fn test_macos_shell_command() {
        let abs = MacOsAbstraction;
        assert_eq!(abs.shell_command(), ("sh", "-c"));
        assert_eq!(abs.process_grep_command(), "pgrep");
    }

    #[test]
    fn test_get_abstraction_on_macos() {
        let abs = get_abstraction();
        if cfg!(target_os = "macos") {
            assert_eq!(abs.platform(), Platform::MacOS);
        }
    }

    #[test]
    fn test_set_permissions_on_existing_file() {
        let abs = MacOsAbstraction;
        let tmp = std::env::temp_dir().join("macos_perms_test");
        let _ = std::fs::write(&tmp, b"test");
        if cfg!(unix) {
            assert!(abs.set_permissions(&tmp, 0o644).is_ok());
            assert!(abs.set_executable(&tmp).is_ok());
            assert!(abs.set_readonly_owner(&tmp).is_ok());
        }
        let _ = std::fs::remove_file(&tmp);
    }
}
