use super::*;

impl std::fmt::Debug for CodeSandbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeSandbox")
            .field("config", &self.config)
            .field("session_id", &self.session_id)
            .finish()
    }
}
impl CodeSandbox {
    pub fn new(config: SandboxConfig, session_id: Uuid) -> Self {
        Self { config, session_id }
    }

    pub async fn execute(&self, code: &str, language: CodeLanguage) -> ExecutionResult {
        if !self.config.enabled {
            return ExecutionResult::error("Sandbox execution is disabled");
        }

        let start_time = std::time::Instant::now();

        let result = match self.config.runtime {
            SandboxRuntime::LXC => self.execute_lxc(code, &language).await,
            SandboxRuntime::Docker => self.execute_docker(code, &language).await,
            SandboxRuntime::Firecracker => self.execute_firecracker(code, &language).await,
            SandboxRuntime::Process => self.execute_process(code, &language).await,
        };

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        match result {
            Ok(mut exec_result) => {
                exec_result.execution_time_ms = execution_time_ms;
                exec_result
            }
            Err(e) => {
                let mut err_result = ExecutionResult::error(&e);
                err_result.execution_time_ms = execution_time_ms;
                err_result
            }
        }
    }

    async fn execute_lxc(
        &self,
        code: &str,
        language: &CodeLanguage,
    ) -> Result<ExecutionResult, String> {
        let container_name = format!("gb-sandbox-{}", Uuid::new_v4());

        std::fs::create_dir_all(&self.config.work_dir)
            .map_err(|e| format!("Failed to create work dir: {}", e))?;

        let code_file = format!(
            "{}/{}.{}",
            self.config.work_dir,
            self.session_id,
            language.file_extension()
        );
        std::fs::write(&code_file, code)
            .map_err(|e| format!("Failed to write code file: {}", e))?;

        let lxc_conf = format!("/etc/lxc/{}.conf", language.lxc_image());
        let interpreter = language.interpreter();

        let timeout_duration = Duration::from_secs(self.config.timeout_seconds);
        let output = timeout(timeout_duration, async {
            let cmd_result = SafeCommand::new("lxc-execute")
                .and_then(|c| c.arg("-n"))
                .and_then(|c| c.arg(&container_name))
                .and_then(|c| c.arg("-f"))
                .and_then(|c| c.arg(&lxc_conf))
                .and_then(|c| c.arg("--"))
                .and_then(|c| c.arg(interpreter))
                .and_then(|c| c.arg(&code_file));

            match cmd_result {
                Ok(cmd) => cmd.execute_async().await.map_err(|e| std::io::Error::other(e.to_string())),
                Err(e) => Err(std::io::Error::other(e.to_string())),
            }
        })
        .await;

        let _ = std::fs::remove_file(&code_file);

        match output {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);

                Ok(ExecutionResult {
                    stdout,
                    stderr,
                    exit_code,
                    execution_time_ms: 0,
                    timed_out: false,
                    killed: !output.status.success(),
                    error: None,
                })
            }
            Ok(Err(e)) => Err(format!("LXC execution failed: {}", e)),
            Err(_) => Ok(ExecutionResult::timeout()),
        }
    }

    async fn execute_docker(
        &self,
        code: &str,
        language: &CodeLanguage,
    ) -> Result<ExecutionResult, String> {
        let image = match language {
            CodeLanguage::Python => "python:3.11-slim",
            CodeLanguage::JavaScript => "node:20-slim",
            CodeLanguage::Bash => "alpine:latest",
        };

        let args = vec![
            "run".to_string(),
            "--rm".to_string(),
            "--network".to_string(),
            if self.config.network_enabled {
                "bridge"
            } else {
                "none"
            }
            .to_string(),
            "--memory".to_string(),
            format!("{}m", self.config.memory_limit_mb),
            "--cpus".to_string(),
            format!("{:.2}", f64::from(self.config.cpu_limit_percent) / 100.0),
            "--read-only".to_string(),
            "--security-opt".to_string(),
            "no-new-privileges".to_string(),
            image.to_string(),
            language.interpreter().to_string(),
            "-c".to_string(),
            code.to_string(),
        ];

        let timeout_duration = Duration::from_secs(self.config.timeout_seconds);
        let output = timeout(timeout_duration, async {
            let cmd_result = SafeCommand::new("docker")
                .and_then(|c| c.args(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>()));

            match cmd_result {
                Ok(cmd) => cmd.execute_async().await.map_err(|e| std::io::Error::other(e.to_string())),
                Err(e) => Err(std::io::Error::other(e.to_string())),
            }
        })
        .await;

        match output {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);

                Ok(ExecutionResult {
                    stdout,
                    stderr,
                    exit_code,
                    execution_time_ms: 0,
                    timed_out: false,
                    killed: !output.status.success(),
                    error: None,
                })
            }
            Ok(Err(e)) => Err(format!("Docker execution failed: {}", e)),
            Err(_) => Ok(ExecutionResult::timeout()),
        }
    }

    async fn execute_firecracker(
        &self,
        code: &str,
        language: &CodeLanguage,
    ) -> Result<ExecutionResult, String> {
        warn!("Firecracker runtime not yet implemented, falling back to process isolation");
        self.execute_process(code, language).await
    }

    async fn execute_process(
        &self,
        code: &str,
        language: &CodeLanguage,
    ) -> Result<ExecutionResult, String> {
        let temp_dir = format!("{}/{}", self.config.work_dir, Uuid::new_v4());
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to create temp dir: {}", e))?;

        let code_file = format!("{}/code.{}", temp_dir, language.file_extension());
        std::fs::write(&code_file, code)
            .map_err(|e| format!("Failed to write code file: {}", e))?;

        let (cmd_name, cmd_args): (&str, Vec<&str>) = match *language {
            CodeLanguage::Python => ("python3", vec![code_file.as_str()]),
            CodeLanguage::JavaScript => ("node", vec![code_file.as_str()]),
            CodeLanguage::Bash => ("bash", vec![code_file.as_str()]),
        };

        let timeout_duration = Duration::from_secs(self.config.timeout_seconds);
        let output = timeout(timeout_duration, async {
            let cmd_result = SafeCommand::new(cmd_name)
                .and_then(|c| c.args(&cmd_args))
                .and_then(|c| c.working_dir(std::path::Path::new(&temp_dir)));

            match cmd_result {
                Ok(cmd) => cmd.execute_async().await.map_err(|e| std::io::Error::other(e.to_string())),
                Err(e) => Err(std::io::Error::other(e.to_string())),
            }
        })
        .await;

        let _ = std::fs::remove_dir_all(&temp_dir);

        match output {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);

                Ok(ExecutionResult {
                    stdout,
                    stderr,
                    exit_code,
                    execution_time_ms: 0,
                    timed_out: false,
                    killed: false,
                    error: None,
                })
            }
            Ok(Err(e)) => Err(format!("Process execution failed: {}", e)),
            Err(_) => Ok(ExecutionResult::timeout()),
        }
    }

    pub async fn execute_file(&self, file_path: &str, language: CodeLanguage) -> ExecutionResult {
        match std::fs::read_to_string(file_path) {
            Ok(code) => self.execute(&code, language).await,
            Err(e) => ExecutionResult::error(&format!("Failed to read file: {}", e)),
        }
    }
}
pub fn register_sandbox_keywords(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    run_python_keyword(Arc::clone(&state), user.clone(), engine);
    run_javascript_keyword(Arc::clone(&state), user.clone(), engine);
    run_bash_keyword(Arc::clone(&state), user.clone(), engine);
    run_file_keyword(state, user, engine);
}
