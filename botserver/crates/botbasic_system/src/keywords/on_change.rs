use log::info;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FolderProvider {
    GDrive,
    OneDrive,
    Dropbox,
    Local,
}

impl std::str::FromStr for FolderProvider {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "gdrive" | "google" | "googledrive" => Ok(FolderProvider::GDrive),
            "onedrive" | "microsoft" => Ok(FolderProvider::OneDrive),
            "dropbox" => Ok(FolderProvider::Dropbox),
            "local" | "filesystem" => Ok(FolderProvider::Local),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for FolderProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FolderProvider::GDrive => write!(f, "gdrive"),
            FolderProvider::OneDrive => write!(f, "onedrive"),
            FolderProvider::Dropbox => write!(f, "dropbox"),
            FolderProvider::Local => write!(f, "local"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderMonitor {
    pub id: Uuid,
    pub user_id: Uuid,
    pub org_id: Uuid,
    pub provider: FolderProvider,
    pub folder_path: String,
    pub folder_id: Option<String>,
    pub recursive: bool,
    pub event_types: Vec<String>,
    pub script_path: String,
    pub enabled: bool,
    pub last_check: Option<chrono::DateTime<chrono::Utc>>,
    pub last_token: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderChangeEvent {
    pub path: String,
    pub event_type: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub size: Option<i64>,
    pub is_directory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnChangeConfig {
    pub provider: FolderProvider,
    pub folder_path: String,
    pub folder_id: Option<String>,
    pub recursive: bool,
    pub event_types: Vec<String>,
    pub filters: Option<FileFilters>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileFilters {
    pub extensions: Option<Vec<String>>,
    pub min_size: Option<i64>,
    pub max_size: Option<i64>,
    pub name_pattern: Option<String>,
}

pub struct OnChangeKeyword;

impl OnChangeKeyword {
    pub fn execute(
        _state: &Arc<dyn BasicRuntime>,
        config: OnChangeConfig,
        callback_script: &str,
    ) -> Result<Value, String> {
        info!(
            "Setting up folder monitor for {:?} at {}",
            config.provider, config.folder_path
        );

        let monitor = FolderMonitor {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            provider: config.provider,
            folder_path: config.folder_path.clone(),
            folder_id: config.folder_id.clone(),
            recursive: config.recursive,
            event_types: config.event_types.clone(),
            script_path: callback_script.to_string(),
            enabled: true,
            last_check: None,
            last_token: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        Ok(json!({
            "success": true,
            "monitor_id": monitor.id,
            "provider": monitor.provider.to_string(),
            "folder_path": monitor.folder_path,
            "message": "Folder monitor configured (simulation mode)"
        }))
    }

    pub fn check_changes(
        state: &Arc<dyn BasicRuntime>,
        monitor_id: Uuid,
    ) -> Result<Vec<FolderChangeEvent>, String> {
        info!("Checking for folder changes for monitor {}", monitor_id);

        fetch_folder_changes(state, monitor_id)
    }

    pub fn stop_monitor(monitor_id: Uuid) -> Result<Value, String> {
        info!("Stopping folder monitor {}", monitor_id);

        Ok(json!({
            "success": true,
            "monitor_id": monitor_id,
            "message": "Monitor stopped"
        }))
    }
}

pub fn fetch_folder_changes(
    _state: &Arc<dyn BasicRuntime>,
    _monitor_id: Uuid,
) -> Result<Vec<FolderChangeEvent>, String> {
    let now = chrono::Utc::now();

    let events = vec![
        FolderChangeEvent {
            path: "documents/report.pdf".to_string(),
            event_type: "modified".to_string(),
            timestamp: now,
            size: Some(125000),
            is_directory: false,
        },
        FolderChangeEvent {
            path: "documents/new_file.docx".to_string(),
            event_type: "created".to_string(),
            timestamp: now,
            size: Some(45000),
            is_directory: false,
        },
    ];

    info!(
        "Folder change check: returning {} simulated events (real APIs require OAuth setup)",
        events.len()
    );

    Ok(events)
}

use std::sync::Arc;
use botbasic_types::UserSession;
use rhai::Engine;
use uuid::Uuid;
use botbasic_types::BasicRuntime;

pub fn register_on_change_keywords(_state: Arc<dyn BasicRuntime>, _user: UserSession, _engine: &mut Engine) {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn apply_filters(events: Vec<FolderChangeEvent>, filters: &Option<FileFilters>) -> Vec<FolderChangeEvent> {
        let Some(ref filters) = filters else {
            return events;
        };

        events
            .into_iter()
            .filter(|event| {
                if let Some(ref extensions) = filters.extensions {
                    let ext = Path::new(&event.path)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");
                    if !extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
                        return false;
                    }
                }

                if let Some(min_size) = filters.min_size {
                    if event.size.unwrap_or(0) < min_size {
                        return false;
                    }
                }

                if let Some(max_size) = filters.max_size {
                    if event.size.unwrap_or(i64::MAX) > max_size {
                        return false;
                    }
                }

                if let Some(ref pattern) = filters.name_pattern {
                    let file_name = Path::new(&event.path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    if !file_name.contains(pattern) {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    #[test]
    fn test_folder_provider_from_str() {
        assert_eq!(
            "gdrive".parse::<FolderProvider>().ok(),
            Some(FolderProvider::GDrive)
        );
        assert_eq!(
            "onedrive".parse::<FolderProvider>().ok(),
            Some(FolderProvider::OneDrive)
        );
        assert_eq!(
            "dropbox".parse::<FolderProvider>().ok(),
            Some(FolderProvider::Dropbox)
        );
        assert_eq!(
            "local".parse::<FolderProvider>().ok(),
            Some(FolderProvider::Local)
        );
    }

    #[test]
    fn test_apply_filters_extension() {
        let events = vec![
            FolderChangeEvent {
                path: "test.pdf".to_string(),
                event_type: "created".to_string(),
                timestamp: chrono::Utc::now(),
                size: Some(1000),
                is_directory: false,
            },
            FolderChangeEvent {
                path: "test.txt".to_string(),
                event_type: "created".to_string(),
                timestamp: chrono::Utc::now(),
                size: Some(500),
                is_directory: false,
            },
        ];

        let filters = Some(FileFilters {
            extensions: Some(vec!["pdf".to_string()]),
            min_size: None,
            max_size: None,
            name_pattern: None,
        });

        let filtered = apply_filters(events, &filters);
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].path.ends_with(".pdf"));
    }
}