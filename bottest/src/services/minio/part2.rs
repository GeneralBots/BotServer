//! Split from `minio.rs` (#1504).

use super::*;

impl MinioService {
    pub const DEFAULT_ACCESS_KEY: &'static str = "minioadmin";

    pub const DEFAULT_SECRET_KEY: &'static str = "minioadmin";

    pub(crate) fn find_minio_binary() -> Result<PathBuf> {
        if let Ok(stack_path) = std::env::var("BOTSERVER_STACK_PATH") {
            let minio_path = PathBuf::from(&stack_path).join("bin/drive/minio");
            if minio_path.exists() {
                log::info!(
                    "Using MinIO from BOTSERVER_STACK_PATH: {}",
                    minio_path.display()
                );
                return Ok(minio_path);
            }
        }

        let cwd = std::env::current_dir().unwrap_or_default();
        let relative_paths = [
            "../botserver/botserver-stack/bin/drive/minio",
            "botserver/botserver-stack/bin/drive/minio",
            "botserver-stack/bin/drive/minio",
        ];

        for rel_path in &relative_paths {
            let minio_path = cwd.join(rel_path);
            if minio_path.exists() {
                log::info!("Using MinIO from botserver-stack: {}", minio_path.display());
                return Ok(minio_path);
            }
        }

        let system_paths = [
            "/usr/local/bin/minio",
            "/usr/bin/minio",
            "/opt/minio/minio",
            "/opt/homebrew/bin/minio",
        ];

        for path in &system_paths {
            let minio_path = PathBuf::from(path);
            if minio_path.exists() {
                log::info!("Using system MinIO from: {}", minio_path.display());
                return Ok(minio_path);
            }
        }

        if let Ok(minio_path) = which::which("minio") {
            log::info!("Using MinIO from PATH: {}", minio_path.display());
            return Ok(minio_path);
        }

        anyhow::bail!("MinIO not found. Install MinIO or set BOTSERVER_STACK_PATH env var")
    }

    pub async fn start(api_port: u16, data_dir: &str) -> Result<Self> {
        let bin_path = Self::find_minio_binary()?;
        log::info!("Using MinIO from: {}", bin_path.display());

        let data_path = PathBuf::from(data_dir).join("minio");
        ensure_dir(&data_path)?;

        let console_port = api_port + 1000;

        let mut service = Self {
            api_port,
            console_port,
            data_dir: data_path,
            bin_path,
            process: None,
            access_key: Self::DEFAULT_ACCESS_KEY.to_string(),
            secret_key: Self::DEFAULT_SECRET_KEY.to_string(),
        };

        service.start_server()?;
        service.wait_ready().await?;

        Ok(service)
    }

    pub async fn start_with_credentials(
        api_port: u16,
        data_dir: &str,
        access_key: &str,
        secret_key: &str,
    ) -> Result<Self> {
        let bin_path = Self::find_minio_binary()?;
        log::info!("Using MinIO from: {}", bin_path.display());

        let data_path = PathBuf::from(data_dir).join("minio");
        ensure_dir(&data_path)?;

        let console_port = api_port + 1000;

        let mut service = Self {
            api_port,
            console_port,
            data_dir: data_path,
            bin_path,
            process: None,
            access_key: access_key.to_string(),
            secret_key: secret_key.to_string(),
        };

        service.start_server()?;
        service.wait_ready().await?;

        Ok(service)
    }

    pub(crate) fn start_server(&mut self) -> Result<()> {
        log::info!(
            "Starting MinIO on port {} (console: {})",
            self.api_port,
            self.console_port
        );

        let child = Command::new(&self.bin_path)
            .args([
                "server",
                self.data_dir.to_str().unwrap(),
                "--address",
                &format!("127.0.0.1:{}", self.api_port),
                "--console-address",
                &format!("127.0.0.1:{}", self.console_port),
            ])
            .env("MINIO_ROOT_USER", &self.access_key)
            .env("MINIO_ROOT_PASSWORD", &self.secret_key)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to start MinIO")?;

        self.process = Some(child);
        Ok(())
    }

    pub(crate) async fn wait_ready(&self) -> Result<()> {
        log::info!("Waiting for MinIO to be ready...");

        wait_for(HEALTH_CHECK_TIMEOUT, HEALTH_CHECK_INTERVAL, || async {
            check_tcp_port("127.0.0.1", self.api_port).await
        })
        .await
        .context("MinIO failed to start in time")?;

        let health_url = format!("http://127.0.0.1:{}/minio/health/live", self.api_port);
        for _ in 0..30 {
            if let Ok(resp) = reqwest::get(&health_url).await {
                if resp.status().is_success() {
                    return Ok(());
                }
            }
            sleep(Duration::from_millis(100)).await;
        }

        Ok(())
    }

    pub async fn create_bucket(&self, name: &str) -> Result<()> {
        log::info!("Creating bucket '{name}'");

        if let Ok(mc) = Self::find_mc_binary() {
            let alias_name = format!("test{}", self.api_port);
            let _ = Command::new(&mc)
                .args([
                    "alias",
                    "set",
                    &alias_name,
                    &self.endpoint(),
                    &self.access_key,
                    &self.secret_key,
                ])
                .output();

            let output = Command::new(&mc)
                .args(["mb", "--ignore-existing", &format!("{alias_name}/{name}")])
                .output()?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.contains("already") {
                    anyhow::bail!("Failed to create bucket: {stderr}");
                }
            }

            return Ok(());
        }

        let url = format!("{}/{}", self.endpoint(), name);
        let client = reqwest::Client::new();
        let resp = client
            .put(&url)
            .basic_auth(&self.access_key, Some(&self.secret_key))
            .send()
            .await?;

        if !resp.status().is_success() && resp.status().as_u16() != 409 {
            anyhow::bail!("Failed to create bucket: {}", resp.status());
        }

        Ok(())
    }

    pub async fn put_object(&self, bucket: &str, key: &str, data: &[u8]) -> Result<()> {
        log::debug!("Putting object '{}/{}' ({} bytes)", bucket, key, data.len());

        let url = format!("{}/{}/{}", self.endpoint(), bucket, key);
        let client = reqwest::Client::new();
        let resp = client
            .put(&url)
            .basic_auth(&self.access_key, Some(&self.secret_key))
            .body(data.to_vec())
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Failed to put object: {}", resp.status());
        }

        Ok(())
    }

    pub async fn get_object(&self, bucket: &str, key: &str) -> Result<Vec<u8>> {
        log::debug!("Getting object '{bucket}/{key}'");

        let url = format!("{}/{}/{}", self.endpoint(), bucket, key);
        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .basic_auth(&self.access_key, Some(&self.secret_key))
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Failed to get object: {}", resp.status());
        }

        Ok(resp.bytes().await?.to_vec())
    }

    pub async fn delete_object(&self, bucket: &str, key: &str) -> Result<()> {
        log::debug!("Deleting object '{bucket}/{key}'");

        let url = format!("{}/{}/{}", self.endpoint(), bucket, key);
        let client = reqwest::Client::new();
        let resp = client
            .delete(&url)
            .basic_auth(&self.access_key, Some(&self.secret_key))
            .send()
            .await?;

        if !resp.status().is_success() && resp.status().as_u16() != 404 {
            anyhow::bail!("Failed to delete object: {}", resp.status());
        }

        Ok(())
    }

    pub async fn list_objects(&self, bucket: &str, prefix: Option<&str>) -> Result<Vec<String>> {
        log::debug!("Listing objects in bucket '{bucket}'");

        let mut url = format!("{}/{}", self.endpoint(), bucket);
        if let Some(p) = prefix {
            url = format!("{url}?prefix={p}");
        }

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .basic_auth(&self.access_key, Some(&self.secret_key))
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Failed to list objects: {}", resp.status());
        }

        let body = resp.text().await?;
        let mut objects = Vec::new();

        for line in body.lines() {
            if let Some(start) = line.find("<Key>") {
                if let Some(end) = line.find("</Key>") {
                    let key = &line[start + 5..end];
                    objects.push(key.to_string());
                }
            }
        }

        Ok(objects)
    }

    pub async fn bucket_exists(&self, name: &str) -> Result<bool> {
        let url = format!("{}/{}", self.endpoint(), name);
        let client = reqwest::Client::new();
        let resp = client
            .head(&url)
            .basic_auth(&self.access_key, Some(&self.secret_key))
            .send()
            .await?;

        Ok(resp.status().is_success())
    }

    pub async fn delete_bucket(&self, name: &str) -> Result<()> {
        log::info!("Deleting bucket '{name}'");

        let url = format!("{}/{}", self.endpoint(), name);
        let client = reqwest::Client::new();
        let resp = client
            .delete(&url)
            .basic_auth(&self.access_key, Some(&self.secret_key))
            .send()
            .await?;

        if !resp.status().is_success() && resp.status().as_u16() != 404 {
            anyhow::bail!("Failed to delete bucket: {}", resp.status());
        }

        Ok(())
    }

    #[must_use]
    pub fn endpoint(&self) -> String {
        format!("http://127.0.0.1:{}", self.api_port)
    }

    #[must_use]
    pub fn console_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.console_port)
    }

    #[must_use]
    pub const fn api_port(&self) -> u16 {
        self.api_port
    }

    #[must_use]
    pub const fn console_port(&self) -> u16 {
        self.console_port
    }

    #[must_use]
    pub fn credentials(&self) -> (String, String) {
        (self.access_key.clone(), self.secret_key.clone())
    }

    #[must_use]
    pub fn s3_config(&self) -> HashMap<String, String> {
        let mut config = HashMap::new();
        config.insert("endpoint_url".to_string(), self.endpoint());
        config.insert("access_key_id".to_string(), self.access_key.clone());
        config.insert("secret_access_key".to_string(), self.secret_key.clone());
        config.insert("region".to_string(), "us-east-1".to_string());
        config.insert("force_path_style".to_string(), "true".to_string());
        config
    }

    pub(crate) fn find_mc_binary() -> Result<PathBuf> {
        let common_paths = ["/usr/local/bin/mc", "/usr/bin/mc", "/opt/homebrew/bin/mc"];

        for path in common_paths {
            let p = PathBuf::from(path);
            if p.exists() {
                return Ok(p);
            }
        }

        which::which("mc").context("mc binary not found")
    }

    pub async fn stop(&mut self) -> Result<()> {
        if let Some(ref mut child) = self.process {
            log::info!("Stopping MinIO...");

            #[cfg(unix)]
            {
                let pid = Pid::from_raw(child.id() as i32);
                let _ = kill(pid, Signal::SIGTERM);
            }
            #[cfg(not(unix))]
            let _ = child.kill();

            for _ in 0..50 {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        self.process = None;
                        return Ok(());
                    }
                    Ok(None) => sleep(Duration::from_millis(100)).await,
                    Err(_) => break,
                }
            }

            #[cfg(unix)]
            {
                let pid = Pid::from_raw(child.id() as i32);
                let _ = kill(pid, Signal::SIGKILL);
            }
            #[cfg(not(unix))]
            let _ = child.kill();
            let _ = child.wait();
            self.process = None;
        }

        Ok(())
    }

    pub fn cleanup(&self) -> Result<()> {
        if self.data_dir.exists() {
            std::fs::remove_dir_all(&self.data_dir)?;
        }
        Ok(())
    }
}

impl Drop for MinioService {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.process {
            #[cfg(unix)]
            {
                let pid = Pid::from_raw(child.id() as i32);
                let _ = kill(pid, Signal::SIGTERM);
            }
            #[cfg(not(unix))]
            let _ = child.kill();

            std::thread::sleep(Duration::from_millis(500));

            #[cfg(unix)]
            {
                let pid = Pid::from_raw(child.id() as i32);
                let _ = kill(pid, Signal::SIGKILL);
            }
            #[cfg(not(unix))]
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
