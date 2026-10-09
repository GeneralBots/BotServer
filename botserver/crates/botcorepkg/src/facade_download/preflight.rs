use super::*;

pub(super) fn safe_apt_get(args: &[&str]) -> Option<std::process::Output> {
    SafeCommand::new("apt-get")
        .and_then(|c| c.args(args))
        .ok()
        .and_then(|cmd| cmd.execute().ok())
}

pub(super) fn safe_brew(args: &[&str]) -> Option<std::process::Output> {
    SafeCommand::new("brew")
        .and_then(|c| c.args(args))
        .ok()
        .and_then(|cmd| cmd.execute().ok())
}
