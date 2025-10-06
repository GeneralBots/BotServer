// wget https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb
// sudo dpkg -i google-chrome-stable_current_amd64.deb
use log::info;

use std::env;
use std::error::Error;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Command;
use std::sync::Arc;
use thirtyfour::{ChromiumLikeCapabilities, DesiredCapabilities, WebDriver};
use tokio::fs;
use tokio::sync::Semaphore;

use crate::shared::utils::{download_file, extract_zip_recursive};

pub struct BrowserSetup {
    pub brave_path: String,
    pub chromedriver_path: String,
}

pub struct BrowserPool {
    webdriver_url: String,
    semaphore: Semaphore,
    brave_path: String,
}

impl BrowserPool {
    pub fn new(webdriver_url: String, max_concurrent: usize, brave_path: String) -> Self {
        Self {
            webdriver_url,
            semaphore: Semaphore::new(max_concurrent),
            brave_path,
        }
    }

    pub async fn with_browser<F, T>(&self, f: F) -> Result<T, Box<dyn Error + Send + Sync>>
    where
        F: FnOnce(
                WebDriver,
            )
                -> Pin<Box<dyn Future<Output = Result<T, Box<dyn Error + Send + Sync>>> + Send>>
            + Send
            + 'static,
        T: Send + 'static,
    {
        // Acquire a permit to respect the concurrency limit
        let _permit = self.semaphore.acquire().await?;

        // Build Chrome/Brave capabilities
        let mut caps = DesiredCapabilities::chrome();
        caps.set_binary(&self.brave_path)?;
        // caps.add_arg("--headless=new")?; // Uncomment if headless mode is desired
        caps.add_arg("--disable-gpu")?;
        caps.add_arg("--no-sandbox")?;

        // Create a new WebDriver instance
        let driver = WebDriver::new(&self.webdriver_url, caps).await?;

        // Execute the user‑provided async function with the driver
        let result = f(driver).await;

        result
    }
}

impl BrowserSetup {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Check for Brave installation
        let brave_path = Self::find_brave().await?;

        // Check for chromedriver
        let chromedriver_path = Self::setup_chromedriver().await?;

        Ok(Self {
            brave_path,
            chromedriver_path,
        })
    }

    async fn find_brave() -> Result<String, Box<dyn std::error::Error>> {
        let mut possible_paths = vec![
            // Windows - Program Files
            String::from(r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe"),
            // macOS
            String::from("/Applications/Brave Browser.app/Contents/MacOS/Brave Browser"),
            // Linux
            String::from("/usr/bin/brave-browser"),
            String::from("/usr/bin/brave"),
        ];

        // Windows - AppData (usuário atual)
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
        // Create chromedriver directory in executable's parent directory
        let mut chromedriver_dir = env::current_exe()?.parent().unwrap().to_path_buf();
        chromedriver_dir.push("chromedriver");

        // Ensure the directory exists
        if !chromedriver_dir.exists() {
            fs::create_dir(&chromedriver_dir).await?;
        }

        // Determine the final chromedriver path
        let chromedriver_path = if cfg!(target_os = "windows") {
            chromedriver_dir.join("chromedriver.exe")
        } else {
            chromedriver_dir.join("chromedriver")
        };

        // Check if chromedriver exists
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
            info!("Downloading chromedriver for {}...", platform);

            // Download the zip file
            download_file(download_url, &zip_path.to_str().unwrap()).await?;

            // Extract the zip to a temporary directory first
            let mut temp_extract_dir = std::env::temp_dir();
            temp_extract_dir.push("chromedriver_extract");
            let temp_extract_dir1 = temp_extract_dir.clone();

            // Clean up any previous extraction
            let _ = fs::remove_dir_all(&temp_extract_dir).await;
            fs::create_dir(&temp_extract_dir).await?;

            extract_zip_recursive(&zip_path, &temp_extract_dir)?;

            // Chrome for Testing zips contain a platform-specific directory
            // Find the chromedriver binary in the extracted structure
            let mut extracted_binary_path = temp_extract_dir;
            extracted_binary_path.push(format!("chromedriver-{}", platform));
            extracted_binary_path.push(if cfg!(target_os = "windows") {
                "chromedriver.exe"
            } else {
                "chromedriver"
            });

            // Try to move the file, fall back to copy if cross-device
            match fs::rename(&extracted_binary_path, &chromedriver_path).await {
                Ok(_) => (),
                Err(e) if e.kind() == std::io::ErrorKind::CrossesDevices => {
                    // Cross-device move failed, use copy instead
                    fs::copy(&extracted_binary_path, &chromedriver_path).await?;
                    // Set permissions on the copied file
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

            // Clean up
            let _ = fs::remove_file(&zip_path).await;
            let _ = fs::remove_dir_all(temp_extract_dir1).await;

            // Set executable permissions (if not already set during copy)
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

// Modified BrowserPool initialization
pub async fn initialize_browser_pool() -> Result<Arc<BrowserPool>, Box<dyn std::error::Error>> {
    let setup = BrowserSetup::new().await?;

    // Start chromedriver process if not running
    if !is_process_running("chromedriver").await {
        Command::new(&setup.chromedriver_path)
            .arg("--port=9515")
            .spawn()?;

        // Give chromedriver time to start
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    Ok(Arc::new(BrowserPool::new(
        "http://localhost:9515".to_string(),
        5, // Max concurrent browsers
        setup.brave_path,
    )))
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
