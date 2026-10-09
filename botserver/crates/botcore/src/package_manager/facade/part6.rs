use super::*;

impl PackageManager {
    pub fn install_system_packages(&self, component: &ComponentConfig) -> Result<()> {
        let packages = match self.os_type {
            OsType::Linux => &component.linux_packages,
            OsType::MacOS => &component.macos_packages,
            OsType::Windows => &component.windows_packages,
        };
        if packages.is_empty() {
            return Ok(());
        }
        trace!(
            "Installing {} system packages for component '{}'",
            packages.len(),
            component.name
        );
        match self.os_type {
            OsType::Linux => {
                if let Some(output) = safe_apt_get(&["update"]) {
                    if !output.status.success() {
                        warn!("apt-get update had issues");
                    }
                }
                let mut args = vec!["install", "-y"];
                for pkg in packages {
                    args.push(pkg.as_str());
                }
                if let Some(output) = safe_apt_get(&args) {
                    if !output.status.success() {
                        warn!("Some packages may have failed to install");
                    }
                }
            }
            OsType::MacOS => {
                let mut args = vec!["install"];
                for pkg in packages {
                    args.push(pkg.as_str());
                }
                if let Some(output) = safe_brew(&args) {
                    if !output.status.success() {
                        warn!("Homebrew installation had warnings");
                    }
                }
            }
            OsType::Windows => {
                trace!("Windows packages managed via GB portable binaries, skipping system package manager");
            }
        }
        Ok(())
    }
    pub async fn download_and_install(
        &self,
        url: &str,
        component: &str,
        binary_name: Option<&str>,
    ) -> Result<()> {
        let bin_path = self.base_path.join("bin").join(component);
        std::fs::create_dir_all(&bin_path)?;

        let cache_base = self.base_path.parent().unwrap_or(&self.base_path);
        let cache = match DownloadCache::new(cache_base) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to initialize download cache: {}", e);
                match DownloadCache::new(&self.base_path) {
                    Ok(c) => c,
                    Err(e) => {
                        log::error!("Failed to create fallback cache: {}", e);
                        return Err(anyhow::anyhow!("Failed to create download cache"));
                    }
                }
            }
        };

        let cache_result = cache.resolve_url(url);

        let source_file = match cache_result {
            CacheResult::Cached(cached_path) => {
                info!(
                    "Using cached file for {}: {}",
                    component,
                    cached_path.display()
                );
                cached_path
            }
            CacheResult::Download {
                url: download_url,
                cache_path,
            } => {
                info!("Downloading {} from {}", component, download_url);
                println!("Downloading {}", download_url);

                self.download_with_reqwest(&download_url, &cache_path, component)
                    .await?;

                info!("Cached {} to {}", component, cache_path.display());
                cache_path
            }
        };

        self.handle_downloaded_file(&source_file, &bin_path, binary_name)?;
        Ok(())
    }
    pub async fn download_with_reqwest(
        &self,
        url: &str,
        target_file: &std::path::Path,
        component: &str,
    ) -> Result<()> {
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
                trace!(
                    "Retry attempt {}/{} for {}",
                    attempt,
                    MAX_RETRIES,
                    component
                );
                std::thread::sleep(RETRY_DELAY * attempt);
            }
            match self
                .attempt_reqwest_download(&client, url, target_file)
                .await
            {
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
    pub async fn attempt_reqwest_download(
        &self,
        _client: &Client,
        url: &str,
        temp_file: &std::path::Path,
    ) -> Result<u64> {
        let output_path = temp_file.to_str().context("Invalid temp file path")?;
        utils::download_file(url, output_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to download file using shared utility: {}", e))?;
        let metadata = std::fs::metadata(temp_file).context("Failed to get file metadata")?;
        let size = metadata.len();
        Ok(size)
    }
    pub fn handle_downloaded_file(
        &self,
        temp_file: &std::path::Path,
        bin_path: &std::path::Path,
        binary_name: Option<&str>,
    ) -> Result<()> {
        let metadata = std::fs::metadata(temp_file)?;
        if metadata.len() == 0 {
            return Err(anyhow::anyhow!("Downloaded file is empty"));
        }
        let file_extension = temp_file
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        match file_extension {
            "gz" | "tgz" => {
                self.extract_tar_gz(temp_file, bin_path)?;
            }
            "zip" => {
                self.extract_zip(temp_file, bin_path)?;
            }
            _ => {
                if let Some(name) = binary_name {
                    self.install_binary(temp_file, bin_path, name)?;
                } else {
                    let final_path = bin_path.join(temp_file.file_name().unwrap_or_default());

                    if temp_file.to_string_lossy().contains("botserver-installers") {
                        std::fs::copy(temp_file, &final_path)?;
                    } else {
                        std::fs::rename(temp_file, &final_path)?;
                    }
                    self.make_executable(&final_path)?;
                }
            }
        }
        Ok(())
    }
    pub fn extract_tar_gz(
        &self,
        temp_file: &std::path::Path,
        bin_path: &std::path::Path,
    ) -> Result<()> {
        let tar_gz = std::fs::File::open(temp_file)?;
        let tar = flate2::read::GzDecoder::new(tar_gz);
        let mut archive = tar::Archive::new(tar);
        archive.unpack(bin_path)?;

        collapse_single_subdirectory(bin_path)?;

        if !temp_file.to_string_lossy().contains("botserver-installers") {
            std::fs::remove_file(temp_file)?;
        }
        Ok(())
    }
    pub fn extract_zip(
        &self,
        temp_file: &std::path::Path,
        bin_path: &std::path::Path,
    ) -> Result<()> {
        trace!("extract_zip: opening {}", temp_file.display());
        let file = std::fs::File::open(temp_file)
            .with_context(|| format!("Failed to open zip: {}", temp_file.display()))?;
        trace!("extract_zip: file opened, size={}", file.metadata().map(|m| m.len()).unwrap_or(0));

        let mut archive = zip::ZipArchive::new(file)
            .with_context(|| format!("Failed to read zip: {}", temp_file.display()))?;
        trace!("extract_zip: archive read, {} entries", archive.len());

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)
                .with_context(|| format!("Failed to read entry {i} from zip"))?;
            let name = entry.name().to_string();

            // Skip unnecessary directories to speed up extraction
            // (e.g. pgAdmin 4 in EDB PostgreSQL distribution has 22k+ files)
            if name.contains("pgAdmin 4") || name.contains("StackBuilder") {
                continue;
            }

            let target = bin_path.join(&name);
            trace!("extracting[{}]: {}", i, name);
            if entry.is_dir() {
                std::fs::create_dir_all(&target)
                    .with_context(|| format!("Failed to create dir: {}", target.display()))?;
            } else {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("Failed to create parent dir for: {}", target.display()))?;
                }
                let mut outfile = std::fs::File::create(&target)
                    .with_context(|| format!("Failed to create file: {}", target.display()))?;
                std::io::copy(&mut entry, &mut outfile)
                    .with_context(|| format!("Failed to write: {}", target.display()))?;
            }
            #[cfg(unix)]
            {
                if let Some(mode) = entry.unix_mode() {
                    if mode & 0o111 != 0 {
                        let _ = botlib::os::fs::get_permissions_manager()
                            .set_executable(&target);
                    }
                }
            }
        }

        #[cfg(unix)]
        {
            if let Ok(entries) = std::fs::read_dir(bin_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Ok(_metadata) = std::fs::metadata(&path) {
                            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                            if ext.is_empty() || ext == "sh" || ext == "bash" {
                                let _ = botlib::os::fs::get_permissions_manager().set_executable(&path);
                                trace!("Made executable: {}", path.display());
                            }
                        }
                    }
                }
            }
        }

        collapse_single_subdirectory(bin_path)?;

        if !temp_file.to_string_lossy().contains("botserver-installers") {
            let _ = std::fs::remove_file(temp_file);
        }
        Ok(())
    }
}
