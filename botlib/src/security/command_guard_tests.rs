//! Unit tests for the `SafeCommand` allowlist.
//!
//! Split out of `command_guard.rs` to keep that file inside the project size
//! limit; the allowlist itself stays beside the guard implementation.

use super::*;

#[test]
fn test_safe_command_allowed() {
    assert!(SafeCommand::new("pdftotext").is_ok());
    assert!(SafeCommand::new("pandoc").is_ok());
    assert!(SafeCommand::new("nvidia-smi").is_ok());
}

#[test]
fn test_safe_command_disallowed() {
    assert!(SafeCommand::new("wget").is_err());
    assert!(SafeCommand::new("nc").is_err());
    assert!(SafeCommand::new("netcat").is_err());
    assert!(SafeCommand::new("dd").is_err());
    assert!(SafeCommand::new("mkfs").is_err());
}

/// Binaries that must never reach the allowlist: they either open a raw socket
/// to a remote listener or let a caller rewrite the filesystem. Every one of
/// them was removed from `ALLOWED_COMMANDS`; these assertions pin that down so
/// a future entry does not silently reintroduce host command execution.
#[test]
fn dangerous_binaries_stay_out_of_the_allowlist() {
    for binary in [
        "nc", "netcat", "wget", "dd", "mkfs", "rm", "openssl", "initdb", "pg_ctl", "createdb",
        "systemctl",
    ] {
        assert!(
            SafeCommand::new(binary).is_err(),
            "{binary} must not be executable through SafeCommand"
        );
    }
}

/// `rclone`, `osascript` and `rm` are reachable from the desktop shell, but
/// only through the guard in `botapp::desktop::safe_command`. The server-side
/// guard has no caller for them, so it must keep refusing them.
#[test]
fn desktop_only_binaries_are_not_server_allowed() {
    for binary in ["rclone", "osascript", "rm"] {
        assert!(SafeCommand::new(binary).is_err());
    }
}