use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Output;
use std::sync::LazyLock;

static ALLOWED_COMMANDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "ffmpeg",
        "ffprobe",
        "convert",
        "bash",
        "sh",
        "python3",
        "which",
        "curl",
        "notify-send",
        "osascript",
    ])
});

static FORBIDDEN_SHELL_CHARS: LazyLock<HashSet<char>> = LazyLock::new(|| {
    HashSet::from([';', '|', '&', '$', '`', '<', '>', '\n', '\r', '\0'])
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandGuardError {
    CommandNotAllowed(String),
    InvalidArgument(String),
    PathTraversal(String),
    ExecutionFailed(String),
    ShellInjectionAttempt(String),
}

impl std::fmt::Display for CommandGuardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommandNotAllowed(cmd) => write!(f, "Command not in allowlist: {cmd}"),
            Self::InvalidArgument(arg) => write!(f, "Invalid argument: {arg}"),
            Self::PathTraversal(path) => write!(f, "Path traversal detected: {path}"),
            Self::ExecutionFailed(msg) => write!(f, "Command execution failed: {msg}"),
            Self::ShellInjectionAttempt(input) => {
                write!(f, "Shell injection attempt detected: {input}")
            }
        }
    }
}

impl std::error::Error for CommandGuardError {}

impl From<CommandGuardError> for String {
    fn from(val: CommandGuardError) -> Self {
        val.to_string()
    }
}

fn validate_argument(arg: &str) -> Result<(), CommandGuardError> {
    if arg.is_empty() {
        return Err(CommandGuardError::InvalidArgument("Empty argument".to_string()));
    }
    if arg.len() > 4096 {
        return Err(CommandGuardError::InvalidArgument("Argument too long".to_string()));
    }
    for ch in arg.chars() {
        if FORBIDDEN_SHELL_CHARS.contains(&ch) {
            return Err(CommandGuardError::ShellInjectionAttempt(format!(
                "Forbidden character '{ch}' in argument"
            )));
        }
    }
    Ok(())
}

fn normalize_path_compare(p: &std::path::Path) -> String {
    let s = p.to_string_lossy();
    let s = if s.starts_with("\\\\?\\") { s[4..].to_string() } else { s.to_string() };
    let s = if s.starts_with("Z:\\") || s.starts_with("Z:/")
        || s.starts_with("z:\\") || s.starts_with("z:/")
    {
        s.replacen("Z:\\", "/", 1).replacen("Z:/", "/", 1)
            .replacen("z:\\", "/", 1).replacen("z:/", "/", 1)
    } else {
        s
    };
    s.replace('\\', "/")
}

fn validate_path(
    path: &std::path::Path,
    allowed_paths: &[PathBuf],
) -> Result<PathBuf, CommandGuardError> {
    let canonical = path
        .to_path_buf()
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf());

    for allowed in allowed_paths {
        if canonical.starts_with(allowed) {
            return Ok(canonical);
        }
        let np = normalize_path_compare(&canonical);
        let nr = normalize_path_compare(allowed);
        if np.starts_with(&nr) {
            return Ok(canonical);
        }
    }

    if canonical.starts_with("/tmp") || canonical.starts_with("/var/tmp") {
        return Ok(canonical);
    }
    let np = normalize_path_compare(&canonical);
    if np.starts_with("/tmp") || np.starts_with("/var/tmp") {
        return Ok(canonical);
    }

    Err(CommandGuardError::PathTraversal(
        path.to_string_lossy().to_string(),
    ))
}

pub struct SafeCommand {
    command: String,
    args: Vec<String>,
    working_dir: Option<PathBuf>,
    allowed_paths: Vec<PathBuf>,
    envs: HashMap<String, String>,
    stdout_cfg: StdioConfig,
    stderr_cfg: StdioConfig,
}

#[derive(Default)]
enum StdioConfig {
    #[default]
    Inherit,
    Piped,
}


impl StdioConfig {
    fn to_stdio(&self) -> std::process::Stdio {
        match self {
            StdioConfig::Inherit => std::process::Stdio::inherit(),
            StdioConfig::Piped => std::process::Stdio::piped(),
        }
    }
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
            working_dir: None,
            allowed_paths: vec![
                PathBuf::from("/tmp"),
                PathBuf::from("/var/tmp"),
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            ],
            envs: HashMap::new(),
            stdout_cfg: StdioConfig::Inherit,
            stderr_cfg: StdioConfig::Inherit,
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
            return Err(CommandGuardError::InvalidArgument("Empty argument".to_string()));
        }
        if arg.len() > 4096 {
            return Err(CommandGuardError::InvalidArgument("Argument too long".to_string()));
        }
        let dangerous_patterns = ["$(", "`", "&&", "||", ">>", "<<", ".."];
        for pattern in dangerous_patterns {
            if arg.contains(pattern) {
                return Err(CommandGuardError::ShellInjectionAttempt(format!(
                    "Dangerous pattern '{pattern}' detected"
                )));
            }
        }
        self.args.push(arg.to_string());
        Ok(self)
    }

    pub fn shell_script_arg(mut self, script: &str) -> Result<Self, CommandGuardError> {
        let is_unix_shell = self.command == "bash" || self.command == "sh";
        if !is_unix_shell {
            return Err(CommandGuardError::InvalidArgument(
                "shell_script_arg only allowed for bash/sh commands".to_string(),
            ));
        }
        let valid_flag = self.args.last().is_some_and(|a| a == "-c");
        if !valid_flag {
            return Err(CommandGuardError::InvalidArgument(
                "shell_script_arg requires -c flag to be set first".to_string(),
            ));
        }
        if script.is_empty() {
            return Err(CommandGuardError::InvalidArgument("Empty script".to_string()));
        }
        if script.len() > 8192 {
            return Err(CommandGuardError::InvalidArgument("Script too long".to_string()));
        }
        let forbidden_patterns = ["$(", "`", ".."];
        for pattern in forbidden_patterns {
            if script.contains(pattern) {
                return Err(CommandGuardError::ShellInjectionAttempt(format!(
                    "Dangerous pattern '{pattern}' in shell script"
                )));
            }
        }
        self.args.push(script.to_string());
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

    pub fn env(mut self, key: &str, value: &str) -> Result<Self, CommandGuardError> {
        validate_argument(key)?;
        validate_argument(value)?;
        self.envs.insert(key.to_string(), value.to_string());
        Ok(self)
    }

    pub fn stdout(mut self, _stdout: std::process::Stdio) -> Self {
        self.stdout_cfg = StdioConfig::Piped;
        self
    }

    pub fn stderr(mut self, _stderr: std::process::Stdio) -> Self {
        self.stderr_cfg = StdioConfig::Piped;
        self
    }

    pub fn execute(&self) -> Result<Output, CommandGuardError> {
        let mut cmd = std::process::Command::new(&self.command);
        cmd.args(&self.args);

        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        cmd.env_clear();
        cmd.env(
            "PATH",
            "/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin",
        );
        cmd.env("HOME", dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp")));
        cmd.env("LANG", "C.UTF-8");

        for (key, value) in &self.envs {
            cmd.env(key, value);
        }

        cmd.stdout(self.stdout_cfg.to_stdio());
        cmd.stderr(self.stderr_cfg.to_stdio());

        cmd.output()
            .map_err(|e| CommandGuardError::ExecutionFailed(e.to_string()))
    }
}
