use super::*;

impl InsightsService {
    pub async fn update_settings(
        &self,
        user_id: Uuid,
        req: UpdateSettingsRequest,
    ) -> Result<InsightsSettings, InsightsError> {
        let mut settings = self.get_settings(user_id).await?;

        if let Some(enabled) = req.tracking_enabled {
            settings.tracking_enabled = enabled;
        }
        if let Some(daily) = req.daily_digest {
            settings.daily_digest = daily;
        }
        if let Some(weekly) = req.weekly_digest {
            settings.weekly_digest = weekly;
        }
        if let Some(time) = req.digest_time {
            settings.digest_time = time;
        }
        if let Some(goals) = req.goals {
            settings.goals = Some(goals);
        }

        Ok(settings)
    }

    pub async fn update_focus_mode(
        &self,
        user_id: Uuid,
        req: UpdateFocusModeRequest,
    ) -> Result<FocusMode, InsightsError> {
        Ok(FocusMode {
            id: Uuid::new_v4(),
            user_id,
            enabled: req.enabled,
            schedule: req.schedule,
            block_notifications: req.block_notifications.unwrap_or(true),
            auto_decline_meetings: req.auto_decline_meetings.unwrap_or(false),
            status_message: req.status_message,
        })
    }

    pub async fn get_app_breakdown(
        &self,
        _user_id: Uuid,
        _date: NaiveDate,
    ) -> Result<Vec<AppUsageSummary>, InsightsError> {
        Ok(vec![
            AppUsageSummary {
                app_name: "Chat".to_string(),
                total_seconds: 3600,
                percentage: 25.0,
                sessions: 15,
            },
            AppUsageSummary {
                app_name: "Mail".to_string(),
                total_seconds: 2700,
                percentage: 18.75,
                sessions: 8,
            },
            AppUsageSummary {
                app_name: "Documents".to_string(),
                total_seconds: 5400,
                percentage: 37.5,
                sessions: 5,
            },
            AppUsageSummary {
                app_name: "Calendar".to_string(),
                total_seconds: 1800,
                percentage: 12.5,
                sessions: 10,
            },
            AppUsageSummary {
                app_name: "Tasks".to_string(),
                total_seconds: 900,
                percentage: 6.25,
                sessions: 12,
            },
        ])
    }
}
