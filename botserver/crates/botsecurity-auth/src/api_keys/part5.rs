use super::*;

impl ApiKeyUsage {
    pub fn new(key_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            key_id,
            requests_this_minute: 0,
            requests_this_hour: 0,
            requests_this_day: 0,
            minute_reset_at: now + Duration::minutes(1),
            hour_reset_at: now + Duration::hours(1),
            day_reset_at: now + Duration::days(1),
            total_requests: 0,
            last_request_at: None,
        }
    }

    pub fn record_request(&mut self) {
        let now = Utc::now();

        if now >= self.minute_reset_at {
            self.requests_this_minute = 0;
            self.minute_reset_at = now + Duration::minutes(1);
        }
        if now >= self.hour_reset_at {
            self.requests_this_hour = 0;
            self.hour_reset_at = now + Duration::hours(1);
        }
        if now >= self.day_reset_at {
            self.requests_this_day = 0;
            self.day_reset_at = now + Duration::days(1);
        }

        self.requests_this_minute += 1;
        self.requests_this_hour += 1;
        self.requests_this_day += 1;
        self.total_requests += 1;
        self.last_request_at = Some(now);
    }

    pub fn is_rate_limited(&self, limits: &RateLimitConfig) -> bool {
        let now = Utc::now();

        if now < self.minute_reset_at && self.requests_this_minute >= limits.requests_per_minute {
            return true;
        }
        if now < self.hour_reset_at && self.requests_this_hour >= limits.requests_per_hour {
            return true;
        }
        if now < self.day_reset_at && self.requests_this_day >= limits.requests_per_day {
            return true;
        }

        false
    }

    pub fn time_until_reset(&self, limits: &RateLimitConfig) -> Option<Duration> {
        let now = Utc::now();

        if now < self.minute_reset_at && self.requests_this_minute >= limits.requests_per_minute {
            return Some(self.minute_reset_at - now);
        }
        if now < self.hour_reset_at && self.requests_this_hour >= limits.requests_per_hour {
            return Some(self.hour_reset_at - now);
        }
        if now < self.day_reset_at && self.requests_this_day >= limits.requests_per_day {
            return Some(self.day_reset_at - now);
        }

        None
    }
}
