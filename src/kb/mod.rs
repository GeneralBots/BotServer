use crate::shared::models::KBCollection;
use crate::shared::state::AppState;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::time::{interval, Duration};

pub mod embeddings;
pub mod minio_handler;
pub mod qdrant_client;

/// Represents a change in a KB file
#[derive(Debug, Clone)]
pub enum FileChangeEvent {
    Created(String),
    Modified(String),
    Deleted(String),
}

/// KB Manager service that coordinates MinIO monitoring and Qdrant indexing
pub struct KBManager {
    state: Arc<AppState>,
    watched_collections: Arc<tokio::sync::RwLock<HashMap<String, KBCollection>>>,
}

impl KBManager {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            watched_collections: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Start watching a KB collection folder
    pub async fn add_collection(
        &self,
        bot_id: String,
        user_id: String,
        collection_name: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let folder_path = format!(".gbkb/{}", collection_name);
        let qdrant_collection = format!("kb_{}_{}", bot_id, collection_name);

        info!(
            "Adding KB collection: {} -> {}",
            collection_name, qdrant_collection
        );

        // Create Qdrant collection if it doesn't exist
        qdrant_client::ensure_collection_exists(&self.state, &qdrant_collection).await?;

        let now = chrono::Utc::now().to_rfc3339();
        let collection = KBCollection {
            id: uuid::Uuid::new_v4().to_string(),
            bot_id,
            user_id,
            name: collection_name.to_string(),
            folder_path: folder_path.clone(),
            qdrant_collection: qdrant_collection.clone(),
            document_count: 0,
            is_active: 1,
            created_at: now.clone(),
            updated_at: now,
        };

        let mut collections = self.watched_collections.write().await;
        collections.insert(collection_name.to_string(), collection);

        info!("KB collection added successfully: {}", collection_name);
        Ok(())
    }

    /// Remove a KB collection
    pub async fn remove_collection(
        &self,
        collection_name: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut collections = self.watched_collections.write().await;
        collections.remove(collection_name);
        info!("KB collection removed: {}", collection_name);
        Ok(())
    }

    /// Start the KB monitoring service
    pub fn spawn(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!("KB Manager service started");
            let mut tick = interval(Duration::from_secs(30));

            loop {
                tick.tick().await;

                let collections = self.watched_collections.read().await;
                for (name, collection) in collections.iter() {
                    if let Err(e) = self.check_collection_updates(collection).await {
                        error!("Error checking collection {}: {}", name, e);
                    }
                }
            }
        })
    }

    /// Check for updates in a collection
    async fn check_collection_updates(
        &self,
        collection: &KBCollection,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Checking updates for collection: {}", collection.name);

        let s3_client = match &self.state.s3_client {
            Some(client) => client,
            None => {
                warn!("S3 client not configured");
                return Ok(());
            }
        };

        let config = match &self.state.config {
            Some(cfg) => cfg,
            None => {
                error!("App configuration missing");
                return Err("App configuration missing".into());
            }
        };

        let bucket_name = format!("{}default.gbai", config.minio.org_prefix);

        // List objects in the collection folder
        let list_result = s3_client
            .list_objects_v2()
            .bucket(&bucket_name)
            .prefix(&collection.folder_path)
            .send()
            .await?;

        if let Some(contents) = list_result.contents {
            for object in contents {
                if let Some(key) = object.key {
                    // Skip directories
                    if key.ends_with('/') {
                        continue;
                    }

                    // Check if file needs indexing
                    if let Err(e) = self
                        .process_file(
                            &collection,
                            &key,
                            object.size.unwrap_or(0),
                            object.last_modified.map(|dt| dt.to_string()),
                        )
                        .await
                    {
                        error!("Error processing file {}: {}", key, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Process a single file (check if changed and index if needed)
    async fn process_file(
        &self,
        collection: &KBCollection,
        file_path: &str,
        file_size: i64,
        _last_modified: Option<String>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Get file content hash
        let content = self.get_file_content(file_path).await?;
        // Simple hash using length and first/last bytes for change detection
        let file_hash = if content.len() > 100 {
            format!(
                "{:x}_{:x}_{}",
                content.len(),
                content[0] as u32 * 256 + content[1] as u32,
                content[content.len() - 1] as u32 * 256 + content[content.len() - 2] as u32
            )
        } else {
            format!("{:x}", content.len())
        };

        // Check if file is already indexed with same hash
        if self
            .is_file_indexed(collection.bot_id.clone(), file_path, &file_hash)
            .await?
        {
            debug!("File already indexed: {}", file_path);
            return Ok(());
        }

        info!(
            "Indexing file: {} to collection {}",
            file_path, collection.name
        );

        // Extract text based on file type
        let text_content = self.extract_text(file_path, &content).await?;

        // Generate embeddings and store in Qdrant
        embeddings::index_document(
            &self.state,
            &collection.qdrant_collection,
            file_path,
            &text_content,
        )
        .await?;

        // Save metadata to database
        let metadata = serde_json::json!({
            "file_type": self.get_file_type(file_path),
            "last_modified": _last_modified,
        });

        self.save_document_metadata(
            collection.bot_id.clone(),
            &collection.name,
            file_path,
            file_size,
            &file_hash,
            metadata,
        )
        .await?;

        info!("File indexed successfully: {}", file_path);
        Ok(())
    }

    /// Get file content from MinIO
    async fn get_file_content(
        &self,
        file_path: &str,
    ) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let s3_client = self
            .state
            .s3_client
            .as_ref()
            .ok_or("S3 client not configured")?;

        let config = self
            .state
            .config
            .as_ref()
            .ok_or("App configuration missing")?;

        let bucket_name = format!("{}default.gbai", config.minio.org_prefix);

        let response = s3_client
            .get_object()
            .bucket(&bucket_name)
            .key(file_path)
            .send()
            .await?;

        let data = response.body.collect().await?;
        Ok(data.into_bytes().to_vec())
    }

    /// Extract text from various file types
    async fn extract_text(
        &self,
        file_path: &str,
        content: &[u8],
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let path_lower = file_path.to_ascii_lowercase();

        if path_lower.ends_with(".pdf") {
            match pdf_extract::extract_text_from_mem(content) {
                Ok(text) => Ok(text),
                Err(e) => {
                    error!("PDF extraction failed for {}: {}", file_path, e);
                    Err(format!("PDF extraction failed: {}", e).into())
                }
            }
        } else if path_lower.ends_with(".txt") || path_lower.ends_with(".md") {
            String::from_utf8(content.to_vec())
                .map_err(|e| format!("UTF-8 decoding failed: {}", e).into())
        } else if path_lower.ends_with(".docx") {
            // TODO: Add DOCX support
            warn!("DOCX format not yet supported: {}", file_path);
            Err("DOCX format not supported".into())
        } else {
            // Try as plain text
            String::from_utf8(content.to_vec())
                .map_err(|e| format!("Unsupported file format or UTF-8 error: {}", e).into())
        }
    }

    /// Check if file is already indexed
    async fn is_file_indexed(
        &self,
        _bot_id: String,
        _file_path: &str,
        _file_hash: &str,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        // TODO: Query database to check if file with same hash exists
        // For now, return false to always reindex
        Ok(false)
    }

    /// Save document metadata to database
    async fn save_document_metadata(
        &self,
        _bot_id: String,
        _collection_name: &str,
        file_path: &str,
        file_size: i64,
        file_hash: &str,
        _metadata: serde_json::Value,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // TODO: Save to database using Diesel
        info!(
            "Saving metadata for {}: size={}, hash={}",
            file_path, file_size, file_hash
        );
        Ok(())
    }

    /// Get file type from path
    fn get_file_type(&self, file_path: &str) -> String {
        file_path
            .rsplit('.')
            .next()
            .unwrap_or("unknown")
            .to_lowercase()
    }
}
