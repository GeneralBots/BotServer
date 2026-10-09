use super::*;

impl UserSecurityProfile {
    pub fn new(user_id: Uuid) -> Self {
        Self {
            user_id,
            known_ips: Vec::new(),
            known_devices: Vec::new(),
            known_locations: Vec::new(),
            last_login: None,
            last_location: None,
            login_times: Vec::new(),
            risk_score: 0.0,
            is_locked: false,
            lock_expires_at: None,
        }
    }

    pub fn is_known_ip(&self, ip: &str) -> bool {
        self.known_ips.contains(&ip.to_string())
    }

    pub fn is_known_device(&self, device: &str) -> bool {
        self.known_devices.contains(&device.to_string())
    }

    pub fn add_known_ip(&mut self, ip: &str) {
        if !self.is_known_ip(ip) {
            self.known_ips.push(ip.to_string());
            if self.known_ips.len() > 100 {
                self.known_ips.remove(0);
            }
        }
    }

    pub fn add_known_device(&mut self, device: &str) {
        if !self.is_known_device(device) {
            self.known_devices.push(device.to_string());
            if self.known_devices.len() > 50 {
                self.known_devices.remove(0);
            }
        }
    }

    pub fn record_login(&mut self, location: Option<GeoLocation>) {
        let now = Utc::now();
        self.last_login = Some(now);
        self.login_times.push(now.hour());
        if self.login_times.len() > 1000 {
            self.login_times.remove(0);
        }
        if let Some(loc) = location {
            self.last_location = Some(loc.clone());
            self.known_locations.push(loc);
            if self.known_locations.len() > 50 {
                self.known_locations.remove(0);
            }
        }
    }

    pub fn is_unusual_login_time(&self, hour: u32) -> bool {
        if self.login_times.len() < 10 {
            return false;
        }

        let count = self.login_times.iter().filter(|&&h| h == hour).count();
        let percentage = count as f64 / self.login_times.len() as f64;

        percentage < 0.01
    }

    pub fn lock(&mut self, duration: Duration) {
        self.is_locked = true;
        self.lock_expires_at = Some(Utc::now() + duration);
    }

    pub fn unlock(&mut self) {
        self.is_locked = false;
        self.lock_expires_at = None;
    }

    pub fn check_lock_status(&mut self) -> bool {
        if self.is_locked {
            if let Some(expires) = self.lock_expires_at {
                if Utc::now() > expires {
                    self.unlock();
                    return false;
                }
            }
            return true;
        }
        false
    }
}
