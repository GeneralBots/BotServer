//! Execution methods of [`SafeCommand`] (split from `command_guard.rs`).

use super::*;

impl SafeCommand {
    pub fn execute(&self) -> Result<Output, CommandGuardError> {
        let mut cmd = std::process::Command::new(&self.command);
        self.append_args(&mut cmd);

        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        self.apply_common_env(&mut cmd);

        cmd.output()
            .map_err(|e| CommandGuardError::ExecutionFailed(e.to_string()))
    }

    pub async fn execute_async(&self) -> Result<Output, CommandGuardError> {
        let mut cmd = std::process::Command::new(&self.command);
        self.append_args(&mut cmd);

        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        self.apply_common_env(&mut cmd);

        cmd.output()
            .map_err(|e| CommandGuardError::ExecutionFailed(e.to_string()))
    }

    pub fn spawn(&mut self) -> Result<Child, CommandGuardError> {
        let mut cmd = std::process::Command::new(&self.command);
        self.append_args(&mut cmd);

        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        if let Some(stdout) = self.stdout.take() {
            cmd.stdout(stdout);
        }

        if let Some(stderr) = self.stderr.take() {
            cmd.stderr(stderr);
        }

        self.apply_common_env(&mut cmd);

        cmd.spawn()
            .map_err(|e| CommandGuardError::ExecutionFailed(e.to_string()))
    }

    pub fn spawn_with_envs(
        &self,
        envs: &HashMap<String, String>,
    ) -> Result<Child, CommandGuardError> {
        let mut cmd = std::process::Command::new(&self.command);
        self.append_args(&mut cmd);

        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        self.apply_common_env(&mut cmd);

        for (key, value) in envs {
            if validate_argument(key).is_ok() && validate_argument(value).is_ok() {
                cmd.env(key, value);
            }
        }

        cmd.spawn()
            .map_err(|e| CommandGuardError::ExecutionFailed(e.to_string()))
    }

    pub fn noop_child() -> Result<Child, CommandGuardError> {
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(["/C", "exit /b 0"])
                .spawn()
                .map_err(|e| CommandGuardError::ExecutionFailed(e.to_string()))
        }

        #[cfg(not(target_os = "windows"))]
        {
            std::process::Command::new("true")
                .spawn()
                .map_err(|e| CommandGuardError::ExecutionFailed(e.to_string()))
        }
    }
}
