use crate::basic::compiler::BasicCompiler;
use crate::kb::embeddings;
use crate::kb::qdrant_client;
use crate::shared::state::AppState;
use aws_sdk_s3::Client as S3Client;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::time::{interval, Duration};

/// Tracks file state for change detection
#[derive(Debug, Clone)]
pub struct FileState {
    pub path: String,
    pub size: i64,
    pub etag: String,
    pub last_modified: Option<String>,
}

/// Drive monitor that watches for changes and triggers compilation/indexing
pub struct DriveMonitor {
    state: Arc<AppState>,
    bucket_name: String,
    file_states: Arc<tokio::sync::RwLock<HashMap<String, FileState>>>,
}

impl DriveMonitor {
    pub fn new(state: Arc<AppState>, bucket_name: String) -> Self {
        Self {
            state,
            bucket_name,
            file_states: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Start the drive monitoring service
    pub fn spawn(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!(
                "Drive Monitor service started for bucket: {}",
                self.bucket_name
            );
            let mut tick = interval(Duration::from_secs(30)); // Check every 30 seconds

            loop {
                tick.tick().await;

                if let Err(e) = self.check_for_changes().await {
                    error!("Error checking for drive changes: {}", e);
                }
            }
        })
    }

    /// Check for file changes in the drive
    async fn check_for_changes(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let s3_client = match &self.state.s3_client {
            Some(client) => client,
            None => {
                debug!("S3 client not configured");
                return Ok(());
            }
        };

        // Check .gbdialog folder for BASIC tools
        self.check_gbdialog_changes(s3_client).await?;

        // Check .gbkb folder for KB documents
        self.check_gbkb_changes(s3_client).await?;

        Ok(())
    }

    /// Check .gbdialog folder for BASIC tool changes
    async fn check_gbdialog_changes(
        &self,
        s3_client: &S3Client,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let prefix = ".gbdialog/";
        debug!("Checking {} folder for changes", prefix);

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
                        // Skip directories and non-.bas files
                        if key.ends_with('/') || !key.ends_with(".bas") {
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

        // Compare with previous state and handle changes
        let mut file_states = self.file_states.write().await;

        for (path, current_state) in current_files.iter() {
            if let Some(previous_state) = file_states.get(path) {
                // File exists, check if modified
                if current_state.etag != previous_state.etag {
                    info!("BASIC tool modified: {}", path);
                    if let Err(e) = self.compile_tool(s3_client, path).await {
                        error!("Failed to compile tool {}: {}", path, e);
                    }
                }
            } else {
                // New file
                info!("New BASIC tool detected: {}", path);
                if let Err(e) = self.compile_tool(s3_client, path).await {
                    error!("Failed to compile tool {}: {}", path, e);
                }
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
                info!("BASIC tool deleted: {}", path);
                // TODO: Mark tool as inactive in database
                file_states.remove(&path);
            }
        }

        // Update state with current files
        for (path, state) in current_files {
            file_states.insert(path, state);
        }

        Ok(())
    }

    /// Check .gbkb folder for KB document changes
    async fn check_gbkb_changes(
        &self,
        s3_client: &S3Client,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let prefix = ".gbkb/";
        debug!("Checking {} folder for changes", prefix);

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

                        // Only process supported file types
                        let ext = key.rsplit('.').next().unwrap_or("").to_lowercase();
                        if !["pdf", "txt", "md", "docx"].contains(&ext.as_str()) {
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

        // Compare with previous state and handle changes
        let mut file_states = self.file_states.write().await;

        for (path, current_state) in current_files.iter() {
            if let Some(previous_state) = file_states.get(path) {
                // File exists, check if modified
                if current_state.etag != previous_state.etag {
                    info!("KB document modified: {}", path);
                    if let Err(e) = self.index_document(s3_client, path).await {
                        error!("Failed to index document {}: {}", path, e);
                    }
                }
            } else {
                // New file
                info!("New KB document detected: {}", path);
                if let Err(e) = self.index_document(s3_client, path).await {
                    error!("Failed to index document {}: {}", path, e);
                }
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
                info!("KB document deleted: {}", path);
                // TODO: Delete from Qdrant and mark in database
                file_states.remove(&path);
            }
        }

        // Update state with current files
        for (path, state) in current_files {
            file_states.insert(path, state);
        }

        Ok(())
    }

    /// Compile a BASIC tool file
    async fn compile_tool(
        &self,
        s3_client: &S3Client,
        file_path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("Compiling BASIC tool: {}", file_path);

        // Download source from S3
        let get_response = s3_client
            .get_object()
            .bucket(&self.bucket_name)
            .key(file_path)
            .send()
            .await?;

        let data = get_response.body.collect().await?;
        let source_content = String::from_utf8(data.into_bytes().to_vec())?;

        // Extract tool name
        let tool_name = file_path
            .strip_prefix(".gbdialog/")
            .unwrap_or(file_path)
            .strip_suffix(".bas")
            .unwrap_or(file_path)
            .to_string();

        // Calculate file hash for change detection
        let _file_hash = format!("{:x}", source_content.len());

        // Create work directory
        let work_dir = "./work/default.gbai/default.gbdialog";
        std::fs::create_dir_all(work_dir)?;

        // Write source to local file
        let local_source_path = format!("{}/{}.bas", work_dir, tool_name);
        std::fs::write(&local_source_path, &source_content)?;

        // Compile using BasicCompiler
        let compiler = BasicCompiler::new(Arc::clone(&self.state));
        let result = compiler.compile_file(&local_source_path, work_dir)?;

        info!("Tool compiled successfully: {}", tool_name);
        info!("  AST: {}", result.ast_path);

        // Save to database
        if let Some(mcp_tool) = result.mcp_tool {
            info!(
                "  MCP tool definition generated with {} parameters",
                mcp_tool.input_schema.properties.len()
            );
        }

        if result.openai_tool.is_some() {
            info!("  OpenAI tool definition generated");
        }

        // TODO: Insert/update in basic_tools table
        // INSERT INTO basic_tools (id, bot_id, tool_name, file_path, ast_path, file_hash,
        //                          mcp_json, tool_json, compiled_at, is_active, created_at, updated_at)
        // VALUES (...) ON CONFLICT (bot_id, tool_name) DO UPDATE SET ...

        Ok(())
    }

    /// Index a KB document
    async fn index_document(
        &self,
        s3_client: &S3Client,
        file_path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("Indexing KB document: {}", file_path);

        // Extract collection name from path (.gbkb/collection_name/file.pdf)
        let parts: Vec<&str> = file_path.split('/').collect();
        if parts.len() < 3 {
            warn!("Invalid KB path structure: {}", file_path);
            return Ok(());
        }

        let collection_name = parts[1];

        // Download file from S3
        let get_response = s3_client
            .get_object()
            .bucket(&self.bucket_name)
            .key(file_path)
            .send()
            .await?;

        let data = get_response.body.collect().await?;
        let bytes = data.into_bytes().to_vec();

        // Extract text based on file type
        let text_content = self.extract_text(file_path, &bytes)?;

        if text_content.trim().is_empty() {
            warn!("No text extracted from: {}", file_path);
            return Ok(());
        }

        info!(
            "Extracted {} characters from {}",
            text_content.len(),
            file_path
        );

        // Create Qdrant collection name
        let qdrant_collection = format!("kb_default_{}", collection_name);

        // Ensure collection exists
        qdrant_client::ensure_collection_exists(&self.state, &qdrant_collection).await?;

        // Index document
        embeddings::index_document(&self.state, &qdrant_collection, file_path, &text_content)
            .await?;

        info!("Document indexed successfully: {}", file_path);

        // TODO: Insert/update in kb_documents table
        // INSERT INTO kb_documents (id, bot_id, user_id, collection_name, file_path, file_size,
        //                           file_hash, first_published_at, last_modified_at, indexed_at,
        //                           metadata, created_at, updated_at)
        // VALUES (...) ON CONFLICT (...) DO UPDATE SET ...

        Ok(())
    }

    /// Extract text from various file types
    fn extract_text(
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
        } else {
            // Try as plain text
            String::from_utf8(content.to_vec())
                .map_err(|e| format!("Unsupported file format or UTF-8 error: {}", e).into())
        }
    }

    /// Clear all tracked file states
    pub async fn clear_state(&self) {
        let mut states = self.file_states.write().await;
        states.clear();
        info!("Cleared all file states");
    }
}
