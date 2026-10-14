use std::path::PathBuf;

use crate::os_abstraction::{OsAbstraction, Platform};

/// Linux-specific operating system abstraction.
pub struct LinuxAbstraction;

impl OsAbstraction for LinuxAbstraction {
    fn platform(&self) -> Platform {
        Platform::Linux
    }

    fn is_admin(&self) -> bool {
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
        std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/opt/gbo".to_string());
                PathBuf::from(home).join(".local/share/gbo")
            })
    }

    fn default_config_dir(&self) -> PathBuf {
        std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/opt/gbo".to_string());
                PathBuf::from(home).join(".config/gbo")
            })
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
    fn test_linux_platform() {
        let abs = LinuxAbstraction;
        assert_eq!(abs.platform(), Platform::Linux);
    }

    #[test]
    fn test_linux_default_dirs() {
        let abs = LinuxAbstraction;
        let data = abs.default_data_dir();
        let config = abs.default_config_dir();
        assert!(!data.as_os_str().is_empty());
        assert!(!config.as_os_str().is_empty());
    }

    #[test]
    fn test_linux_shell_command() {
        let abs = LinuxAbstraction;
        assert_eq!(abs.shell_command(), ("sh", "-c"));
        assert_eq!(abs.process_grep_command(), "pgrep");
    }

    #[test]
    fn test_get_abstraction_on_linux() {
        let abs = get_abstraction();
        if cfg!(target_os = "linux") {
            assert_eq!(abs.platform(), Platform::Linux);
        }
    }

    #[test]
    fn test_set_permissions_on_existing_file() {
        let abs = LinuxAbstraction;
        let tmp = std::env::temp_dir().join("linux_perms_test");
        let _ = std::fs::write(&tmp, b"test");
        if cfg!(unix) {
            assert!(abs.set_permissions(&tmp, 0o644).is_ok());
            assert!(abs.set_executable(&tmp).is_ok());
            assert!(abs.set_readonly_owner(&tmp).is_ok());
        }
        let _ = std::fs::remove_file(&tmp);
    }
}
