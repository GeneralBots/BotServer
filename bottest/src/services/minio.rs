use super::{check_tcp_port, ensure_dir, wait_for, HEALTH_CHECK_INTERVAL, HEALTH_CHECK_TIMEOUT};
use anyhow::{Context, Result};
#[cfg(unix)]
use nix::sys::signal::{kill, Signal};
#[cfg(unix)]
use nix::unistd::Pid;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

pub struct MinioService {
    api_port: u16,
    console_port: u16,
    data_dir: PathBuf,
    bin_path: PathBuf,
    process: Option<Child>,
    access_key: String,
    secret_key: String,
}

mod part2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_format() {
        let service = MinioService {
            api_port: 9000,
            console_port: 10000,
            data_dir: PathBuf::from("/tmp/test"),
            bin_path: PathBuf::from("/tmp/minio"),
            process: None,
            access_key: "test".to_string(),
            secret_key: "secret".to_string(),
        };

        assert_eq!(service.endpoint(), "http://127.0.0.1:9000");
        assert_eq!(service.console_url(), "http://127.0.0.1:10000");
    }

    #[test]
    fn test_credentials() {
        let service = MinioService {
            api_port: 9000,
            console_port: 10000,
            data_dir: PathBuf::from("/tmp/test"),
            bin_path: PathBuf::from("/tmp/minio"),
            process: None,
            access_key: "mykey".to_string(),
            secret_key: "mysecret".to_string(),
        };

        let (key, secret) = service.credentials();
        assert_eq!(key, "mykey");
        assert_eq!(secret, "mysecret");
    }

    #[test]
    fn test_s3_config() {
        let service = MinioService {
            api_port: 9000,
            console_port: 10000,
            data_dir: PathBuf::from("/tmp/test"),
            bin_path: PathBuf::from("/tmp/minio"),
            process: None,
            access_key: "access".to_string(),
            secret_key: "secret".to_string(),
        };

        let config = service.s3_config();
        assert_eq!(
            config.get("endpoint_url"),
            Some(&"http://127.0.0.1:9000".to_string())
        );
        assert_eq!(config.get("access_key_id"), Some(&"access".to_string()));
        assert_eq!(config.get("force_path_style"), Some(&"true".to_string()));
    }
}
