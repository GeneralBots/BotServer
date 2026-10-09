use super::*;

pub async fn download_and_install(base_path: &std::path::Path, url: &str, component: &str, binary_name: Option<&str>) -> Result<()> {
    let bin_path = base_path.join("bin").join(component);
    std::fs::create_dir_all(&bin_path)?;
    let cache_base = base_path.parent().unwrap_or(base_path);
    let cache = match DownloadCache::new(cache_base) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to initialize download cache: {}", e);
            match DownloadCache::new(base_path) {
                Ok(c) => c,
                Err(e) => {
                    log::error!("Failed to create fallback cache: {}", e);
                    return Err(anyhow::anyhow!("Failed to create download cache"));
                }
            }
        }
    };
    let cache_result = cache.resolve_component_url(component, url);
    let source_file = match cache_result {
        CacheResult::Cached(cached_path) => {
            info!("Using cached file for {}: {}", component, cached_path.display());
            cached_path
        }
        CacheResult::Download { url: download_url, cache_path } => {
            info!("Downloading {} from {}", component, download_url);
            println!("Downloading {}", download_url);
            download_with_reqwest(&download_url, &cache_path, component).await?;
            info!("Cached {} to {}", component, cache_path.display());
            cache_path
        }
    };
    handle_downloaded_file(&source_file, &bin_path, binary_name)?;
    Ok(())
}

pub async fn download_with_reqwest(url: &str, target_file: &std::path::Path, component: &str) -> Result<()> {
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(2);
    if let Some(parent) = target_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("botserver-package-manager/1.0")
        .build()?;
    let mut last_error = None;
    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            trace!("Retry attempt {}/{} for {}", attempt, MAX_RETRIES, component);
            std::thread::sleep(RETRY_DELAY * attempt);
        }
        match attempt_reqwest_download(&client, url, target_file).await {
            Ok(_size) => {
                if attempt > 0 {
                    trace!("Download succeeded on retry attempt {}", attempt);
                }
                return Ok(());
            }
            Err(e) => {
                warn!("Download attempt {} failed: {}", attempt + 1, e);
                last_error = Some(e);
                let _ = std::fs::remove_file(target_file);
            }
        }
    }
    Err(anyhow::anyhow!(
        "Failed to download {} after {} attempts. Last error: {}",
        component,
        MAX_RETRIES + 1,
        last_error.unwrap_or_else(|| anyhow::anyhow!("unknown error"))
    ))
}

pub async fn attempt_reqwest_download(_client: &Client, url: &str, temp_file: &std::path::Path) -> Result<u64> {
    let output_path = temp_file.to_str().context("Invalid temp file path")?;
    download_file(url, output_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to download file using shared utility: {}", e))?;
    let metadata = std::fs::metadata(temp_file).context("Failed to get file metadata")?;
    let size = metadata.len();
    Ok(size)
}
