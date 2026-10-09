use super::*;

impl CleanupService {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    pub async fn preview_cleanup(
        &self,
        organization_id: Uuid,
        retention_days: Option<i64>,
        categories: Option<Vec<CleanupCategory>>,
    ) -> Result<CleanupPreview, CleanupError> {
        let retention = self.validate_retention_days(retention_days.unwrap_or(DEFAULT_RETENTION_DAYS))?;
        let cutoff_date = Utc::now() - Duration::days(retention);
        let cats = categories.unwrap_or_else(CleanupCategory::default_enabled);

        let mut category_previews = Vec::new();
        let mut total_records = 0i64;
        let mut total_bytes = 0i64;

        for category in cats {
            let preview = self.preview_category(organization_id, category, cutoff_date).await?;
            total_records += preview.record_count;
            total_bytes += preview.estimated_bytes;
            category_previews.push(preview);
        }

        Ok(CleanupPreview {
            categories: category_previews,
            total_records,
            estimated_bytes_freed: total_bytes,
            estimated_bytes_freed_formatted: format_bytes(total_bytes),
            cutoff_date,
        })
    }

    async fn preview_category(
        &self,
        organization_id: Uuid,
        category: CleanupCategory,
        cutoff_date: DateTime<Utc>,
    ) -> Result<CategoryPreview, CleanupError> {
        let mut conn = self.pool.get().map_err(|e| {
            log::error!("Failed to get database connection: {e}");
            CleanupError::DatabaseConnection
        })?;

        let table = category.table_name();
        let ts_col = category.timestamp_column();

        let count_sql = format!(
            "SELECT COUNT(*) as count FROM {table} WHERE organization_id = $1 AND {ts_col} < $2"
        );

        let count_result: Vec<CountRow> = diesel::sql_query(&count_sql)
            .bind::<diesel::sql_types::Uuid, _>(organization_id)
            .bind::<Timestamptz, _>(cutoff_date)
            .load(&mut conn)
            .unwrap_or_default();

        let record_count = count_result.first().map(|r| r.count).unwrap_or(0);

        let size_sql = format!(
            "SELECT COALESCE(pg_total_relation_size('{table}') * \
            (SELECT COUNT(*) FROM {table} WHERE organization_id = $1 AND {ts_col} < $2)::float / \
            NULLIF((SELECT COUNT(*) FROM {table} WHERE organization_id = $1), 0), 0)::bigint as size_bytes"
        );

        let size_result: Vec<SizeRow> = diesel::sql_query(&size_sql)
            .bind::<diesel::sql_types::Uuid, _>(organization_id)
            .bind::<Timestamptz, _>(cutoff_date)
            .load(&mut conn)
            .unwrap_or_default();

        let estimated_bytes = size_result.first().map(|r| r.size_bytes).unwrap_or(0);

        let oldest_sql = format!(
            "SELECT MIN({ts_col}) as oldest FROM {table} WHERE organization_id = $1 AND {ts_col} < $2"
        );

        let oldest_result: Result<Vec<OldestRow>, _> = diesel::sql_query(&oldest_sql)
            .bind::<diesel::sql_types::Uuid, _>(organization_id)
            .bind::<Timestamptz, _>(cutoff_date)
            .load(&mut conn);

        let oldest_record = oldest_result.ok().and_then(|r| r.first().map(|o| o.oldest));

        Ok(CategoryPreview {
            category,
            display_name: category.to_string(),
            record_count,
            estimated_bytes,
            estimated_bytes_formatted: format_bytes(estimated_bytes),
            oldest_record,
            is_essential: category.is_essential(),
        })
    }

    pub async fn execute_cleanup(
        &self,
        organization_id: Uuid,
        retention_days: Option<i64>,
        categories: Option<Vec<CleanupCategory>>,
        performed_by: Option<Uuid>,
        trigger: CleanupTrigger,
    ) -> Result<CleanupResult, CleanupError> {
        let start_time = std::time::Instant::now();
        let retention = self.validate_retention_days(retention_days.unwrap_or(DEFAULT_RETENTION_DAYS))?;
        let cutoff_date = Utc::now() - Duration::days(retention);
        let cats = categories.unwrap_or_else(CleanupCategory::default_enabled);

        info!(
            "Starting cleanup for org {} with {} day retention ({} categories)",
            organization_id,
            retention,
            cats.len()
        );

        let mut category_results = Vec::new();
        let mut total_deleted = 0i64;
        let mut total_bytes = 0i64;
        let mut errors = Vec::new();

        for category in &cats {
            match self.cleanup_category(organization_id, *category, cutoff_date).await {
                Ok(result) => {
                    total_deleted += result.records_deleted;
                    total_bytes += result.bytes_freed;
                    category_results.push(result);
                }
                Err(e) => {
                    let error_msg = format!("{}: {e}", category);
                    errors.push(error_msg.clone());
                    category_results.push(CategoryResult {
                        category: *category,
                        display_name: category.to_string(),
                        records_deleted: 0,
                        bytes_freed: 0,
                        success: false,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let success = errors.is_empty();

        let history = CleanupHistory {
            id: Uuid::new_v4().to_string(),
            organization_id,
            performed_at: Utc::now(),
            performed_by,
            trigger,
            categories_cleaned: cats.clone(),
            records_deleted: total_deleted,
            bytes_freed: total_bytes,
            retention_days_used: retention,
            duration_ms,
            success,
            error_message: if errors.is_empty() {
                None
            } else {
                Some(errors.join("; "))
            },
        };

        if let Err(e) = self.save_cleanup_history(&history).await {
            warn!("Failed to save cleanup history: {e}");
        }

        info!(
            "Cleanup completed for org {}: deleted {} records, freed {} in {}ms",
            organization_id,
            total_deleted,
            format_bytes(total_bytes),
            duration_ms
        );

        Ok(CleanupResult {
            success,
            categories_cleaned: category_results,
            total_records_deleted: total_deleted,
            bytes_freed: total_bytes,
            bytes_freed_formatted: format_bytes(total_bytes),
            duration_ms,
            errors,
        })
    }

    async fn cleanup_category(
        &self,
        organization_id: Uuid,
        category: CleanupCategory,
        cutoff_date: DateTime<Utc>,
    ) -> Result<CategoryResult, CleanupError> {
        let mut conn = self.pool.get().map_err(|_| CleanupError::DatabaseConnection)?;

        let table = category.table_name();
        let ts_col = category.timestamp_column();

        let size_before: i64 = diesel::sql_query(format!(
            "SELECT pg_total_relation_size('{table}') as size_bytes"
        ))
        .load::<SizeRow>(&mut conn)
        .ok()
        .and_then(|r| r.first().map(|s| s.size_bytes))
        .unwrap_or(0);

        let delete_sql = format!(
            "DELETE FROM {table} WHERE organization_id = $1 AND {ts_col} < $2"
        );

        let deleted = diesel::sql_query(&delete_sql)
            .bind::<diesel::sql_types::Uuid, _>(organization_id)
            .bind::<Timestamptz, _>(cutoff_date)
            .execute(&mut conn)
            .map_err(|e| {
                log::error!("Failed to cleanup {}: {e}", category);
                CleanupError::CleanupFailed(category.to_string())
            })?;

        let size_after: i64 = diesel::sql_query(format!(
            "SELECT pg_total_relation_size('{table}') as size_bytes"
        ))
        .load::<SizeRow>(&mut conn)
        .ok()
        .and_then(|r| r.first().map(|s| s.size_bytes))
        .unwrap_or(0);

        let bytes_freed = (size_before - size_after).max(0);

        debug!(
            "Cleaned {} from {}: {} records, {} freed",
            category,
            table,
            deleted,
            format_bytes(bytes_freed)
        );

        Ok(CategoryResult {
            category,
            display_name: category.to_string(),
            records_deleted: deleted as i64,
            bytes_freed,
            success: true,
            error: None,
        })
    }
}
