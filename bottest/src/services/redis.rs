use super::{check_tcp_port, ensure_dir, wait_for, HEALTH_CHECK_INTERVAL, HEALTH_CHECK_TIMEOUT};
use anyhow::{Context, Result};
#[cfg(unix)]
use nix::sys::signal::{kill, Signal};
#[cfg(unix)]
use nix::unistd::Pid;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

pub struct RedisService {
    port: u16,
    data_dir: PathBuf,
    process: Option<Child>,
    password: Option<String>,
}

mod part2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_string_no_password() {
        let service = RedisService {
            port: 6379,
            data_dir: PathBuf::from("/tmp/test"),
            process: None,
            password: None,
        };

        assert_eq!(service.connection_string(), "redis://127.0.0.1:6379");
    }

    #[test]
    fn test_connection_string_with_password() {
        let service = RedisService {
            port: 6379,
            data_dir: PathBuf::from("/tmp/test"),
            process: None,
            password: Some("secret123".to_string()),
        };

        assert_eq!(
            service.connection_string(),
            "redis://:secret123@127.0.0.1:6379"
        );
    }

    #[test]
    fn test_host_port() {
        let service = RedisService {
            port: 16379,
            data_dir: PathBuf::from("/tmp/test"),
            process: None,
            password: None,
        };

        assert_eq!(service.host_port(), ("127.0.0.1", 16379));
    }
}
