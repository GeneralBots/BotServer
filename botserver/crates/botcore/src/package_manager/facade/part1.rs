use super::*;

pub(super) fn safe_lxc(args: &[&str]) -> Option<std::process::Output> {
    SafeCommand::new("lxc")
        .and_then(|c| c.args(args))
        .ok()
        .and_then(|cmd| cmd.execute().ok())
}
pub(super) fn safe_lxc_exec_in_container(container: &str, command: &str) -> Option<std::process::Output> {
    SafeCommand::new("lxc")
        .and_then(|c| c.args(&["exec", container, "--", "bash", "-c", command]))
        .ok()
        .and_then(|cmd| cmd.execute().ok())
}
pub(super) fn safe_lxd(args: &[&str]) -> Option<std::process::Output> {
    let cmd_res = SafeCommand::new("lxd").and_then(|c| c.args(args));

    cmd_res.ok().and_then(|cmd| cmd.execute().ok())
}
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
