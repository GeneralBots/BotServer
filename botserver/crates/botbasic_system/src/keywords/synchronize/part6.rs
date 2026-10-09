use super::*;

impl SynchronizeService {
    pub fn new(_pool: DbPool) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            http_client,
            base_url: None,
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = Some(base_url);
        self
    }

    pub async fn synchronize(
        &self,
        config: &SynchronizeConfig,
        bot_id: Uuid,
    ) -> Result<SyncResult, SynchronizeError> {
        let started_at = Utc::now();
        let mut result = SyncResult {
            started_at,
            ..Default::default()
        };

        info!(
            "Starting synchronization: endpoint={}, table={}, key={}",
            config.endpoint, config.table_name, config.key_field
        );

        let page_size = config
            .page_size
            .unwrap_or(DEFAULT_PAGE_SIZE)
            .min(MAX_PAGE_SIZE);
        let conflict_resolution = config
            .conflict_resolution
            .clone()
            .unwrap_or_default();
        let dry_run = config.dry_run.unwrap_or(false);

        let mut current_page = 1u32;
        let mut has_more = true;

        while has_more {
            debug!("Fetching page {} from {}", current_page, config.endpoint);

            match self
                .fetch_page(config, current_page, page_size)
                .await
            {
                Ok(records) => {
                    let record_count = records.len();
                    result.records_fetched += record_count as u32;
                    result.pages_processed += 1;

                    if record_count == 0 {
                        has_more = false;
                        continue;
                    }

                    for record in records {
                        match self
                            .process_record(
                                &record,
                                config,
                                bot_id,
                                &conflict_resolution,
                                dry_run,
                            )
                            .await
                        {
                            Ok(action) => match action {
                                RecordAction::Inserted => result.records_inserted += 1,
                                RecordAction::Updated => result.records_updated += 1,
                                RecordAction::Skipped => result.records_skipped += 1,
                            },
                            Err(e) => {
                                result.records_failed += 1;
                                result.errors.push(SyncError {
                                    record_key: extract_key_value(&record, &config.key_field),
                                    page: Some(current_page),
                                    error_type: e.error_type(),
                                    message: e.to_string(),
                                    retryable: e.is_retryable(),
                                });
                            }
                        }
                    }

                    if record_count < page_size as usize {
                        has_more = false;
                    } else {
                        current_page += 1;
                    }
                }
                Err(e) => {
                    log::error!("Failed to fetch page {}: {}", current_page, e);
                    result.errors.push(SyncError {
                        record_key: None,
                        page: Some(current_page),
                        error_type: e.error_type(),
                        message: e.to_string(),
                        retryable: e.is_retryable(),
                    });

                    if !e.is_retryable() {
                        break;
                    }

                    if let Some(delay) = self.handle_retry(current_page).await {
                        tokio::time::sleep(delay).await;
                        continue;
                    } else {
                        break;
                    }
                }
            }
        }

        result.completed_at = Utc::now();
        result.duration_ms = (result.completed_at - result.started_at).num_milliseconds() as u64;
        result.success = result.errors.is_empty();

        info!(
            "Synchronization completed: fetched={}, inserted={}, updated={}, skipped={}, failed={}, duration={}ms",
            result.records_fetched,
            result.records_inserted,
            result.records_updated,
            result.records_skipped,
            result.records_failed,
            result.duration_ms
        );

        Ok(result)
    }

    async fn fetch_page(
        &self,
        config: &SynchronizeConfig,
        page: u32,
        limit: u32,
    ) -> Result<Vec<Value>, SynchronizeError> {
        let url = self.build_url(config, page, limit)?;

        let mut request = self.http_client.get(&url);

        if let Some(headers) = &config.headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        let response = request
            .send()
            .await
            .map_err(|e| SynchronizeError::FetchFailed(e.to_string()))?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(SynchronizeError::RateLimited);
        }

        if !response.status().is_success() {
            return Err(SynchronizeError::FetchFailed(format!(
                "HTTP {} from {}",
                response.status(),
                url
            )));
        }

        let body: Value = response
            .json()
            .await
            .map_err(|e| SynchronizeError::ParseFailed(e.to_string()))?;

        let records = self.extract_records(&body)?;

        Ok(records)
    }

    pub(super) fn build_url(
        &self,
        config: &SynchronizeConfig,
        page: u32,
        limit: u32,
    ) -> Result<String, SynchronizeError> {
        let base = self.base_url.as_deref().unwrap_or("");
        let endpoint = &config.endpoint;

        let url = if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
            endpoint.clone()
        } else {
            format!("{}{}", base, endpoint)
        };

        let separator = if url.contains('?') { "&" } else { "?" };

        Ok(format!(
            "{}{}{}={}&{}={}",
            url, separator, config.page_param, page, config.limit_param, limit
        ))
    }

    pub(super) fn extract_records(&self, body: &Value) -> Result<Vec<Value>, SynchronizeError> {
        if let Some(arr) = body.as_array() {
            return Ok(arr.clone());
        }

        let possible_keys = ["data", "results", "items", "records", "rows", "content"];
        for key in possible_keys {
            if let Some(arr) = body.get(key).and_then(|v| v.as_array()) {
                return Ok(arr.clone());
            }
        }

        if body.is_object() && !body.as_object().is_none_or(|o| o.is_empty()) {
            return Ok(vec![body.clone()]);
        }

        Err(SynchronizeError::ParseFailed(
            "Could not extract records from response".to_string(),
        ))
    }

    async fn process_record(
        &self,
        record: &Value,
        config: &SynchronizeConfig,
        bot_id: Uuid,
        conflict_resolution: &ConflictResolution,
        dry_run: bool,
    ) -> Result<RecordAction, SynchronizeError> {
        let transformed = self.transform_record(record, config)?;

        let key_value = extract_key_value(&transformed, &config.key_field)
            .ok_or_else(|| {
                SynchronizeError::ValidationFailed(format!(
                    "Missing key field: {}",
                    config.key_field
                ))
            })?;

        if dry_run {
            debug!("Dry run: would process record with key={}", key_value);
            return Ok(RecordAction::Skipped);
        }

        let existing = self
            .find_existing_record(bot_id, &config.table_name, &config.key_field, &key_value)
            .await?;

        match (existing, conflict_resolution) {
            (None, _) => {
                self.insert_record(bot_id, &config.table_name, &transformed)
                    .await?;
                Ok(RecordAction::Inserted)
            }
            (Some(_), ConflictResolution::Skip) => Ok(RecordAction::Skipped),
            (Some(_), ConflictResolution::Error) => {
                Err(SynchronizeError::ConflictDetected(key_value))
            }
            (Some(_), ConflictResolution::Update | ConflictResolution::Upsert) => {
                self.update_record(
                    bot_id,
                    &config.table_name,
                    &config.key_field,
                    &key_value,
                    &transformed,
                )
                .await?;
                Ok(RecordAction::Updated)
            }
        }
    }
}
