use super::*;

impl MediaUploadService {
    async fn upload_to_azure_blob(
        &self,
        data: &[u8],
        upload: &MediaUpload,
    ) -> Result<PlatformUploadResult, MediaUploadError> {
        let account = std::env::var("AZURE_STORAGE_ACCOUNT")
            .map_err(|_| MediaUploadError::ConfigError("AZURE_STORAGE_ACCOUNT not configured".to_string()))?;
        let container = std::env::var("AZURE_STORAGE_CONTAINER")
            .map_err(|_| MediaUploadError::ConfigError("AZURE_STORAGE_CONTAINER not configured".to_string()))?;

        let blob_name = format!("uploads/{}/{}", upload.organization_id, upload.id);
        let content_type = &upload.content_type;

        let client = reqwest::Client::new();
        // The storage account becomes part of the hostname; constrain it to
        // hostname-safe characters so the upload cannot be retargeted.
        let account: String = account
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
            .collect();
        let container: String = container
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
            .collect();
        let url = format!(
            "https://{}.blob.core.windows.net/{}/{}",
            account, container, blob_name
        );

        let response = client
            .put(&url)
            .header("Content-Type", content_type)
            .header("x-ms-blob-type", "BlockBlob")
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| MediaUploadError::UploadFailed(format!("Azure upload failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(MediaUploadError::UploadFailed(
                format!("Azure returned status: {}", response.status())
            ));
        }

        Ok(PlatformUploadResult {
            media_id: format!("azure_{}", upload.id),
            url: Some(url),
            thumbnail_url: None,
        })
    }

    async fn upload_to_local_storage(
        &self,
        data: &[u8],
        upload: &MediaUpload,
    ) -> Result<PlatformUploadResult, MediaUploadError> {
        let storage_path = std::env::var("LOCAL_STORAGE_PATH")
            .unwrap_or_else(|_| "/var/lib/generalbots/uploads".to_string());
        let base_url = std::env::var("LOCAL_STORAGE_URL")
            .unwrap_or_else(|_| "/uploads".to_string());

        let org_dir = format!("{}/{}", storage_path, upload.organization_id);
        std::fs::create_dir_all(&org_dir)
            .map_err(|e| MediaUploadError::UploadFailed(format!("Failed to create directory: {}", e)))?;

        let file_path = format!("{}/{}", org_dir, upload.id);
        std::fs::write(&file_path, data)
            .map_err(|e| MediaUploadError::UploadFailed(format!("Failed to write file: {}", e)))?;

        let public_url = format!("{}/{}/{}", base_url, upload.organization_id, upload.id);

        Ok(PlatformUploadResult {
            media_id: format!("local_{}", upload.id),
            url: Some(public_url),
            thumbnail_url: None,
        })
    }

    pub async fn append_chunk(
        &self,
        upload_id: Uuid,
        chunk_index: u32,
        data: Vec<u8>,
    ) -> Result<ChunkedUploadAppendResponse, MediaUploadError> {
        let mut chunked = self.chunked_uploads.write().await;
        let state = chunked
            .get_mut(&upload_id)
            .ok_or(MediaUploadError::UploadNotFound)?;

        let total_chunks = state.chunks_received.len() as u32;

        if chunk_index >= total_chunks {
            return Err(MediaUploadError::InvalidChunkIndex);
        }

        if state.chunks_received[chunk_index as usize] {
            return Err(MediaUploadError::ChunkAlreadyReceived);
        }

        state.chunk_data[chunk_index as usize] = data;
        state.chunks_received[chunk_index as usize] = true;

        let chunks_done = state.chunks_received.iter().filter(|&&x| x).count() as u32;
        let progress = (chunks_done as f32 / total_chunks as f32) * 100.0;

        state.upload.upload_progress = progress;
        state.upload.status = UploadStatus::Uploading;
        state.upload.updated_at = Utc::now();

        Ok(ChunkedUploadAppendResponse {
            upload_id,
            chunks_received: chunks_done,
            total_chunks,
            progress,
        })
    }
}
