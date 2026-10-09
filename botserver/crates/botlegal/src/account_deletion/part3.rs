use super::*;

impl DataExportService {
    pub fn new(download_expiry_hours: u32) -> Self {
        Self {
            requests: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            download_expiry_hours,
        }
    }

    pub async fn create_export_request(
        &self,
        user_id: Uuid,
        format: ExportFormat,
    ) -> DataExportRequest {
        let request = DataExportRequest {
            id: Uuid::new_v4(),
            user_id,
            requested_at: Utc::now(),
            status: ExportStatus::Pending,
            format,
            download_url: None,
            expires_at: None,
            completed_at: None,
        };

        let mut requests = self.requests.write().await;
        requests.insert(request.id, request.clone());

        request
    }

    pub async fn complete_export(
        &self,
        request_id: Uuid,
        download_url: String,
    ) -> Option<DataExportRequest> {
        let mut requests = self.requests.write().await;
        let request = requests.get_mut(&request_id)?;

        let now = Utc::now();
        request.status = ExportStatus::Ready;
        request.download_url = Some(download_url);
        request.completed_at = Some(now);
        request.expires_at = Some(now + chrono::Duration::hours(i64::from(self.download_expiry_hours)));

        Some(request.clone())
    }

    pub async fn get_export_request(&self, request_id: Uuid) -> Option<DataExportRequest> {
        let requests = self.requests.read().await;
        requests.get(&request_id).cloned()
    }

    pub async fn get_user_export_requests(&self, user_id: Uuid) -> Vec<DataExportRequest> {
        let requests = self.requests.read().await;
        requests
            .values()
            .filter(|r| r.user_id == user_id)
            .cloned()
            .collect()
    }
}
impl Default for DataExportService {
    fn default() -> Self {
        Self::new(48)
    }
}
