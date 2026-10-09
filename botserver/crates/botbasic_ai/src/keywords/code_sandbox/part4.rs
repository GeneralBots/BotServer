use super::*;

impl ExecutionResult {
    pub fn success(stdout: String, stderr: String, execution_time_ms: u64) -> Self {
        Self {
            stdout,
            stderr,
            exit_code: 0,
            execution_time_ms,
            timed_out: false,
            killed: false,
            error: None,
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: -1,
            execution_time_ms: 0,
            timed_out: false,
            killed: false,
            error: Some(message.to_string()),
        }
    }

    pub fn timeout() -> Self {
        Self {
            stdout: String::new(),
            stderr: "Execution timed out".to_string(),
            exit_code: -1,
            execution_time_ms: 0,
            timed_out: true,
            killed: true,
            error: Some("Execution exceeded time limit".to_string()),
        }
    }

    pub fn is_success(&self) -> bool {
        self.exit_code == 0 && self.error.is_none() && !self.timed_out
    }

    pub fn output(&self) -> String {
        if self.is_success() {
            self.stdout.clone()
        } else if let Some(err) = &self.error {
            format!("Error: {}\n{}", err, self.stderr)
        } else {
            format!("Error (exit code {}): {}", self.exit_code, self.stderr)
        }
    }
}
