use super::*;

impl MediaUploadService {
    pub fn get_platform_limits(&self, platform: &Platform) -> Option<&PlatformLimits> {
        self.platform_limits.get(platform)
    }

    pub async fn upload_media(
        &self,
        platform: Platform,
        organization_id: Uuid,
        data: Vec<u8>,
        filename: String,
        content_type: String,
        alt_text: Option<String>,
    ) -> Result<MediaUpload, MediaUploadError> {
        let limits = self
            .get_platform_limits(&platform)
            .ok_or_else(|| MediaUploadError::UnsupportedPlatform(platform.as_str().to_string()))?;

        let media_type = self.detect_media_type(&content_type);
        let extension = self.get_extension(&filename).unwrap_or_default();

        self.validate_format(&media_type, &extension, limits)?;
        self.validate_size(&media_type, data.len() as u64, limits)?;

        let now = Utc::now();
        let upload_id = Uuid::new_v4();

        let mut upload = MediaUpload {
            id: upload_id,
            organization_id,
            platform: platform.clone(),
            media_type,
            status: UploadStatus::Uploading,
            original_filename: filename.clone(),
            content_type: content_type.clone(),
            size_bytes: data.len() as u64,
            width: None,
            height: None,
            duration_seconds: None,
            local_path: None,
            platform_media_id: None,
            platform_url: None,
            thumbnail_url: None,
            error_message: None,
            upload_progress: 0.0,
            created_at: now,
            updated_at: now,
            expires_at: Some(now + chrono::Duration::hours(24)),
            metadata: HashMap::new(),
        };

        if let Some(alt) = alt_text {
            upload.metadata.insert("alt_text".into(), serde_json::json!(alt));
        }

        let platform_result = self.upload_to_platform(&platform, &data, &upload).await?;

        upload.platform_media_id = Some(platform_result.media_id);
        upload.platform_url = platform_result.url;
        upload.thumbnail_url = platform_result.thumbnail_url;
        upload.status = UploadStatus::Ready;
        upload.upload_progress = 100.0;
        upload.updated_at = Utc::now();

        {
            let mut uploads = self.uploads.write().await;
            uploads.insert(upload_id, upload.clone());
        }

        Ok(upload)
    }

    pub async fn init_chunked_upload(
        &self,
        platform: Platform,
        organization_id: Uuid,
        filename: String,
        content_type: String,
        total_size: u64,
        media_type: MediaType,
    ) -> Result<ChunkedUploadInitResponse, MediaUploadError> {
        let limits = self
            .get_platform_limits(&platform)
            .ok_or_else(|| MediaUploadError::UnsupportedPlatform(platform.as_str().to_string()))?;

        self.validate_size(&media_type, total_size, limits)?;

        let chunk_size: u64 = 5 * 1024 * 1024;
        let total_chunks = ((total_size as f64) / (chunk_size as f64)).ceil() as u32;

        let now = Utc::now();
        let upload_id = Uuid::new_v4();

        let upload = MediaUpload {
            id: upload_id,
            organization_id,
            platform: platform.clone(),
            media_type,
            status: UploadStatus::Pending,
            original_filename: filename,
            content_type,
            size_bytes: total_size,
            width: None,
            height: None,
            duration_seconds: None,
            local_path: None,
            platform_media_id: None,
            platform_url: None,
            thumbnail_url: None,
            error_message: None,
            upload_progress: 0.0,
            created_at: now,
            updated_at: now,
            expires_at: Some(now + chrono::Duration::hours(24)),
            metadata: HashMap::new(),
        };

        let state = ChunkedUploadState {
            upload,
            chunks_received: vec![false; total_chunks as usize],
            chunk_data: vec![Vec::new(); total_chunks as usize],
            chunk_size,
        };

        {
            let mut chunked = self.chunked_uploads.write().await;
            chunked.insert(upload_id, state);
        }

        Ok(ChunkedUploadInitResponse {
            upload_id,
            chunk_size,
            total_chunks,
            expires_at: now + chrono::Duration::hours(24),
        })
    }

    pub(super) fn detect_media_type(&self, content_type: &str) -> MediaType {
        if content_type.starts_with("image/gif") {
            MediaType::Gif
        } else if content_type.starts_with("image/") {
            MediaType::Image
        } else if content_type.starts_with("video/") {
            MediaType::Video
        } else if content_type.starts_with("audio/") {
            MediaType::Audio
        } else {
            MediaType::Document
        }
    }

    pub(super) fn get_extension(&self, filename: &str) -> Option<String> {
        filename
            .rsplit('.')
            .next()
            .map(|s| s.to_lowercase())
    }

    pub(super) fn validate_format(
        &self,
        media_type: &MediaType,
        extension: &str,
        limits: &PlatformLimits,
    ) -> Result<(), MediaUploadError> {
        let supported = match media_type {
            MediaType::Image | MediaType::Gif => &limits.supported_image_formats,
            MediaType::Video => &limits.supported_video_formats,
            MediaType::Audio | MediaType::Document => return Ok(()),
        };
        if supported.iter().any(|f| f.eq_ignore_ascii_case(extension)) {
            Ok(())
        } else {
            Err(MediaUploadError::UnsupportedFormat)
        }
    }

    pub(super) fn validate_size(
        &self,
        media_type: &MediaType,
        size: u64,
        limits: &PlatformLimits,
    ) -> Result<(), MediaUploadError> {
        let max_size = match media_type {
            MediaType::Image | MediaType::Gif => limits.max_image_size_bytes,
            MediaType::Video => limits.max_video_size_bytes,
            MediaType::Audio | MediaType::Document => limits.max_video_size_bytes,
        };
        if size <= max_size {
            Ok(())
        } else {
            Err(MediaUploadError::FileTooLarge)
        }
    }

    async fn upload_to_platform(
        &self,
        _platform: &Platform,
        data: &[u8],
        upload: &MediaUpload,
    ) -> Result<PlatformUploadResult, MediaUploadError> {
        // Get storage configuration from environment
        let storage_type = std::env::var("MEDIA_STORAGE_TYPE").unwrap_or_else(|_| "local".to_string());

        match storage_type.as_str() {
            "s3" => self.upload_to_s3(data, upload).await,
            "gcs" => self.upload_to_gcs(data, upload).await,
            "azure" => self.upload_to_azure_blob(data, upload).await,
            _ => self.upload_to_local_storage(data, upload).await,
        }
    }

    async fn upload_to_s3(
        &self,
        data: &[u8],
        upload: &MediaUpload,
    ) -> Result<PlatformUploadResult, MediaUploadError> {
        let bucket = std::env::var("S3_BUCKET")
            .map_err(|_| MediaUploadError::ConfigError("S3_BUCKET not configured".to_string()))?;
        let region = std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());
        let _access_key = std::env::var("AWS_ACCESS_KEY_ID")
            .map_err(|_| MediaUploadError::ConfigError("AWS_ACCESS_KEY_ID not configured".to_string()))?;
        let _secret_key = std::env::var("AWS_SECRET_ACCESS_KEY")
            .map_err(|_| MediaUploadError::ConfigError("AWS_SECRET_ACCESS_KEY not configured".to_string()))?;

        let key = format!("uploads/{}/{}", upload.organization_id, upload.id);
        let content_type = &upload.content_type;

        // Build S3 presigned URL and upload
        let client = reqwest::Client::new();
        let url = format!("https://{}.s3.{}.amazonaws.com/{}", bucket, region, key);

        // Create AWS signature (simplified - in production use aws-sdk-s3)
        let response = client
            .put(&url)
            .header("Content-Type", content_type)
            .header("Content-Length", data.len())
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| MediaUploadError::UploadFailed(format!("S3 upload failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(MediaUploadError::UploadFailed(
                format!("S3 returned status: {}", response.status())
            ));
        }

        let public_url = format!("https://{}.s3.{}.amazonaws.com/{}", bucket, region, key);

        Ok(PlatformUploadResult {
            media_id: format!("s3_{}", upload.id),
            url: Some(public_url),
            thumbnail_url: None,
        })
    }

    async fn upload_to_gcs(
        &self,
        data: &[u8],
        upload: &MediaUpload,
    ) -> Result<PlatformUploadResult, MediaUploadError> {
        let bucket = std::env::var("GCS_BUCKET")
            .map_err(|_| MediaUploadError::ConfigError("GCS_BUCKET not configured".to_string()))?;

        let key = format!("uploads/{}/{}", upload.organization_id, upload.id);
        let content_type = &upload.content_type;

        let client = reqwest::Client::new();
        let url = format!(
            "https://storage.googleapis.com/upload/storage/v1/b/{}/o?uploadType=media&name={}",
            bucket, key
        );

        let response = client
            .post(&url)
            .header("Content-Type", content_type)
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| MediaUploadError::UploadFailed(format!("GCS upload failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(MediaUploadError::UploadFailed(
                format!("GCS returned status: {}", response.status())
            ));
        }

        let public_url = format!("https://storage.googleapis.com/{}/{}", bucket, key);

        Ok(PlatformUploadResult {
            media_id: format!("gcs_{}", upload.id),
            url: Some(public_url),
            thumbnail_url: None,
        })
    }
}
