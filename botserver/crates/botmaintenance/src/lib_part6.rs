use super::*;

impl CleanupService {
    pub(crate) async fn save_cleanup_history(&self, history: &CleanupHistory) -> Result<(), CleanupError> {
        let mut conn = self.pool.get().map_err(|_| CleanupError::DatabaseConnection)?;

        let trigger_str = match history.trigger {
            CleanupTrigger::Manual => "manual",
            CleanupTrigger::Scheduled => "scheduled",
            CleanupTrigger::StorageAlert => "storage_alert",
        };

        let categories_str: String = history
            .categories_cleaned
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
            INSERT INTO cleanup_history (id, organization_id, performed_at, performed_by,
            trigger, categories_cleaned, records_deleted,
            bytes_freed, retention_days_used, duration_ms,
            success, error_message)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#;

        diesel::sql_query(sql)
            .bind::<Text, _>(&history.id)
            .bind::<diesel::sql_types::Uuid, _>(history.organization_id)
            .bind::<Timestamptz, _>(history.performed_at)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Uuid>, _>(history.performed_by)
            .bind::<Text, _>(trigger_str)
            .bind::<Text, _>(&categories_str)
            .bind::<BigInt, _>(history.records_deleted)
            .bind::<BigInt, _>(history.bytes_freed)
            .bind::<BigInt, _>(history.retention_days_used)
            .bind::<BigInt, _>(history.duration_ms as i64)
            .bind::<diesel::sql_types::Bool, _>(history.success)
            .bind::<diesel::sql_types::Nullable<Text>, _>(history.error_message.as_deref())
            .execute(&mut conn)
            .map_err(|e| {
                log::error!("Failed to save cleanup history: {e}");
                CleanupError::HistorySaveFailed
            })?;

        Ok(())
    }

    pub(crate) async fn get_organization_quota(&self, organization_id: Uuid) -> Result<i64, CleanupError> {
        let mut conn = self.pool.get().map_err(|_| CleanupError::DatabaseConnection)?;

        #[derive(QueryableByName)]
        struct QuotaRow {
            #[diesel(sql_type = BigInt)]
            storage_quota_bytes: i64,
        }

        let sql = "SELECT storage_quota_bytes FROM organizations WHERE id = $1";

        let result: Option<QuotaRow> = diesel::sql_query(sql)
            .bind::<diesel::sql_types::Uuid, _>(organization_id)
            .load(&mut conn)
            .ok()
            .and_then(|r: Vec<QuotaRow>| r.into_iter().next());

        result
            .map(|r| r.storage_quota_bytes)
            .ok_or(CleanupError::OrganizationNotFound)
    }

    pub(super) fn validate_retention_days(&self, days: i64) -> Result<i64, CleanupError> {
        if days < MIN_RETENTION_DAYS {
            return Err(CleanupError::RetentionTooShort(MIN_RETENTION_DAYS));
        }
        if days > MAX_RETENTION_DAYS {
            return Err(CleanupError::RetentionTooLong(MAX_RETENTION_DAYS));
        }
        Ok(days)
    }
}
