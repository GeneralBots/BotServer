//! Part of the bottest CLI — see `cli/mod.rs` for the entry point.
//!
//! Moved out of the binary crate by #1505 so the fixture module tree is
//! compiled once; the CLI is part of the `bottest` library now.

use anyhow::Result;
use std::env;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use tracing::Level;
use tracing::info;
use tracing_subscriber::FmtSubscriber;

use crate::{services, web};

pub const CHROMEDRIVER_URL: &str = "https://storage.googleapis.com/chrome-for-testing-public";

pub fn setup_logging(verbose: bool) {
    let level = if verbose { Level::DEBUG } else { Level::INFO };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);
}

pub fn get_cache_dir() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".cache").join("bottest")
}

pub fn get_chromedriver_path(version: &str) -> PathBuf {
    get_cache_dir().join(format!("chromedriver-{version}"))
}

pub fn get_chrome_path() -> PathBuf {
    get_cache_dir().join("chrome-linux64").join("chrome")
}

pub fn detect_existing_browser() -> Option<String> {
    let browsers = [
        "/usr/bin/brave-browser",
        "/usr/bin/brave",
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
    ];

    for browser in browsers {
        if std::path::Path::new(browser).exists() {
            return Some(browser.to_string());
        }
    }

    let chrome_path = get_chrome_path();
    if chrome_path.exists() {
        return Some(chrome_path.to_string_lossy().to_string());
    }

    None
}

pub fn detect_browser_version(browser_path: &str) -> Option<String> {
    let output = std::process::Command::new(browser_path)
        .arg("--version")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let version_str = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = version_str.split_whitespace().collect();

    for part in parts {
        if part.contains('.') && part.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            let major = part.split('.').next()?;
            return Some(major.to_string());
        }
    }

    None
}

pub fn detect_chromedriver_for_version(major_version: &str) -> Option<PathBuf> {
    let pattern = format!("chromedriver-{major_version}");
    let cache_dir = get_cache_dir();

    if let Ok(entries) = std::fs::read_dir(&cache_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&pattern) && entry.path().is_file() {
                return Some(entry.path());
            }
        }
    }

    None
}

pub async fn download_file(url: &str, dest: &PathBuf) -> Result<()> {
    // Only fetch over https or loopback http — never send tokens elsewhere.
    let parsed = url::Url::parse(url)?;
    anyhow::ensure!(
        parsed.scheme() == "https"
            || (parsed.scheme() == "http"
                && matches!(parsed.host_str(), Some("localhost") | Some("127.0.0.1") | Some("[::1]"))),
        "download URL must be https or loopback http"
    );
    info!("Downloading: {}", url);

    let response = reqwest::get(url).await?;

    if !response.status().is_success() {
        anyhow::bail!("Download failed with status: {}", response.status());
    }

    let bytes = response.bytes().await?;

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(dest, &bytes)?;
    info!("Downloaded to: {:?}", dest);

    Ok(())
}

pub fn extract_zip(zip_path: &PathBuf, dest_dir: &PathBuf) -> Result<()> {
    info!("Extracting: {:?} to {:?}", zip_path, dest_dir);

    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    std::fs::create_dir_all(dest_dir)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => dest_dir.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p)?;
                }
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }

        #[cfg(unix)]
        {
            
            if let Some(mode) = file.unix_mode() {
                std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))?;
            }
        }
    }

    Ok(())
}

pub async fn get_chromedriver_version_for_browser(major_version: &str) -> Result<String> {
    let url = format!(
        "https://googlechromelabs.github.io/chrome-for-testing/LATEST_RELEASE_{major_version}"
    );

    info!("Fetching ChromeDriver version for Chrome {}", major_version);

    let response = reqwest::get(&url).await?;
    if !response.status().is_success() {
        anyhow::bail!("Failed to get ChromeDriver version: {}", response.status());
    }

    let version = response.text().await?.trim().to_string();
    info!("Found ChromeDriver version: {}", version);
    Ok(version)
}

pub async fn setup_chromedriver(browser_path: &str) -> Result<PathBuf> {
    let major_version = detect_browser_version(browser_path).unwrap_or_else(|| "131".to_string());

    info!("Detected browser major version: {}", major_version);

    if let Some(existing) = detect_chromedriver_for_version(&major_version) {
        info!("Found existing ChromeDriver: {:?}", existing);
        return Ok(existing);
    }

    info!(
        "ChromeDriver for version {} not found, downloading...",
        major_version
    );

    let cache_dir = get_cache_dir();
    std::fs::create_dir_all(&cache_dir)?;

    let chrome_version = get_chromedriver_version_for_browser(&major_version).await?;

    let chromedriver_url =
        format!("{CHROMEDRIVER_URL}/{chrome_version}/linux64/chromedriver-linux64.zip");

    let zip_path = cache_dir.join("chromedriver.zip");
    download_file(&chromedriver_url, &zip_path).await?;

    extract_zip(&zip_path, &cache_dir)?;

    let extracted_driver = cache_dir.join("chromedriver-linux64").join("chromedriver");
    let final_path = get_chromedriver_path(&major_version);

    if extracted_driver.exists() {
        std::fs::rename(&extracted_driver, &final_path)?;

        #[cfg(unix)]
        {
            
            std::fs::set_permissions(&final_path, std::fs::Permissions::from_mode(0o755))?;
        }
    }

    std::fs::remove_file(&zip_path).ok();
    std::fs::remove_dir_all(cache_dir.join("chromedriver-linux64")).ok();

    if final_path.exists() {
        info!(
            "ChromeDriver {} installed: {:?}",
            chrome_version, final_path
        );
        Ok(final_path)
    } else {
        anyhow::bail!("Failed to install ChromeDriver");
    }
}

pub async fn setup_chrome_for_testing() -> Result<PathBuf> {
    if let Some(browser) = detect_existing_browser() {
        info!("Found existing browser: {}", browser);
        return Ok(PathBuf::from(browser));
    }

    info!("No compatible browser found, downloading Chrome for Testing...");

    let cache_dir = get_cache_dir();
    std::fs::create_dir_all(&cache_dir)?;

    let chrome_version = get_chromedriver_version_for_browser("131")
        .await
        .unwrap_or_else(|_| "131.0.6778.204".to_string());

    let chrome_url = format!("{CHROMEDRIVER_URL}/{chrome_version}/linux64/chrome-linux64.zip");

    let zip_path = cache_dir.join("chrome.zip");
    download_file(&chrome_url, &zip_path).await?;

    extract_zip(&zip_path, &cache_dir)?;

    std::fs::remove_file(&zip_path).ok();

    let chrome_path = get_chrome_path();
    if chrome_path.exists() {
        #[cfg(unix)]
        {
            
            std::fs::set_permissions(&chrome_path, std::fs::Permissions::from_mode(0o755))?;
        }
        info!("Chrome installed: {:?}", chrome_path);
        Ok(chrome_path)
    } else {
        anyhow::bail!("Failed to install Chrome for Testing");
    }
}

pub async fn setup_test_dependencies() -> Result<(PathBuf, PathBuf)> {
    info!("Setting up test dependencies...");

    let chrome = setup_chrome_for_testing().await?;
    let chrome_str = chrome.to_string_lossy().to_string();
    let chromedriver = setup_chromedriver(&chrome_str).await?;

    info!("Dependencies ready:");
    info!("  ChromeDriver: {:?}", chromedriver);
    info!("  Browser: {:?}", chrome);

    Ok((chromedriver, chrome))
}

pub async fn start_chromedriver(chromedriver_path: &PathBuf, port: u16) -> Result<std::process::Child> {
    // #1297 — the driver path is resolved from the download cache at runtime.
    // Only ever execute a chromedriver binary that lives inside the tool cache
    // before it reaches the command line.
    let cache_dir = get_cache_dir().canonicalize()?;
    let canonical = chromedriver_path.canonicalize()?;
    let file_name = canonical
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();
    if !canonical.starts_with(&cache_dir) || !file_name.starts_with("chromedriver") {
        anyhow::bail!("refusing to execute unexpected binary: {}", canonical.display());
    }

    info!("Starting ChromeDriver on port {}...", port);

    let child = std::process::Command::new(chromedriver_path)
        .arg(format!("--port={port}"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    for _ in 0..30 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if check_webdriver_available(port).await {
            info!("ChromeDriver started successfully");
            return Ok(child);
        }
    }

    anyhow::bail!("ChromeDriver failed to start");
}

pub async fn check_webdriver_available(port: u16) -> bool {
    let url = format!("http://localhost:{port}/status");

    let Ok(client) = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
    else {
        return false;
    };

    client.get(&url).send().await.is_ok()
}

pub async fn run_browser_demo() -> Result<()> {
    info!("Running browser demo...");

    let debug_port = 9222u16;

    let mut browser_service = match services::BrowserService::start(debug_port).await {
        Ok(bs) => bs,
        Err(e) => {
            anyhow::bail!("Failed to start browser: {e}");
        }
    };

    info!("Browser started on CDP port {}", debug_port);

    let config = web::BrowserConfig::default()
        .with_browser(web::BrowserType::Chrome)
        .with_debug_port(debug_port)
        .headless(false)
        .with_timeout(std::time::Duration::from_secs(30));

    let browser = match web::Browser::new(config).await {
        Ok(b) => b,
        Err(e) => {
            let _ = browser_service.stop().await;
            anyhow::bail!("Failed to connect to browser CDP: {e}");
        }
    };

    info!("Browser CDP connection established!");
    info!("Navigating to example.com...");
    browser.goto("https://example.com").await?;

    info!("Waiting 5 seconds so you can see the browser...");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    info!("Navigating to Google...");
    browser.goto("https://www.google.com").await?;

    info!("Waiting 5 seconds...");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    info!("Closing browser...");
    let _ = browser.close();
    let _ = browser_service.stop().await;

    info!("Demo complete!");
    Ok(())
}
