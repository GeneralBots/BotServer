use anyhow::{Context, Result};
use log::{info, warn};
use std::process::{Child, Command, Stdio};
use tokio::time::{sleep, Duration};

pub const DEFAULT_DEBUG_PORT: u16 = 9222;

pub struct BrowserService {
    port: u16,
    process: Option<Child>,
    user_data_dir: String,
}

impl BrowserService {
    pub async fn start(port: u16) -> Result<Self> {
        let _ = std::process::Command::new("pkill")
            .args(["-9", "-f", &format!("--remote-debugging-port={port}")])
            .output();
        sleep(Duration::from_millis(500)).await;

        let binary_path = Self::detect_browser_binary()?;
        let user_data_dir = format!("/tmp/browser-cdp-{}-{}", std::process::id(), port);

        let _ = std::fs::remove_dir_all(&user_data_dir);
        std::fs::create_dir_all(&user_data_dir)?;

        info!("Starting browser with CDP on port {port}");
        println!("🌐 Starting browser: {binary_path}");
        info!("  Binary: {binary_path}");
        info!("  User data: {user_data_dir}");

        let headless = std::env::var("HEADLESS").is_ok();

        let mut cmd = Command::new(&binary_path);
        cmd.arg(format!("--remote-debugging-port={port}"))
            .arg(format!("--user-data-dir={user_data_dir}"))
            .arg("--no-sandbox")
            .arg("--disable-dev-shm-usage")
            .arg("--disable-extensions")
            .arg("--disable-background-networking")
            .arg("--disable-default-apps")
            .arg("--disable-sync")
            .arg("--disable-translate")
            .arg("--metrics-recording-only")
            .arg("--no-first-run")
            .arg("--safebrowsing-disable-auto-update")
            .arg("--ignore-certificate-errors")
            .arg("--ignore-certificate-errors-spki-list")
            .arg("--ignore-ssl-errors")
            .arg("--allow-insecure-localhost")
            .arg("--allow-running-insecure-content")
            .arg("--disable-web-security")
            .arg("--reduce-security-for-testing")
            .arg("--window-position=100,100")
            .arg("--window-size=1280,800")
            .arg("--start-maximized");

        if headless {
            cmd.arg("--headless=new");
            cmd.arg("--disable-gpu");
        }

        cmd.arg("about:blank");

        cmd.stdout(Stdio::null()).stderr(Stdio::null());

        let process = cmd
            .spawn()
            .context(format!("Failed to start browser: {binary_path}"))?;

        println!("  ⏳ Waiting for CDP on port {port}...");

        let service = Self {
            port,
            process: Some(process),
            user_data_dir,
        };

        for i in 0..100 {
            sleep(Duration::from_millis(100)).await;
            if service.is_ready().await {
                info!("Browser CDP ready on port {port}");
                println!("  ✓ Browser CDP ready on port {port}");
                return Ok(service);
            }
            if i % 20 == 0 && i > 0 {
                info!("Waiting for browser CDP... attempt {}/100", i + 1);
                println!("  ... still waiting ({}/100)", i + 1);
            }
        }

        warn!("Browser may not be fully ready on CDP port {port}");
        println!("  ⚠ Browser may not be fully ready");
        Ok(service)
    }

    async fn is_ready(&self) -> bool {
        let url = format!("http://127.0.0.1:{}/json/version", self.port);
        match reqwest::get(&url).await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    fn detect_browser_binary() -> Result<String> {
        if let Ok(path) = std::env::var("BROWSER_BINARY") {
            if std::path::Path::new(&path).exists() {
                info!("Using browser from BROWSER_BINARY env var: {path}");
                return Ok(path);
            }
        }

        let brave_paths = [
            "/opt/brave.com/brave-nightly/brave",
            "/opt/brave.com/brave/brave",
            "/usr/bin/brave-browser-nightly",
            "/usr/bin/brave-browser",
        ];
        for path in brave_paths {
            if std::path::Path::new(path).exists() {
                info!("Detected Brave binary at: {path}");
                return Ok(path.to_string());
            }
        }

        let chrome_paths = [
            "/opt/google/chrome/chrome",
            "/opt/google/chrome/google-chrome",
            "/usr/bin/google-chrome-stable",
            "/usr/bin/google-chrome",
        ];
        for path in chrome_paths {
            if std::path::Path::new(path).exists() {
                info!("Detected Chrome binary at: {path}");
                return Ok(path.to_string());
            }
        }

        let chromium_paths = [
            "/usr/bin/chromium-browser",
            "/usr/bin/chromium",
            "/snap/bin/chromium",
        ];
        for path in chromium_paths {
            if std::path::Path::new(path).exists() {
                info!("Detected Chromium binary at: {path}");
                return Ok(path.to_string());
            }
        }

        anyhow::bail!("No supported browser found. Install Brave, Chrome, or Chromium.")
    }

    #[must_use]
    pub fn ws_url(&self) -> String {
        format!("ws://127.0.0.1:{}", self.port)
    }

    #[must_use]
    pub fn http_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    pub async fn stop(&mut self) -> Result<()> {
        tokio::task::yield_now().await;
        if let Some(mut process) = self.process.take() {
            info!("Stopping browser");
            process.kill().ok();
            process.wait().ok();
        }

        if std::path::Path::new(&self.user_data_dir).exists() {
            std::fs::remove_dir_all(&self.user_data_dir).ok();
        }

        Ok(())
    }

    pub fn cleanup(&mut self) {
        if let Some(mut process) = self.process.take() {
            process.kill().ok();
            process.wait().ok();
        }

        if std::path::Path::new(&self.user_data_dir).exists() {
            std::fs::remove_dir_all(&self.user_data_dir).ok();
        }
    }
}

impl Drop for BrowserService {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_browser() {
        let result = BrowserService::detect_browser_binary();
        if let Ok(path) = result {
            assert!(!path.is_empty());
        }
    }
}
