use anyhow::{Context, Result};
use chromiumoxide::browser::{Browser as CdpBrowser, BrowserConfig as CdpBrowserConfig};
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::page::Page;
use chromiumoxide::Element as CdpElement;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;

use super::{Cookie, Key, Locator, WaitCondition};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum BrowserType {
    #[default]
    Chrome,
    Firefox,
    Safari,
    Edge,
}

impl BrowserType {
    #[must_use]
    pub const fn browser_name(self) -> &'static str {
        match self {
            Self::Chrome => "chrome",
            Self::Firefox => "firefox",
            Self::Safari => "safari",
            Self::Edge => "MicrosoftEdge",
        }
    }

    #[must_use]
    pub const fn capability_name(self) -> &'static str {
        match self {
            Self::Chrome => "goog:chromeOptions",
            Self::Firefox => "moz:firefoxOptions",
            Self::Safari => "safari:options",
            Self::Edge => "ms:edgeOptions",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BrowserConfig {
    pub browser_type: BrowserType,
    pub debug_port: u16,
    pub headless: bool,
    pub window_width: u32,
    pub window_height: u32,
    pub timeout: Duration,
    pub binary_path: Option<String>,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        let binary_path = Self::detect_browser_binary();

        let headless = std::env::var("HEADLESS").is_ok();

        Self {
            browser_type: BrowserType::Chrome,
            debug_port: 9222,
            headless,
            window_width: 1920,
            window_height: 1080,
            timeout: Duration::from_secs(30),
            binary_path,
        }
    }
}

impl BrowserConfig {
    fn detect_browser_binary() -> Option<String> {
        if let Ok(path) = std::env::var("BROWSER_BINARY") {
            if std::path::Path::new(&path).exists() {
                log::info!("Using browser from BROWSER_BINARY env var: {path}");
                return Some(path);
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
                log::info!("Detected Brave binary at: {path}");
                return Some(path.to_string());
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
                log::info!("Detected Chrome binary at: {path}");
                return Some(path.to_string());
            }
        }

        let chromium_paths = [
            "/usr/bin/chromium-browser",
            "/usr/bin/chromium",
            "/snap/bin/chromium",
        ];
        for path in chromium_paths {
            if std::path::Path::new(path).exists() {
                log::info!("Detected Chromium binary at: {path}");
                return Some(path.to_string());
            }
        }

        None
    }

    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn with_browser(mut self, browser: BrowserType) -> Self {
        self.browser_type = browser;
        self
    }

    #[must_use]
    pub const fn with_debug_port(mut self, port: u16) -> Self {
        self.debug_port = port;
        self
    }

    #[must_use]
    pub fn with_webdriver_url(mut self, url: &str) -> Self {
        if let Some(port_str) = url.split(':').next_back() {
            if let Ok(port) = port_str.parse() {
                self.debug_port = port;
            }
        }
        self
    }

    #[must_use]
    pub const fn headless(mut self, headless: bool) -> Self {
        self.headless = headless;
        self
    }

    #[must_use]
    pub const fn with_window_size(mut self, width: u32, height: u32) -> Self {
        self.window_width = width;
        self.window_height = height;
        self
    }

    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    #[must_use]
    pub const fn with_arg(self, _arg: &str) -> Self {
        self
    }

    #[must_use]
    pub fn with_binary(mut self, path: &str) -> Self {
        self.binary_path = Some(path.to_string());
        self
    }

    pub fn build_cdp_config(&self) -> Result<CdpBrowserConfig> {
        let mut builder = CdpBrowserConfig::builder();

        if let Some(ref binary) = self.binary_path {
            builder = builder.chrome_executable(binary);
        }

        if self.headless {
            builder = builder.arg("--headless=new");
        }

        builder = builder
            .arg("--no-sandbox")
            .arg("--disable-dev-shm-usage")
            .arg("--disable-extensions")
            .arg(format!(
                "--window-size={},{}",
                self.window_width, self.window_height
            ))
            .port(self.debug_port);

        builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build CDP browser config: {e}"))
    }

    #[must_use]
    pub fn build_capabilities(&self) -> serde_json::Value {
        serde_json::json!({
            "browserName": self.browser_type.browser_name(),
            "acceptInsecureCerts": true,
        })
    }
}

pub struct Browser {
    cdp: Arc<CdpBrowser>,
    page: Arc<Mutex<Page>>,
    config: BrowserConfig,
    _handle: tokio::task::JoinHandle<()>,
}

mod part2;
mod part3;
pub use part3::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_config_default() {
        let config = BrowserConfig::default();
        assert_eq!(config.browser_type, BrowserType::Chrome);
        assert_eq!(config.debug_port, 9222);
        assert_eq!(config.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_browser_config_builder() {
        let config = BrowserConfig::new()
            .with_debug_port(9333)
            .headless(false)
            .with_window_size(1280, 720)
            .with_timeout(Duration::from_secs(60));

        assert_eq!(config.debug_port, 9333);
        assert!(!config.headless);
        assert_eq!(config.window_width, 1280);
        assert_eq!(config.window_height, 720);
        assert_eq!(config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_browser_type_browser_name() {
        assert_eq!(BrowserType::Chrome.browser_name(), "chrome");
        assert_eq!(BrowserType::Firefox.browser_name(), "firefox");
        assert_eq!(BrowserType::Safari.browser_name(), "safari");
        assert_eq!(BrowserType::Edge.browser_name(), "MicrosoftEdge");
    }
}
