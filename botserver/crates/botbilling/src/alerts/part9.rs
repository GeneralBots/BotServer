use super::*;

impl QuietHours {
    pub fn is_active(&self) -> bool {
        let now = Utc::now();
        let hour = now.format("%H").to_string().parse::<u8>().unwrap_or(0);

        if self.start_hour < self.end_hour {
            hour >= self.start_hour && hour < self.end_hour
        } else {
            hour >= self.start_hour || hour < self.end_hour
        }
    }
}
