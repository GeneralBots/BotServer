use std::path::PathBuf;

use crate::os_abstraction::{OsAbstraction, Platform};

/// Windows-specific operating system abstraction.
pub struct WindowsAbstraction;

impl OsAbstraction for WindowsAbstraction {
    fn platform(&self) -> Platform {
        Platform::Windows
    }

    fn is_admin(&self) -> bool {
        #[cfg(windows)]
        {
            std::process::Command::new("whoami")
                .args(["/groups"])
                .output()
                .ok()
                .map(|o| {
                    let s = String::from_utf8_lossy(&o.stdout);
                    s.contains("S-1-16-12288")
                })
                .unwrap_or(false)
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    fn set_permissions(&self, path: &std::path::Path, mode: u32) -> Result<(), String> {
        #[cfg(windows)]
        {
            let path_str = path.to_string_lossy();
            let mode_str = format!("{:03o}", mode);

            let mut args = vec![path_str.as_ref(), "/grant", "Everyone:R"];
            if mode & 0o200 != 0 {
                args[2] = "Everyone:RW";
            }
            if mode & 0o100 != 0 {
                args[2] = "Everyone:RX";
            }
            if mode & 0o400 != 0 {
                args[2] = "Everyone:R";
            }
            if mode & 0o700 == 0o700 {
                args[2] = "Everyone:F";
            }

            let output = std::process::Command::new("icacls")
                .args(&args)
                .output()
                .map_err(|e| format!("icacls execution failed: {e}"))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("icacls failed: {stderr}"));
            }

            match mode & 0o777 {
                0o644 | 0o600 | 0o400 => {
                    let inherit = std::process::Command::new("icacls")
                        .args([path_str.as_ref(), "/inheritance:r"])
                        .output();
                    if let Ok(o) = inherit {
                        if !o.status.success() {
                            let stderr = String::from_utf8_lossy(&o.stderr);
                            log::warn!("icacls /inheritance:r warning: {stderr}");
                        }
                    }
                }
                _ => {}
            }

            log::debug!("set_permissions({}, {}) via icacls", path_str, mode_str);
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            let _ = mode;
            Err("set_permissions not supported on this platform".to_string())
        }
    }

    fn set_executable(&self, path: &std::path::Path) -> Result<(), String> {
        #[cfg(windows)]
        {
            let path_str = path.to_string_lossy();
            let output = std::process::Command::new("icacls")
                .args([path_str.as_ref(), "/grant", "Everyone:RX"])
                .output()
                .map_err(|e| format!("icacls execution failed: {e}"))?;

            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("icacls /grant Everyone:RX failed: {stderr}"))
            }
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            Err("set_executable not supported on this platform".to_string())
        }
    }

    fn set_readonly_owner(&self, path: &std::path::Path) -> Result<(), String> {
        #[cfg(windows)]
        {
            let path_str = path.to_string_lossy();
            let disable_inherit = std::process::Command::new("icacls")
                .args([path_str.as_ref(), "/inheritance:r", "/grant", "Administrator:F"])
                .output()
                .map_err(|e| format!("icacls execution failed: {e}"))?;

            if disable_inherit.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&disable_inherit.stderr);
                Err(format!("icacls set_readonly_owner failed: {stderr}"))
            }
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            Err("set_readonly_owner not supported on this platform".to_string())
        }
    }

    fn default_data_dir(&self) -> PathBuf {
        #[cfg(windows)]
        {
            dirs::data_dir()
                .map(|p| p.join("GeneralBots"))
                .unwrap_or_else(|| PathBuf::from("C:\\ProgramData\\GeneralBots"))
        }
        #[cfg(not(windows))]
        {
            PathBuf::from("/opt/gbo/data")
        }
    }

    fn default_config_dir(&self) -> PathBuf {
        #[cfg(windows)]
        {
            dirs::config_dir()
                .map(|p| p.join("GeneralBots"))
                .unwrap_or_else(|| PathBuf::from("C:\\ProgramData\\GeneralBots\\conf"))
        }
        #[cfg(not(windows))]
        {
            PathBuf::from("/opt/gbo/conf")
        }
    }

    fn shell_command(&self) -> (&'static str, &'static str) {
        ("cmd", "/c")
    }

    fn process_grep_command(&self) -> &'static str {
        "tasklist"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::os_abstraction::get_abstraction;

    #[test]
    fn test_windows_platform() {
        let abs = WindowsAbstraction;
        assert_eq!(abs.platform(), Platform::Windows);
    }

    #[test]
    fn test_windows_default_dirs() {
        let abs = WindowsAbstraction;
        let data = abs.default_data_dir();
        let config = abs.default_config_dir();
        assert!(!data.as_os_str().is_empty());
        assert!(!config.as_os_str().is_empty());
    }

    #[test]
    fn test_windows_shell_command() {
        let abs = WindowsAbstraction;
        assert_eq!(abs.shell_command(), ("cmd", "/c"));
        assert_eq!(abs.process_grep_command(), "tasklist");
    }

    #[test]
    fn test_get_abstraction_on_windows() {
        let abs = get_abstraction();
        if cfg!(target_os = "windows") {
            assert_eq!(abs.platform(), Platform::Windows);
        }
    }

    #[test]
    fn test_set_permissions_no_crash() {
        let abs = WindowsAbstraction;
        let tmp = std::env::temp_dir().join("windows_perms_test");
        let _ = std::fs::write(&tmp, b"test");
        let result = abs.set_permissions(&tmp, 0o644);
        if cfg!(windows) {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
        let _ = std::fs::remove_file(&tmp);
    }
}
