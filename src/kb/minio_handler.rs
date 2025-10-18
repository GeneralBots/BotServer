use crate::shared::state::AppState;
use aws_sdk_s3::Client as S3Client;
use log::{debug, error, info};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::time::{interval, Duration};

/// MinIO file state tracker
#[derive(Debug, Clone)]
pub struct FileState {
    pub path: String,
    pub size: i64,
    pub etag: String,
    pub last_modified: Option<String>,
}

/// MinIO handler that monitors bucket changes
pub struct MinIOHandler {
    state: Arc<AppState>,
    bucket_name: String,
    watched_prefixes: Arc<tokio::sync::RwLock<Vec<String>>>,
    file_states: Arc<tokio::sync::RwLock<HashMap<String, FileState>>>,
}

impl MinIOHandler {
    pub fn new(state: Arc<AppState>, bucket_name: String) -> Self {
        Self {
            state,
            bucket_name,
            watched_prefixes: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            file_states: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Add a prefix to watch (e.g., ".gbkb/", ".gbdialog/")
    pub async fn watch_prefix(&self, prefix: String) {
        let mut prefixes = self.watched_prefixes.write().await;
        if !prefixes.contains(&prefix) {
            prefixes.push(prefix.clone());
            info!("Now watching MinIO prefix: {}", prefix);
        }
    }

    /// Remove a prefix from watch list
    pub async fn unwatch_prefix(&self, prefix: &str) {
        let mut prefixes = self.watched_prefixes.write().await;
        prefixes.retain(|p| p != prefix);
        info!("Stopped watching MinIO prefix: {}", prefix);
    }

    /// Start the monitoring service
    pub fn spawn(
        self: Arc<Self>,
        change_callback: Arc<dyn Fn(FileChangeEvent) + Send + Sync>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!("MinIO Handler service started");
            let mut tick = interval(Duration::from_secs(15)); // Check every 15 seconds

            loop {
                tick.tick().await;

                if let Err(e) = self.check_for_changes(&change_callback).await {
                    error!("Error checking for MinIO changes: {}", e);
                }
            }
        })
    }

    /// Check for file changes in watched prefixes
    async fn check_for_changes(
        &self,
        callback: &Arc<dyn Fn(FileChangeEvent) + Send + Sync>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let s3_client = match &self.state.s3_client {
            Some(client) => client,
            None => {
                debug!("S3 client not configured");
                return Ok(());
            }
        };

        let prefixes = self.watched_prefixes.read().await;

        for prefix in prefixes.iter() {
            debug!("Checking prefix: {}", prefix);

            if let Err(e) = self.check_prefix_changes(s3_client, prefix, callback).await {
                error!("Error checking prefix {}: {}", prefix, e);
            }
        }

        Ok(())
    }

    /// Check changes in a specific prefix
    async fn check_prefix_changes(
        &self,
        s3_client: &S3Client,
        prefix: &str,
        callback: &Arc<dyn Fn(FileChangeEvent) + Send + Sync>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // List all objects with the prefix
        let mut continuation_token: Option<String> = None;
        let mut current_files = HashMap::new();

        loop {
            let mut list_request = s3_client
                .list_objects_v2()
                .bucket(&self.bucket_name)
                .prefix(prefix);

            if let Some(token) = continuation_token {
                list_request = list_request.continuation_token(token);
            }

            let list_result = list_request.send().await?;

            if let Some(contents) = list_result.contents {
                for object in contents {
                    if let Some(key) = object.key {
                        // Skip directories
                        if key.ends_with('/') {
                            continue;
                        }

                        let file_state = FileState {
                            path: key.clone(),
                            size: object.size.unwrap_or(0),
                            etag: object.e_tag.unwrap_or_default(),
                            last_modified: object.last_modified.map(|dt| dt.to_string()),
                        };

                        current_files.insert(key, file_state);
                    }
                }
            }

            if list_result.is_truncated.unwrap_or(false) {
                continuation_token = list_result.next_continuation_token;
            } else {
                break;
            }
        }

        // Compare with previous state
        let mut file_states = self.file_states.write().await;

        // Check for new or modified files
        for (path, current_state) in current_files.iter() {
            if let Some(previous_state) = file_states.get(path) {
                // File exists, check if modified
                if current_state.etag != previous_state.etag
                    || current_state.size != previous_state.size
                {
                    info!("File modified: {}", path);
                    callback(FileChangeEvent::Modified {
                        path: path.clone(),
                        size: current_state.size,
                        etag: current_state.etag.clone(),
                    });
                }
            } else {
                // New file
                info!("File created: {}", path);
                callback(FileChangeEvent::Created {
                    path: path.clone(),
                    size: current_state.size,
                    etag: current_state.etag.clone(),
                });
            }
        }

        // Check for deleted files
        let previous_paths: Vec<String> = file_states
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();

        for path in previous_paths {
            if !current_files.contains_key(&path) {
                info!("File deleted: {}", path);
                callback(FileChangeEvent::Deleted { path: path.clone() });
                file_states.remove(&path);
            }
        }

        // Update state with current files
        for (path, state) in current_files {
            file_states.insert(path, state);
        }

        Ok(())
    }

    /// Get current state of a file
    pub async fn get_file_state(&self, path: &str) -> Option<FileState> {
        let states = self.file_states.read().await;
        states.get(path).cloned()
    }

    /// Clear all tracked file states
    pub async fn clear_state(&self) {
        let mut states = self.file_states.write().await;
        states.clear();
        info!("Cleared all file states");
    }

    /// Get all tracked files for a prefix
    pub async fn get_files_by_prefix(&self, prefix: &str) -> Vec<FileState> {
        let states = self.file_states.read().await;
        states
            .values()
            .filter(|state| state.path.starts_with(prefix))
            .cloned()
            .collect()
    }
}

/// File change event types
#[derive(Debug, Clone)]
pub enum FileChangeEvent {
    Created {
        path: String,
        size: i64,
        etag: String,
    },
    Modified {
        path: String,
        size: i64,
        etag: String,
    },
    Deleted {
        path: String,
    },
}

impl FileChangeEvent {
    pub fn path(&self) -> &str {
        match self {
            FileChangeEvent::Created { path, .. } => path,
            FileChangeEvent::Modified { path, .. } => path,
            FileChangeEvent::Deleted { path } => path,
        }
    }

    pub fn event_type(&self) -> &str {
        match self {
            FileChangeEvent::Created { .. } => "created",
            FileChangeEvent::Modified { .. } => "modified",
            FileChangeEvent::Deleted { .. } => "deleted",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_change_event_path() {
        let event = FileChangeEvent::Created {
            path: "test.txt".to_string(),
            size: 100,
            etag: "abc123".to_string(),
        };

        assert_eq!(event.path(), "test.txt");
        assert_eq!(event.event_type(), "created");
    }

    #[test]
    fn test_file_change_event_types() {
        let created = FileChangeEvent::Created {
            path: "file1.txt".to_string(),
            size: 100,
            etag: "abc".to_string(),
        };
        let modified = FileChangeEvent::Modified {
            path: "file2.txt".to_string(),
            size: 200,
            etag: "def".to_string(),
        };
        let deleted = FileChangeEvent::Deleted {
            path: "file3.txt".to_string(),
        };

        assert_eq!(created.event_type(), "created");
        assert_eq!(modified.event_type(), "modified");
        assert_eq!(deleted.event_type(), "deleted");
    }
}
