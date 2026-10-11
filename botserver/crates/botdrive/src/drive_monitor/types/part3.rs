use super::*;

impl DriveMonitor {
    pub(crate) async fn index_kb_file(&self, bot_name: &str, full_key: &str, s3_key: &str, etag: Option<String>) {
        let parsed = match parse_kb_path(s3_key) {
            Some(p) => p,
            None => {
                log::debug!("Not a KB file path: {}", s3_key);
                return;
            }
        };

        let mut being_indexed = self.files_being_indexed.write().await;
        if being_indexed.contains(full_key) {
            log::debug!("Already indexing {}, skipping", full_key);
            return;
        }
        being_indexed.insert(full_key.to_string());
        drop(being_indexed);

        let s3 = match &self.state.drive {
            Some(s3) => s3,
            None => {
                log::error!("S3 client not available for KB indexing of {}", full_key);
                self.files_being_indexed.write().await.remove(full_key);
                return;
            }
        };

        let data = match s3.get_object_direct(&self.bucket_name, s3_key).await {
            Ok(d) => d,
            Err(e) => {
                log::error!("Failed to download KB file {}/{}: {}", self.bucket_name, s3_key, e);
                let _ = self.file_repo.mark_failed(full_key);
                self.files_being_indexed.write().await.remove(full_key);
                return;
            }
        };

        let temp_path = std::env::temp_dir().join(format!("gb_kb_{}_{}", uuid::Uuid::new_v4(), parsed.file_name));

        if let Err(e) = std::fs::write(&temp_path, &data) {
            log::error!("Failed to write temp file {}: {}", temp_path.display(), e);
            self.files_being_indexed.write().await.remove(full_key);
            return;
        }

        log::info!("Indexing KB file {}/{} -> temp {}", bot_name, parsed.kb_name, temp_path.display());

        let index_result =
            tokio::time::timeout(std::time::Duration::from_secs(120), self.kb_manager.index_single_file_with_id(
                self.bot_id,
                bot_name,
                &parsed.kb_name,
                &temp_path,
                Some(full_key),
            )).await;

        match index_result {
            Ok(Ok(result)) => {
                log::info!(
                    "Indexed {} chunks from {} into collection {}",
                    result.chunks_indexed,
                    full_key,
                    result.collection_name
                );
                let _ = self.file_repo.mark_indexed(full_key, etag);
                self.upsert_kb_collection(bot_name, &parsed.kb_name, &result.collection_name, result.documents_processed);
            }
            Ok(Err(e)) => {
                log::error!("KB indexing failed for {}: {}", full_key, e);
                let _ = self.file_repo.mark_failed(full_key);
            }
            Err(_) => {
                log::error!("KB indexing timed out after 120s for {}", full_key);
                let _ = self.file_repo.mark_failed(full_key);
            }
        }

        let _ = std::fs::remove_file(&temp_path);
        self.files_being_indexed.write().await.remove(full_key);
    }

    #[cfg(any(feature = "research", feature = "llm"))]
    pub(super) fn delete_kb_file_vectors(&self, bot_name: &str, _full_key: &str, s3_key: &str) {
        let parsed = match parse_kb_path(s3_key) {
            Some(p) => p,
            None => return,
        };

        let kb_manager = self.kb_manager.clone();
        let bot_id = self.bot_id;
        let bot_name = bot_name.to_string();
        let relative_path = parsed.relative_path.clone();

        tokio::spawn(async move {
            match kb_manager.delete_file_from_kb(bot_id, &bot_name, &parsed.kb_name, &relative_path).await {
                Ok(_) => log::info!("Deleted vectors for {} from {}/{}", relative_path, bot_name, parsed.kb_name),
                Err(e) => log::error!("Failed to delete vectors for {} from {}/{}: {}", relative_path, bot_name, parsed.kb_name, e),
            }
        });
    }

    pub(crate) async fn sync_bot_config(&self, bot_name: &str, s3_key: &str, etag: Option<String>) {
        let s3 = match &self.state.drive {
            Some(s3) => s3,
            None => {
                log::error!("S3 client not available for config sync");
                return;
            }
        };

        let data = match s3.get_object_direct(&self.bucket_name, s3_key).await {
            Ok(d) => d,
            Err(e) => {
                log::error!("Failed to download config.csv from {}/{}: {}", self.bucket_name, s3_key, e);
                return;
            }
        };

        let content = match String::from_utf8(data) {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to parse config.csv as UTF-8: {}", e);
                return;
            }
        };

        let config_manager = botcore::config::ConfigManager::new(self.state.conn.clone());

        let branch_id = self.resolve_branch_id();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.to_lowercase().starts_with("key,") {
                continue;
            }
            if let Some((key, value)) = line.split_once(',') {
                let key = key.trim();
                let value = value.trim();
                if key.is_empty() {
                    continue;
                }
                if is_sensitive_key(key) {
                    // #1425 — warn (not info): a user following the docs and
                    // putting a connection password here gets a connection
                    // that can never authenticate; the line must be visible.
                    log::warn!("Ignoring sensitive key '{}' from config.csv for bot {} — secrets must be configured via Vault", key, bot_name);
                    continue;
                }
                if let Err(e) = config_manager.set_config_with_branch(&self.bot_id, key, value, branch_id) {
                    log::error!("Failed to set config {}={} for bot {}: {}", key, value, bot_name, e);
                } else {
                    log::trace!("Synced config {}={} for bot {} (branch_id={:?})", key, value, bot_name, branch_id);
                }
            }
        }

        let relative_key = self.strip_prefix(s3_key);
        let full_key = format!("{}.gbai/{}", bot_name, relative_key);
        let _ = self.file_repo.mark_indexed(&full_key, etag);
    }

    pub(super) fn resolve_branch_id(&self) -> Option<uuid::Uuid> {
        if self.branch_slug == "default" {
            return Some(uuid::Uuid::nil());
        }
        if let Ok(mut conn) = self.state.conn.get() {
            use diesel::sql_query;
            let result = sql_query(
                "SELECT id FROM branches WHERE slug = $1 LIMIT 1"
            )
            .bind::<diesel::sql_types::Text, _>(&self.branch_slug)
            .get_result::<BranchIdRow>(&mut conn)
            .ok();
            if let Some(row) = result {
                log::info!("Resolved branch_id {} from slug '{}'", row.id, self.branch_slug);
                return Some(row.id);
            }
            log::warn!("No branch found for slug '{}', using nil UUID", self.branch_slug);
        }
        Some(uuid::Uuid::nil())
    }


    pub(crate) async fn sync_gbot_to_work(&self, bot_name: &str, s3_key: &str, etag: Option<String>) {
        let s3 = match &self.state.drive {
            Some(s3) => s3,
            None => {
                log::error!("S3 client not available for .gbot sync");
                return;
            }
        };

        let data = match s3.get_object_direct(&self.bucket_name, s3_key).await {
            Ok(d) => d,
            Err(e) => {
                log::error!("Failed to download gbot file from {}/{}: {}", self.bucket_name, s3_key, e);
                return;
            }
        };

        let work_dir = self.bot_work_dir("gbot");
        if let Err(e) = std::fs::create_dir_all(&work_dir) {
            log::error!("Failed to create work dir {}: {}", work_dir.display(), e);
            return;
        }

        let relative_key = self.strip_prefix(s3_key);
        let file_name = relative_key.split('/').next_back().unwrap_or(relative_key);
        let work_path = work_dir.join(file_name);

        match String::from_utf8(data) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&work_path, &content) {
                    log::error!("Failed to write {} to work dir: {}", work_path.display(), e);
                } else {
                    log::trace!("Synced {} to work dir {}", s3_key, work_path.display());
                    let full_key = format!("{}.gbai/{}", bot_name, relative_key);
                    let _ = self.file_repo.mark_indexed(&full_key, etag);
                }
            }
            Err(e) => {
                log::error!("Failed to parse gbot file as UTF-8: {}", e);
            }
        }
    }

    #[cfg(any(feature = "research", feature = "llm"))]
    pub(super) fn upsert_kb_collection(&self, bot_name: &str, kb_name: &str, collection_name: &str, doc_count: usize) {
        use diesel::prelude::*;
        use uuid::Uuid;

        if let Ok(mut conn) = self.state.conn.get() {
            let folder_path = format!("{}.gbai/{}.gbkb/{}", self.branch_slug, bot_name, kb_name);
            diesel::sql_query(
                "INSERT INTO kb_collections (id, bot_id, name, folder_path, qdrant_collection, document_count)
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (bot_id, name) DO UPDATE SET
                folder_path = EXCLUDED.folder_path,
                qdrant_collection = EXCLUDED.qdrant_collection,
                document_count = EXCLUDED.document_count,
                updated_at = NOW()"
            )
            .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
            .bind::<diesel::sql_types::Uuid, _>(self.bot_id)
            .bind::<diesel::sql_types::Text, _>(kb_name)
            .bind::<diesel::sql_types::Text, _>(&folder_path)
            .bind::<diesel::sql_types::Text, _>(collection_name)
            .bind::<diesel::sql_types::Integer, _>(doc_count as i32)
            .execute(&mut conn)
            .unwrap_or_else(|e| {
                log::error!("Failed to upsert kb_collections for {}/{}: {}", bot_name, kb_name, e);
                0
            });
        }
    }
}
