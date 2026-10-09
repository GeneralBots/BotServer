use super::*;

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            daily_retention_days: 7,
            weekly_retention_weeks: 4,
            monthly_retention_months: 12,
            yearly_retention_years: 3,
            minimum_backups: 3,
            maximum_backups: Some(100),
        }
    }
}
