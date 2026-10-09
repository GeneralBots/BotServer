use super::*;

pub fn install_binary(temp_file: &std::path::Path, bin_path: &std::path::Path, name: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    let name = if !name.ends_with(".exe") {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };
    #[cfg(not(target_os = "windows"))]
    let name = name.to_string();
    let final_path = bin_path.join(&name);
    if temp_file.to_string_lossy().contains("botserver-installers") {
        std::fs::copy(temp_file, &final_path)?;
    } else {
        std::fs::rename(temp_file, &final_path)?;
    }
    make_executable(&final_path)?;
    Ok(())
}

pub fn make_executable(path: &std::path::Path) -> Result<()> {
    #[cfg(unix)]
    {
        let _ = botlib::os::fs::get_permissions_manager().set_executable(path);
    }
    #[cfg(windows)]
    {
        trace!("Skipping chmod on Windows for: {}", path.display());
    }
    Ok(())
}

pub fn run_commands(commands: &[String], target: &str, component: &str, base_path: &std::path::Path, db_password_override: &str) -> Result<()> {
    run_commands_with_password(commands, target, component, base_path, db_password_override)
}

pub fn run_commands_with_password(
    commands: &[String], target: &str, component: &str,
    base_path: &std::path::Path, db_password_override: &str,
) -> Result<()> {
    let bin_path = if target == "local" {
        base_path.join("bin").join(component)
    } else {
        PathBuf::from("/opt/gbo/bin")
    };
    let data_path = if target == "local" {
        base_path.join("data").join(component)
    } else {
        PathBuf::from("/opt/gbo/data")
    };
    let conf_path = if target == "local" {
        base_path.join("conf")
    } else {
        PathBuf::from("/opt/gbo/conf")
    };
    let logs_path = if target == "local" {
        base_path.join("logs").join(component)
    } else {
        PathBuf::from("/opt/gbo/logs")
    };
    let db_password = if let Ok(env_pwd) = std::env::var("BOOTSTRAP_DB_PASSWORD") {
        env_pwd
    } else if !db_password_override.is_empty() {
        db_password_override.to_string()
    } else {
        match get_database_url_sync() {
            Ok(url) => {
                let (_, password, _, _, _) = parse_database_url(&url);
                password
            }
            Err(_) => {
                trace!("Vault not available for DB_PASSWORD, using empty string");
                String::new()
            }
        }
    };
    for cmd in commands {
        let rendered_cmd = cmd
            .replace("{{BIN_PATH}}", &bin_path.to_string_lossy())
            .replace("{{DATA_PATH}}", &data_path.to_string_lossy())
            .replace("{{CONF_PATH}}", &conf_path.to_string_lossy())
            .replace("{{LOGS_PATH}}", &logs_path.to_string_lossy())
            .replace("{{STACK_PATH}}", &base_path.to_string_lossy())
            .replace("{{DATA_PATH_FWD}}", &data_path.to_string_lossy().replace('\\', "/"))
            .replace("{{DB_PASSWORD}}", &db_password);
        if target == "local" {
            trace!("Executing command: {}", rendered_cmd);
            #[cfg(target_os = "windows")]
            let cmd_result = {
                use std::os::windows::process::CommandExt;
                let wrapped = format!("\"{rendered_cmd}\"");
                let output = std::process::Command::new("cmd")
                    .arg("/C")
                    .raw_arg(&wrapped)
                    .current_dir(&bin_path)
                    .output()
                    .with_context(|| {
                        format!("Failed to execute command for '{}'", component)
                    })?;
                output
            };
            #[cfg(not(target_os = "windows"))]
            let cmd_result = {
                let cmd = SafeCommand::new("bash")
                    .and_then(|c| c.arg("-c"))
                    .and_then(|c| c.trusted_shell_script_arg(&rendered_cmd))
                    .and_then(|c| c.working_dir(&bin_path))
                    .map_err(|e| anyhow::anyhow!("Failed to build bash command: {}", e))?;
                cmd.execute().with_context(|| {
                    format!("Failed to execute command for component '{}'", component)
                })?
            };
            if !cmd_result.status.success() {
                log::error!("Command had non-zero exit: {}", String::from_utf8_lossy(&cmd_result.stderr));
            }
        } else {
            exec_in_container(target, &rendered_cmd)?;
        }
    }
    Ok(())
}
