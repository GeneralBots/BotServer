use super::*;

impl TerminalSession {
    pub fn new(session_id: &str) -> Self {
        let container_name = format!(
            "term-{}",
            session_id.chars().take(12).collect::<String>()
        );

        let (output_tx, output_rx) = mpsc::channel(100);

        Self {
            session_id: session_id.to_string(),
            container_name,
            process: None,
            stdin: None,
            output_tx,
            output_rx: Some(output_rx),
        }
    }

    pub fn take_output_receiver(&mut self) -> Option<mpsc::Receiver<TerminalOutput>> {
        self.output_rx.take()
    }

    pub async fn start(&mut self) -> Result<(), String> {
        if !self.container_name.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err("Invalid container name".to_string());
        }

        info!("Starting LXC container: {}", self.container_name);

        let safe_cmd = SafeCommand::new("lxc").map_err(|e| format!("{}", e))?;
        let safe_cmd = safe_cmd.args(&["launch", "ubuntu:22.04", &self.container_name, "-e"]).map_err(|e| format!("{}", e))?;
        let launch_output = safe_cmd.execute_async().await
            .map_err(|e| format!("Failed to launch container: {}", e))?;

        if !launch_output.status.success() {
            let stderr = String::from_utf8_lossy(&launch_output.stderr);
            if !stderr.contains("already exists") && !stderr.contains("is already running") {
                warn!("Container launch warning: {}", stderr);
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        info!("Starting bash shell in container: {}", self.container_name);

        // SafeCommand doesn't support async piped I/O for interactive terminals.
        // Security: container_name is validated (alphanumeric + dash only), commands run
        // inside an isolated LXC container, not on the host.
        let mut child = tokio::process::Command::new("lxc")
            .args(["exec", &self.container_name, "--", "bash", "-l"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn bash: {}", e))?;

        let stdin = child.stdin.take().ok_or("Failed to capture stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
        let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

        self.stdin = Some(Arc::new(Mutex::new(stdin)));
        self.process = Some(child);

        let tx = self.output_tx.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if tx.send(TerminalOutput::Stdout(format!("{}\r\n", line))).await.is_err() {
                    break;
                }
            }
        });

        let tx = self.output_tx.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if tx.send(TerminalOutput::Stderr(format!("{}\r\n", line))).await.is_err() {
                    break;
                }
            }
        });

        let tx = self.output_tx.clone();
        tx.send(TerminalOutput::System(
            "Container started. Welcome to your isolated terminal!\r\n".to_string(),
        ))
        .await
        .ok();

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        self.send_command("export TERM=xterm-256color; clear").await?;

        Ok(())
    }

    pub async fn send_command(&self, cmd: &str) -> Result<(), String> {
        if let Some(stdin_mutex) = &self.stdin {
            let mut stdin = stdin_mutex.lock().await;
            let cmd_with_newline = format!("{}\n", cmd);
            stdin
                .write_all(cmd_with_newline.as_bytes())
                .await
                .map_err(|e| format!("Failed to write to stdin: {}", e))?;
            stdin
                .flush()
                .await
                .map_err(|e| format!("Failed to flush stdin: {}", e))?;
        }
        Ok(())
    }

    pub async fn resize(&self, _cols: u16, _rows: u16) -> Result<(), String> {
        Ok(())
    }

    pub async fn kill(&mut self) -> Result<(), String> {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill().await;
        }

        let safe_cmd = SafeCommand::new("lxc").map_err(|e| format!("{}", e))?;
        let _ = safe_cmd.args(&["stop", &self.container_name, "-f"]).map_err(|e| format!("{}", e))?
            .execute_async().await;

        let safe_cmd = SafeCommand::new("lxc").map_err(|e| format!("{}", e))?;
        let _ = safe_cmd.args(&["delete", &self.container_name, "-f"]).map_err(|e| format!("{}", e))?
            .execute_async().await;

        info!("Container {} destroyed", self.container_name);
        Ok(())
    }
}
