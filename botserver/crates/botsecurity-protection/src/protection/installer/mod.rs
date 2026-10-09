#[cfg(not(windows))]
pub(crate) use anyhow::Context;
pub(crate) use anyhow::Result;
#[cfg(not(windows))]
pub(crate) use std::fs;
#[cfg(unix)]
pub(crate) use std::os::unix::fs::PermissionsExt;
#[cfg(not(windows))]
pub(crate) use std::path::Path;
pub(crate) use tracing::{info, warn};
pub(crate) use botsecurity_core::command_guard::SafeCommand;

#[cfg(not(windows))]
const SUDOERS_FILE: &str = "/etc/sudoers.d/gb-protection";

#[cfg(not(windows))]
const SUDOERS_CONTENT: &str = r#"# General Bots Security Protection Tools
# This file is managed by botserver install protection
# DO NOT EDIT MANUALLY

# Lynis - security auditing
{user} ALL=(ALL) NOPASSWD: /usr/bin/lynis audit system
{user} ALL=(ALL) NOPASSWD: /usr/bin/lynis audit system --quick
{user} ALL=(ALL) NOPASSWD: /usr/bin/lynis audit system --quick --no-colors
{user} ALL=(ALL) NOPASSWD: /usr/bin/lynis audit system --no-colors

# RKHunter - rootkit detection
{user} ALL=(ALL) NOPASSWD: /usr/bin/rkhunter --check --skip-keypress
{user} ALL=(ALL) NOPASSWD: /usr/bin/rkhunter --check --skip-keypress --report-warnings-only
{user} ALL=(ALL) NOPASSWD: /usr/bin/rkhunter --update

# Chkrootkit - rootkit detection
{user} ALL=(ALL) NOPASSWD: /usr/bin/chkrootkit
{user} ALL=(ALL) NOPASSWD: /usr/bin/chkrootkit -q

# Suricata - IDS/IPS
{user} ALL=(ALL) NOPASSWD: /usr/bin/systemctl start suricata
{user} ALL=(ALL) NOPASSWD: /usr/bin/systemctl stop suricata
{user} ALL=(ALL) NOPASSWD: /usr/bin/systemctl restart suricata
{user} ALL=(ALL) NOPASSWD: /usr/bin/systemctl enable suricata
{user} ALL=(ALL) NOPASSWD: /usr/bin/systemctl disable suricata
{user} ALL=(ALL) NOPASSWD: /usr/bin/systemctl is-active suricata
{user} ALL=(ALL) NOPASSWD: /usr/bin/suricata-update

# ClamAV - antivirus
{user} ALL=(ALL) NOPASSWD: /usr/bin/systemctl start clamav-daemon
{user} ALL=(ALL) NOPASSWD: /usr/bin/systemctl stop clamav-daemon
{user} ALL=(ALL) NOPASSWD: /usr/bin/systemctl restart clamav-daemon
{user} ALL=(ALL) NOPASSWD: /usr/bin/systemctl enable clamav-daemon
{user} ALL=(ALL) NOPASSWD: /usr/bin/systemctl disable clamav-daemon
{user} ALL=(ALL) NOPASSWD: /usr/bin/systemctl is-active clamav-daemon
{user} ALL=(ALL) NOPASSWD: /usr/bin/freshclam

# LMD (Linux Malware Detect)
{user} ALL=(ALL) NOPASSWD: /usr/local/sbin/maldet -a /home
{user} ALL=(ALL) NOPASSWD: /usr/local/sbin/maldet -a /var/www
{user} ALL=(ALL) NOPASSWD: /usr/local/sbin/maldet -a /tmp
{user} ALL=(ALL) NOPASSWD: /usr/local/sbin/maldet --update-sigs
{user} ALL=(ALL) NOPASSWD: /usr/local/sbin/maldet --update-ver
"#;

#[cfg(not(windows))]
const PACKAGES: &[&str] = &[
    "lynis",
    "rkhunter",
    "chkrootkit",
    "suricata",
    "clamav",
    "clamav-daemon",
];

#[cfg(windows)]
const WINDOWS_TOOLS: &[(&str, &str)] = &[
    ("Windows Defender", "MpCmdRun"),
    ("PowerShell", "powershell"),
    ("Windows Firewall", "netsh"),
];

pub struct ProtectionInstaller {
    user: String,
}

    #[cfg(not(windows))]

#[derive(Default)]
pub struct InstallResult {
    pub success: bool,
    pub packages_installed: Vec<String>,
    pub sudoers_created: bool,
    pub databases_updated: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Default)]
pub struct UninstallResult {
    pub success: bool,
    pub sudoers_removed: bool,
    pub message: String,
    pub errors: Vec<String>,
}

#[derive(Default)]
pub struct VerifyResult {
    pub all_installed: bool,
    pub all_configured: bool,
    pub sudoers_exists: bool,
    pub tools: Vec<ToolVerification>,
}


pub struct ToolVerification {
    pub name: String,
    pub installed: bool,
    pub sudo_configured: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_result_default() {
        let result = InstallResult::default();
        assert!(!result.success);
        assert!(result.packages_installed.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_verify_result_default() {
        let result = VerifyResult::default();
        assert!(!result.all_installed);
        assert!(!result.all_configured);
        assert!(result.tools.is_empty());
    }

    #[test]
    fn test_tool_verification_default() {
        let tool = ToolVerification {
            name: "test".to_string(),
            installed: false,
            sudo_configured: false,
        };
        assert!(!tool.installed);
        assert!(!tool.sudo_configured);
    }

    #[test]
    fn test_uninstall_result_default() {
        let result = UninstallResult::default();
        assert!(!result.success);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_protection_installer_default() {
        let installer = ProtectionInstaller::default();
        assert!(!installer.user.is_empty());
    }

    #[cfg(not(windows))]
    #[test]
    fn test_sudoers_content_has_placeholder() {
        assert!(SUDOERS_CONTENT.contains("{user}"));
    }

    #[cfg(not(windows))]
    #[test]
    fn test_sudoers_content_no_wildcards() {
        // Ensure no dangerous wildcards in sudoers
        assert!(!SUDOERS_CONTENT.contains(" ALL=(ALL) ALL"));
    }

    #[cfg(not(windows))]
    #[test]
    fn test_packages_list() {
        assert!(PACKAGES.len() > 0);
        assert!(PACKAGES.contains(&"lynis"));
        assert!(PACKAGES.contains(&"rkhunter"));
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_tools_list() {
        assert!(WINDOWS_TOOLS.len() > 0);
        assert!(WINDOWS_TOOLS.iter().any(|t| t.0 == "Windows Defender"));
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
