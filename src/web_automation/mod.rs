use headless_chrome::browser::tab::Tab;
use headless_chrome::{Browser, LaunchOptions};
use std::env;
use std::error::Error;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Command;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Semaphore;

use crate::shared::utils::{download_file, extract_zip_recursive};

pub struct BrowserSetup {
    pub brave_path: String,
    pub chromedriver_path: String,
}

pub struct BrowserPool {
    browser: Browser,
    semaphore: Semaphore,
}

impl BrowserPool {
    pub async fn new(
        max_concurrent: usize,
        brave_path: String,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let options = LaunchOptions::default_builder()
            .path(Some(PathBuf::from(brave_path)))
            .args(vec![
                std::ffi::OsStr::new("--disable-gpu"),
                std::ffi::OsStr::new("--no-sandbox"),
                std::ffi::OsStr::new("--disable-dev-shm-usage"),
            ])
            .build()
            .map_err(|e| format!("Failed to build launch options: {}", e))?;

        let browser =
            Browser::new(options).map_err(|e| format!("Failed to launch browser: {}", e))?;

        Ok(Self {
            browser,
            semaphore: Semaphore::new(max_concurrent),
        })
    }

    pub async fn with_browser<F, T>(&self, f: F) -> Result<T, Box<dyn Error + Send + Sync>>
    where
        F: FnOnce(
                Arc<Tab>,
            )
                -> Pin<Box<dyn Future<Output = Result<T, Box<dyn Error + Send + Sync>>> + Send>>
            + Send
            + 'static,
        T: Send + 'static,
    {
        let _permit = self.semaphore.acquire().await?;

        let tab = self
            .browser
            .new_tab()
            .map_err(|e| format!("Failed to create new tab: {}", e))?;

        let result = f(tab.clone()).await;

        // Close the tab when done
        let _ = tab.close(true);

        result
    }
}

impl BrowserSetup {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let brave_path = Self::find_brave().await?;
        let chromedriver_path = Self::setup_chromedriver().await?;

        Ok(Self {
            brave_path,
            chromedriver_path,
        })
    }

    async fn find_brave() -> Result<String, Box<dyn std::error::Error>> {
        let mut possible_paths = vec![
            String::from(r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe"),
            String::from("/Applications/Brave Browser.app/Contents/MacOS/Brave Browser"),
            String::from("/usr/bin/brave-browser"),
            String::from("/usr/bin/brave"),
        ];

        if let Ok(local_appdata) = env::var("LOCALAPPDATA") {
            let mut path = PathBuf::from(local_appdata);
            path.push("BraveSoftware\\Brave-Browser\\Application\\brave.exe");
            possible_paths.push(path.to_string_lossy().to_string());
        }

        for path in possible_paths {
            if fs::metadata(&path).await.is_ok() {
                return Ok(path);
            }
        }

        Err("Brave browser not found. Please install Brave first.".into())
    }

    async fn setup_chromedriver() -> Result<String, Box<dyn std::error::Error>> {
        let mut chromedriver_dir = env::current_exe()?.parent().unwrap().to_path_buf();
        chromedriver_dir.push("chromedriver");

        if !chromedriver_dir.exists() {
            fs::create_dir(&chromedriver_dir).await?;
        }

        let chromedriver_path = if cfg!(target_os = "windows") {
            chromedriver_dir.join("chromedriver.exe")
        } else {
            chromedriver_dir.join("chromedriver")
        };

        if fs::metadata(&chromedriver_path).await.is_err() {
            let (download_url, platform) = match (cfg!(target_os = "windows"), cfg!(target_arch = "x86_64")) {
                (true, true) => (
                    "https://storage.googleapis.com/chrome-for-testing-public/138.0.7204.183/win64/chromedriver-win64.zip",
                    "win64",
                ),
                (true, false) => (
                    "https://storage.googleapis.com/chrome-for-testing-public/138.0.7204.183/win32/chromedriver-win32.zip",
                    "win32",
                ),
                (false, true) if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") => (
                    "https://storage.googleapis.com/chrome-for-testing-public/138.0.7204.183/mac-arm64/chromedriver-mac-arm64.zip",
                    "mac-arm64",
                ),
                (false, true) if cfg!(target_os = "macos") => (
                    "https://storage.googleapis.com/chrome-for-testing-public/138.0.7204.183/mac-x64/chromedriver-mac-x64.zip",
                    "mac-x64",
                ),
                (false, true) => (
                    "https://storage.googleapis.com/chrome-for-testing-public/138.0.7204.183/linux64/chromedriver-linux64.zip",
                    "linux64",
                ),
                _ => return Err("Unsupported platform".into()),
            };

            let mut zip_path = std::env::temp_dir();
            zip_path.push("chromedriver.zip");

            download_file(download_url, &zip_path.to_str().unwrap()).await?;

            let mut temp_extract_dir = std::env::temp_dir();
            temp_extract_dir.push("chromedriver_extract");
            let temp_extract_dir1 = temp_extract_dir.clone();

            let _ = fs::remove_dir_all(&temp_extract_dir).await;
            fs::create_dir(&temp_extract_dir).await?;

            extract_zip_recursive(&zip_path, &temp_extract_dir)?;

            let mut extracted_binary_path = temp_extract_dir;
            extracted_binary_path.push(format!("chromedriver-{}", platform));
            extracted_binary_path.push(if cfg!(target_os = "windows") {
                "chromedriver.exe"
            } else {
                "chromedriver"
            });

            match fs::rename(&extracted_binary_path, &chromedriver_path).await {
                Ok(_) => (),
                Err(e) if e.kind() == std::io::ErrorKind::CrossesDevices => {
                    fs::copy(&extracted_binary_path, &chromedriver_path).await?;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let mut perms = fs::metadata(&chromedriver_path).await?.permissions();
                        perms.set_mode(0o755);
                        fs::set_permissions(&chromedriver_path, perms).await?;
                    }
                }
                Err(e) => return Err(e.into()),
            }

            let _ = fs::remove_file(&zip_path).await;
            let _ = fs::remove_dir_all(temp_extract_dir1).await;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&chromedriver_path).await?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&chromedriver_path, perms).await?;
            }
        }

        Ok(chromedriver_path.to_string_lossy().to_string())
    }
}

pub async fn initialize_browser_pool() -> Result<Arc<BrowserPool>, Box<dyn std::error::Error>> {
    let setup = BrowserSetup::new().await?;

    // Note: headless_chrome doesn't use chromedriver, it uses Chrome DevTools Protocol directly
    // So we don't need to spawn chromedriver process

    Ok(Arc::new(BrowserPool::new(5, setup.brave_path).await?))
}

async fn is_process_running(name: &str) -> bool {
    if cfg!(target_os = "windows") {
        Command::new("tasklist")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(name))
            .unwrap_or(false)
    } else {
        Command::new("pgrep")
            .arg(name)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
