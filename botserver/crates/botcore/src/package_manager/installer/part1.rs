use super::*;

pub(super) fn safe_sh_command(script: &str) -> Option<std::process::Output> {
    let abs = crate::os_abstraction::get_abstraction();
    let (shell, flag) = abs.shell_command();
    #[cfg(target_os = "windows")]
    let script = &format!("\"{script}\"");
    SafeCommand::new(shell)
        .and_then(|c| c.arg(flag))
        .and_then(|c| c.raw_shell_script_arg(script))
        .ok()
        .and_then(|cmd| cmd.execute().ok())
}
pub(super) fn safe_pgrep(args: &[&str]) -> Option<std::process::Output> {
    let abs = crate::os_abstraction::get_abstraction();
    let cmd_name = abs.process_grep_command();
    SafeCommand::new(cmd_name)
        .and_then(|c| c.args(args))
        .ok()
        .and_then(|cmd| cmd.execute().ok())
}
