use super::*;

impl Default for Budget {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            daily_limit: 100.0,
            monthly_limit: 2000.0,
            alert_threshold: 0.8,
            daily_spend: 0.0,
            monthly_spend: 0.0,
            daily_reset_date: now,
            monthly_reset_date: now,
            daily_alert_sent: false,
            monthly_alert_sent: false,
        }
    }
}
