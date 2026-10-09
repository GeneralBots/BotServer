use super::*;

#[cfg(target_os = "windows")]
pub(super) fn safe_nvcc_version() -> Option<std::process::Output> {
    SafeCommand::new("nvcc")
        .and_then(|c| c.arg("--version"))
        .ok()
        .and_then(|cmd| cmd.execute().ok())
}

pub fn safe_sh_command(script: &str) -> Option<std::process::Output> {
    safe_shell_command(script)
}

pub fn safe_shell_command(script: &str) -> Option<std::process::Output> {
    #[cfg(target_os = "windows")]
    {
        SafeCommand::new("cmd")
            .and_then(|c| c.arg("/C"))
            .and_then(|c| c.trusted_shell_script_arg(script))
            .ok()
            .and_then(|cmd| cmd.execute().ok())
    }

    #[cfg(not(target_os = "windows"))]
    {
        SafeCommand::new("sh")
            .and_then(|c| c.arg("-c"))
            .and_then(|c| c.trusted_shell_script_arg(script))
            .ok()
            .and_then(|cmd| cmd.execute().ok())
    }
}

pub fn safe_pgrep(args: &[&str]) -> Option<std::process::Output> {
    #[cfg(target_os = "windows")]
    {
        let filter = args.join(" ");
        SafeCommand::new("tasklist")
            .and_then(|c| c.arg("/FI"))
            .and_then(|c| c.arg(&format!("IMAGENAME eq {}", filter)))
            .ok()
            .and_then(|cmd| cmd.execute().ok())
    }

    #[cfg(not(target_os = "windows"))]
    {
        SafeCommand::new("pgrep")
            .and_then(|c| c.args(args))
            .ok()
            .and_then(|cmd| cmd.execute().ok())
    }
}
