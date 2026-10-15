use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::{Child, Output};
use std::sync::LazyLock;
use super::command_validation::{validate_argument, validate_path};
pub use super::command_validation::CommandGuardError;
use super::utils::get_stack_path;

// Server-side allowlist. Desktop-only binaries (`rclone`, `osascript`, `rm`)
// are deliberately absent: the desktop shell carries its own guard, and
// `nc`/`netcat` would turn any caller into a reverse shell.
static ALLOWED_COMMANDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "pdftotext", "pandoc", "nvidia-smi", "powershell", "clamscan",
        "freshclam", "mc", "ffmpeg", "ffprobe", "convert", "gs",
        "tesseract", "which", "where", "sh", "bash", "cmd", "pkill",
        "pgrep", "kill", "fuser", "curl", "tar", "unzip",
        "pg_dump", "pg_isready", "lxc", "lxc-execute", "lxd", "docker",
        "apt-get", "brew", "rustc", "nvcc", "notify-send",
        "true", "find", "ss", "cargo",
        "redis-server", "redis-cli", "valkey-cli", "valkey-server",
        "minio", "chromedriver", "chrome", "chromium", "brave", "diesel",
        "psql", "forgejo",
        "forgejo-runner", "incus", "lynis", "rkhunter", "chkrootkit",
        "suricata", "suricata-update", "maldet", "sudo",
        "visudo", "id", "netsh", "llama-server", "ollama", "vault",
        "python", "python3", "python3.11", "python3.12",
        "tasklist", "tar.exe", "git",
    ])
});

pub struct SafeCommand {
    command: String,
    args: Vec<String>,
    raw_args: HashSet<usize>,
    working_dir: Option<PathBuf>,
    allowed_paths: Vec<PathBuf>,
    envs: HashMap<String, String>,
    stdout: Option<std::process::Stdio>,
    stderr: Option<std::process::Stdio>,
}

impl SafeCommand {
    pub fn new(command: &str) -> Result<Self, CommandGuardError> {
        let cmd_name = std::path::Path::new(command)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(command);

        if !ALLOWED_COMMANDS.contains(cmd_name) {
            return Err(CommandGuardError::CommandNotAllowed(command.to_string()));
        }

        Ok(Self {
            command: command.to_string(),
            args: Vec::new(),
            raw_args: HashSet::new(),
            working_dir: None,
            allowed_paths: vec![
                PathBuf::from("/tmp"),
                PathBuf::from("/var/tmp"),
                dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            ],
            envs: HashMap::new(),
            stdout: None,
            stderr: None,
        })
    }

    pub fn arg<S: AsRef<str>>(mut self, arg: S) -> Result<Self, CommandGuardError> {
        let arg_ref = arg.as_ref();
        validate_argument(arg_ref)?;
        self.args.push(arg_ref.to_string());
        Ok(self)
    }

    pub fn trusted_arg(mut self, arg: &str) -> Result<Self, CommandGuardError> {
        if arg.is_empty() {
            return Err(CommandGuardError::InvalidArgument(
                "Empty argument".to_string(),
            ));
        }
        if arg.len() > 4096 {
            return Err(CommandGuardError::InvalidArgument(
                "Argument too long".to_string(),
            ));
        }
        let dangerous_patterns = ["$(", "`", "&&", "||", ">>", "<<", ".."];
        for pattern in dangerous_patterns {
            if arg.contains(pattern) {
                return Err(CommandGuardError::ShellInjectionAttempt(format!(
                    "Dangerous pattern '{}' detected",
                    pattern
                )));
            }
        }
        self.args.push(arg.to_string());
        Ok(self)
    }

    pub fn shell_script_arg(mut self, script: &str) -> Result<Self, CommandGuardError> {
        let is_unix_shell = self.command == "bash" || self.command == "sh";
        let is_windows_cmd = self.command == "cmd";
        if !is_unix_shell && !is_windows_cmd {
            return Err(CommandGuardError::InvalidArgument(
                "shell_script_arg only allowed for bash/sh/cmd commands".to_string(),
            ));
        }
        let valid_flag = if is_unix_shell {
            self.args.last().is_some_and(|a| a == "-c")
        } else {
            self.args.last().is_some_and(|a| a == "/C" || a == "/c")
        };
        if !valid_flag {
            return Err(CommandGuardError::InvalidArgument(
                "shell_script_arg requires -c (unix) or /C (windows) flag to be set first"
                    .to_string(),
            ));
        }
        if script.is_empty() {
            return Err(CommandGuardError::InvalidArgument(
                "Empty script".to_string(),
            ));
        }
        if script.len() > 8192 {
            return Err(CommandGuardError::InvalidArgument(
                "Script too long".to_string(),
            ));
        }
        let forbidden_patterns = ["$(", "`", ".."];
        for pattern in forbidden_patterns {
            if script.contains(pattern) {
                return Err(CommandGuardError::ShellInjectionAttempt(format!(
                    "Dangerous pattern '{}' in shell script",
                    pattern
                )));
            }
        }
        self.args.push(script.to_string());
        Ok(self)
    }

    pub fn trusted_shell_script_arg(self, script: &str) -> Result<Self, CommandGuardError> {
        self.shell_script_arg_internal(script, true)
    }

    /// Same as [`trusted_shell_script_arg`], but the script argument is passed
    /// verbatim to the shell without Rust's Windows quoting/escaping. Required
    /// for `cmd /C` scripts that contain embedded quotes (e.g. `"C:\path\prog"`),
    /// which Rust's default argument encoding would otherwise escape as `\"`.
    pub fn raw_shell_script_arg(self, script: &str) -> Result<Self, CommandGuardError> {
        self.shell_script_arg_internal(script, true)?
            .mark_last_arg_raw()
    }

    fn shell_script_arg_internal(
        mut self,
        script: &str,
        trusted: bool,
    ) -> Result<Self, CommandGuardError> {
        let is_unix_shell = self.command == "bash" || self.command == "sh";
        let is_windows_cmd = self.command == "cmd";
        if !is_unix_shell && !is_windows_cmd {
            return Err(CommandGuardError::InvalidArgument(
                "shell_script_arg only allowed for bash/sh/cmd commands".to_string(),
            ));
        }
        let valid_flag = if is_unix_shell {
            self.args.last().is_some_and(|a| a == "-c")
        } else {
            self.args.last().is_some_and(|a| a == "/C" || a == "/c")
        };
        if !valid_flag {
            return Err(CommandGuardError::InvalidArgument(
                "shell_script_arg requires -c (unix) or /C (windows) flag to be set first"
                    .to_string(),
            ));
        }
        if script.is_empty() {
            return Err(CommandGuardError::InvalidArgument(
                "Empty script".to_string(),
            ));
        }
        let limit = if trusted { 16384 } else { 8192 };
        if script.len() > limit {
            return Err(CommandGuardError::InvalidArgument(
                "Script too long".to_string(),
            ));
        }
        if !trusted {
            let forbidden_patterns = ["$(", "`", ".."];
            for pattern in forbidden_patterns {
                if script.contains(pattern) {
                    return Err(CommandGuardError::ShellInjectionAttempt(format!(
                        "Dangerous pattern '{}' in shell script",
                        pattern
                    )));
                }
            }
        }
        self.args.push(script.to_string());
        Ok(self)
    }

    fn append_args(&self, cmd: &mut std::process::Command) {
        #[cfg(target_os = "windows")]
        for (idx, arg) in self.args.iter().enumerate() {
            use std::os::windows::process::CommandExt;
            if self.raw_args.contains(&idx) {
                cmd.raw_arg(arg);
                continue;
            }
            cmd.arg(arg);
        }
        #[cfg(not(target_os = "windows"))]
        for arg in self.args.iter() {
            cmd.arg(arg);
        }
    }

    fn mark_last_arg_raw(mut self) -> Result<Self, CommandGuardError> {
        let idx = self
            .args
            .len()
            .checked_sub(1)
            .ok_or(CommandGuardError::InvalidArgument("No args".to_string()))?;
        self.raw_args.insert(idx);
        Ok(self)
    }

    pub fn args(mut self, args: &[&str]) -> Result<Self, CommandGuardError> {
        for arg in args {
            validate_argument(arg)?;
            self.args.push((*arg).to_string());
        }
        Ok(self)
    }

    pub fn path_arg(mut self, path: &std::path::Path) -> Result<Self, CommandGuardError> {
        let validated_path = validate_path(path, &self.allowed_paths)?;
        self.args.push(validated_path.to_string_lossy().to_string());
        Ok(self)
    }

    pub fn working_dir(mut self, dir: &std::path::Path) -> Result<Self, CommandGuardError> {
        let validated = validate_path(dir, &self.allowed_paths)?;
        self.working_dir = Some(validated);
        Ok(self)
    }

    pub fn allow_path(mut self, path: PathBuf) -> Self {
        self.allowed_paths.push(path);
        self
    }

    pub fn env(mut self, key: &str, value: &str) -> Result<Self, CommandGuardError> {
        validate_argument(key)?;
        validate_argument(value)?;
        self.envs.insert(key.to_string(), value.to_string());
        Ok(self)
    }

    pub fn stdout(mut self, stdout: std::process::Stdio) -> Self {
        self.stdout = Some(stdout);
        self
    }

    pub fn stderr(mut self, stderr: std::process::Stdio) -> Self {
        self.stderr = Some(stderr);
        self
    }

    fn build_path_env(&self) -> String {
        #[cfg(target_os = "windows")]
        let separator = ";";
        #[cfg(not(target_os = "windows"))]
        let separator = ":";

        #[cfg(target_os = "windows")]
        let mut path_entries: Vec<String> = vec![
            std::env::var("PATH").unwrap_or_default(),
        ];

        #[cfg(not(target_os = "windows"))]
        let mut path_entries = vec![
            "/snap/bin".to_string(),
            "/usr/local/bin".to_string(),
            "/usr/bin".to_string(),
            "/bin".to_string(),
            "/usr/sbin".to_string(),
            "/sbin".to_string(),
        ];

        let stack_path = get_stack_path();
        #[cfg(not(target_os = "windows"))]
        let shared_bin = format!("{}/bin/shared", stack_path);
        #[cfg(target_os = "windows")]
        let shared_bin = format!("{}\\bin\\shared", stack_path);
        if std::path::Path::new(&shared_bin).exists() {
            path_entries.insert(0, shared_bin);
        }

        let component_bins = [
            format!("{}/bin/cache/bin", stack_path),
            format!("{}/bin/tables/bin", stack_path),
            format!("{}/bin/vault", stack_path),
            format!("{}/bin/drive", stack_path),
            format!("{}/bin/directory", stack_path),
        ];
        for bin_dir in component_bins {
            let normalised = bin_dir.replace('/', std::path::MAIN_SEPARATOR_STR);
            if std::path::Path::new(&normalised).exists() {
                path_entries.insert(0, normalised);
            }
        }

        path_entries.join(separator)
    }

    fn apply_common_env(&self, cmd: &mut std::process::Command) {
        cmd.env_clear();
        cmd.env("PATH", self.build_path_env());

        #[cfg(target_os = "windows")]
        {
            cmd.env(
                "USERPROFILE",
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("C:\\Users\\Default"))
                    .to_string_lossy()
                    .to_string(),
            );
            cmd.env(
                "SYSTEMROOT",
                std::env::var("SYSTEMROOT").unwrap_or_else(|_| "C:\\Windows".into()),
            );
        }

        #[cfg(not(target_os = "windows"))]
        {
            cmd.env(
                "HOME",
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
                    .to_string_lossy()
                    .to_string(),
            );
            cmd.env("LANG", "C.UTF-8");
        }

        for (key, value) in &self.envs {
            cmd.env(key, value);
        }
    }

}

mod part2;

#[cfg(test)]
#[path = "command_guard_tests.rs"]
mod tests;
