use super::*;

pub(super) fn prepare_command(
    program: &str,
    args: &[String],
    cwd: &std::path::Path,
) -> Result<std::process::Command, GuardError> {
    validate_program(program)?;
    if !program_is_allowed(program) {
        return Err(GuardError::CommandNotAllowed(program.into()));
    }
    if args.len() > MAX_ARGS {
        return Err(GuardError::InvalidArgument("too many arguments".into()));
    }
    for arg in args {
        validate_arg(arg)?;
    }

    #[cfg(target_os = "windows")]
    let mut command = {
        let node_root = std::env::var_os("ProgramFiles")
            .map(std::path::PathBuf::from)
            .map(|root| root.join("nodejs"));
        let npm_cli = match program {
            "npm" => Some("npm-cli.js"),
            "npx" => Some("npx-cli.js"),
            _ => None,
        };
        match (node_root, npm_cli) {
            (Some(root), Some(cli)) if root.join("node.exe").is_file() => {
                let mut command = std::process::Command::new(root.join("node.exe"));
                command.arg(root.join("node_modules").join("npm").join("bin").join(cli));
                command
            }
            _ => std::process::Command::new(program),
        }
    };
    #[cfg(not(target_os = "windows"))]
    let mut command = std::process::Command::new(program);

    command.args(args).current_dir(cwd).env_clear();

    #[cfg(not(target_os = "windows"))]
    command.env("PATH", CHILD_PATH);

    #[cfg(target_os = "windows")]
    for key in [
        "SystemRoot",
        "WINDIR",
        "COMSPEC",
        "PATH",
        "PATHEXT",
        "ProgramFiles",
        "USERPROFILE",
        "TEMP",
        "TMP",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    Ok(command)
}
