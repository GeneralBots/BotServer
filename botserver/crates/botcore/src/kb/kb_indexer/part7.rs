use super::*;

impl KbFolderMonitor {
    pub fn new(work_root: PathBuf, embedding_config: EmbeddingConfig) -> Self {
        let qdrant_config = if let Ok(sm) = crate::shared::utils::get_secrets_manager_sync() {
            let (url, api_key, _collection) = get_vectordb_config_from_value(&sm);
                QdrantConfig {
                    url,
                    api_key: Some(api_key),
                    timeout_secs: 30,
                }
        } else {
            QdrantConfig::default()
        };
        let indexer = KbIndexer::new(embedding_config, qdrant_config);

        Self { indexer, work_root }
    }

    pub async fn process_gbkb_folder(&self, bot_id: Uuid, bot_name: &str, kb_folder: &Path) -> Result<IndexingResult> {
        let kb_name = kb_folder
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid KB folder name"))?;

        info!("Processing .gbkb folder: {} for bot {}", kb_name, bot_name);

        let local_path = self
            .work_root
            .join(bot_name)
            .join(format!("{}.gbkb", bot_name))
            .join(kb_name);

        let result = self
            .indexer
            .index_kb_folder(bot_id, bot_name, kb_name, &local_path)
            .await?;

        info!(
            "Indexed {} documents ({} chunks) into collection {}",
            result.documents_processed, result.chunks_indexed, result.collection_name
        );

        Ok(result)
    }
}
