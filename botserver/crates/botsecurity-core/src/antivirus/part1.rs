use super::*;

impl Default for AntivirusConfig {
    fn default() -> Self {
        Self {
            clamav_path: None,
            real_time_protection: true,
            quarantine_dir: PathBuf::from("./data/quarantine"),
            log_dir: PathBuf::from("./logs/antivirus"),
            auto_quarantine: true,
            excluded_paths: vec![],
            excluded_extensions: vec![],
            max_file_size_mb: 100,
            scan_archives: true,
            definition_update_url: None,
        }
    }
}
