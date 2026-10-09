use super::*;

impl CleanupService {
    pub async fn get_storage_usage(&self, organization_id: Uuid) -> Result<StorageUsage, CleanupError> {
        let mut conn = self.pool.get().map_err(|_| CleanupError::DatabaseConnection)?;

        let mut categories = Vec::new();
        let mut total_bytes = 0i64;

        for category in CleanupCategory::all() {
            let table = category.table_name();
            let _ts_col = category.timestamp_column();

            let count_sql = format!(
                "SELECT COUNT(*) as count FROM {table} WHERE organization_id = $1"
            );

            let count: i64 = diesel::sql_query(&count_sql)
                .bind::<diesel::sql_types::Uuid, _>(organization_id)
                .load::<CountRow>(&mut conn)
                .ok()
                .and_then(|r| r.first().map(|c| c.count))
                .unwrap_or(0);

            let size_sql = format!(
                "SELECT COALESCE(pg_total_relation_size('{table}') * \
                (SELECT COUNT(*) FROM {table} WHERE organization_id = $1)::float / \
                NULLIF((SELECT COUNT(*) FROM {table}), 0), 0)::bigint as size_bytes"
            );

            let bytes: i64 = diesel::sql_query(&size_sql)
                .bind::<diesel::sql_types::Uuid, _>(organization_id)
                .load::<SizeRow>(&mut conn)
                .ok()
                .and_then(|r| r.first().map(|s| s.size_bytes))
                .unwrap_or(0);

            total_bytes += bytes;

            categories.push(CategoryStorage {
                category,
                display_name: category.to_string(),
                bytes_used: bytes,
                bytes_formatted: format_bytes(bytes),
                record_count: count,
                percent_of_total: 0.0,
            });
        }

        for cat in &mut categories {
            cat.percent_of_total = if total_bytes > 0 {
                (cat.bytes_used as f32 / total_bytes as f32) * 100.0
            } else {
                0.0
            };
        }

        categories.sort_by(|a, b| b.bytes_used.cmp(&a.bytes_used));

        let quota = self.get_organization_quota(organization_id).await.ok();
        let quota_percent = quota.map(|q| (total_bytes as f32 / q as f32) * 100.0);

        Ok(StorageUsage {
            organization_id,
            categories,
            total_bytes,
            total_bytes_formatted: format_bytes(total_bytes),
            quota_bytes: quota,
            quota_percent_used: quota_percent,
            last_updated: Utc::now(),
        })
    }

    pub async fn get_cleanup_history(
        &self,
        organization_id: Uuid,
        limit: Option<i32>,
    ) -> Result<Vec<CleanupHistory>, CleanupError> {
        let mut conn = self.pool.get().map_err(|_| CleanupError::DatabaseConnection)?;

        let limit = limit.unwrap_or(50).min(100);

        #[derive(QueryableByName)]
        struct HistoryRow {
            #[diesel(sql_type = Text)]
            id: String,
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            organization_id: Uuid,
            #[diesel(sql_type = Timestamptz)]
            performed_at: DateTime<Utc>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
            performed_by: Option<Uuid>,
            #[diesel(sql_type = Text)]
            trigger: String,
            #[diesel(sql_type = Text)]
            categories_cleaned: String,
            #[diesel(sql_type = BigInt)]
            records_deleted: i64,
            #[diesel(sql_type = BigInt)]
            bytes_freed: i64,
            #[diesel(sql_type = BigInt)]
            retention_days_used: i64,
            #[diesel(sql_type = BigInt)]
            duration_ms: i64,
            #[diesel(sql_type = diesel::sql_types::Bool)]
            success: bool,
            #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
            error_message: Option<String>,
        }

        let sql = r#"
            SELECT id, organization_id, performed_at, performed_by, trigger,
            categories_cleaned, records_deleted, bytes_freed,
            retention_days_used, duration_ms, success, error_message
            FROM cleanup_history
            WHERE organization_id = $1
            ORDER BY performed_at DESC
            LIMIT $2
            "#;

        let rows: Vec<HistoryRow> = diesel::sql_query(sql)
            .bind::<diesel::sql_types::Uuid, _>(organization_id)
            .bind::<diesel::sql_types::Integer, _>(limit)
            .load(&mut conn)
            .unwrap_or_default();

        let history = rows
            .into_iter()
            .map(|row| {
                let trigger = match row.trigger.as_str() {
                    "scheduled" => CleanupTrigger::Scheduled,
                    "storage_alert" => CleanupTrigger::StorageAlert,
                    _ => CleanupTrigger::Manual,
                };

                let categories: Vec<CleanupCategory> = row
                    .categories_cleaned
                    .split(',')
                    .filter_map(|s| match s.trim() {
                        "bot_history" => Some(CleanupCategory::BotHistory),
                        "chat_sessions" => Some(CleanupCategory::ChatSessions),
                        "system_logs" => Some(CleanupCategory::SystemLogs),
                        "access_logs" => Some(CleanupCategory::AccessLogs),
                        "debug_logs" => Some(CleanupCategory::DebugLogs),
                        "temp_files" => Some(CleanupCategory::TempFiles),
                        "cache_data" => Some(CleanupCategory::CacheData),
                        "analytics_raw" => Some(CleanupCategory::AnalyticsRaw),
                        _ => None,
                    })
                    .collect();

                CleanupHistory {
                    id: row.id,
                    organization_id: row.organization_id,
                    performed_at: row.performed_at,
                    performed_by: row.performed_by,
                    trigger,
                    categories_cleaned: categories,
                    records_deleted: row.records_deleted,
                    bytes_freed: row.bytes_freed,
                    retention_days_used: row.retention_days_used,
                    duration_ms: row.duration_ms as u64,
                    success: row.success,
                    error_message: row.error_message,
                }
            })
            .collect();

        Ok(history)
    }

    pub async fn get_cleanup_config(&self, organization_id: Uuid) -> Result<CleanupConfig, CleanupError> {
        let mut conn = self.pool.get().map_err(|_| CleanupError::DatabaseConnection)?;

        #[derive(QueryableByName)]
        struct ConfigRow {
            #[diesel(sql_type = BigInt)]
            retention_days: i64,
            #[diesel(sql_type = Text)]
            enabled_categories: String,
            #[diesel(sql_type = diesel::sql_types::Bool)]
            auto_cleanup_enabled: bool,
            #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
            auto_cleanup_schedule: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Integer)]
            storage_alert_threshold: i32,
        }

        let sql = r#"
            SELECT retention_days, enabled_categories, auto_cleanup_enabled,
            auto_cleanup_schedule, storage_alert_threshold
            FROM cleanup_config
            WHERE organization_id = $1
            "#;

        let result: Option<ConfigRow> = diesel::sql_query(sql)
            .bind::<diesel::sql_types::Uuid, _>(organization_id)
            .load(&mut conn)
            .ok()
            .and_then(|r: Vec<ConfigRow>| r.into_iter().next());

        match result {
            Some(row) => {
                let categories: Vec<CleanupCategory> = row
                    .enabled_categories
                    .split(',')
                    .filter_map(|s| match s.trim() {
                        "bot_history" => Some(CleanupCategory::BotHistory),
                        "chat_sessions" => Some(CleanupCategory::ChatSessions),
                        "system_logs" => Some(CleanupCategory::SystemLogs),
                        "access_logs" => Some(CleanupCategory::AccessLogs),
                        "debug_logs" => Some(CleanupCategory::DebugLogs),
                        "temp_files" => Some(CleanupCategory::TempFiles),
                        "cache_data" => Some(CleanupCategory::CacheData),
                        "analytics_raw" => Some(CleanupCategory::AnalyticsRaw),
                        _ => None,
                    })
                    .collect();

                Ok(CleanupConfig {
                    organization_id,
                    retention_days: row.retention_days,
                    enabled_categories: categories,
                    auto_cleanup_enabled: row.auto_cleanup_enabled,
                    auto_cleanup_schedule: row.auto_cleanup_schedule,
                    storage_alert_threshold_percent: row.storage_alert_threshold as u8,
                })
            }
            None => Ok(CleanupConfig {
                organization_id,
                ..Default::default()
            }),
        }
    }

    pub async fn save_cleanup_config(&self, config: &CleanupConfig) -> Result<(), CleanupError> {
        let mut conn = self.pool.get().map_err(|_| CleanupError::DatabaseConnection)?;

        let categories_str: String = config
            .enabled_categories
            .iter()
            .map(|c| match c {
                CleanupCategory::BotHistory => "bot_history",
                CleanupCategory::ChatSessions => "chat_sessions",
                CleanupCategory::SystemLogs => "system_logs",
                CleanupCategory::AccessLogs => "access_logs",
                CleanupCategory::DebugLogs => "debug_logs",
                CleanupCategory::TempFiles => "temp_files",
                CleanupCategory::CacheData => "cache_data",
                CleanupCategory::AnalyticsRaw => "analytics_raw",
                CleanupCategory::OldCalendarEvents => "old_calendar_events",
                CleanupCategory::Attachments => "attachments",
            })
            .collect::<Vec<_>>()
            .join(",");

        let sql = r#"
            INSERT INTO cleanup_config (organization_id, retention_days, enabled_categories,
            auto_cleanup_enabled, auto_cleanup_schedule,
            storage_alert_threshold, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            ON CONFLICT (organization_id) DO UPDATE SET
            retention_days = EXCLUDED.retention_days,
            enabled_categories = EXCLUDED.enabled_categories,
            auto_cleanup_enabled = EXCLUDED.auto_cleanup_enabled,
            auto_cleanup_schedule = EXCLUDED.auto_cleanup_schedule,
            storage_alert_threshold = EXCLUDED.storage_alert_threshold,
            updated_at = NOW()
            "#;

        diesel::sql_query(sql)
            .bind::<diesel::sql_types::Uuid, _>(config.organization_id)
            .bind::<BigInt, _>(config.retention_days)
            .bind::<Text, _>(&categories_str)
            .bind::<diesel::sql_types::Bool, _>(config.auto_cleanup_enabled)
            .bind::<diesel::sql_types::Nullable<Text>, _>(config.auto_cleanup_schedule.as_deref())
            .bind::<diesel::sql_types::Integer, _>(config.storage_alert_threshold_percent as i32)
            .execute(&mut conn)
            .map_err(|e| {
                log::error!("Failed to save cleanup config: {e}");
                CleanupError::ConfigSaveFailed
            })?;

        Ok(())
    }
}
