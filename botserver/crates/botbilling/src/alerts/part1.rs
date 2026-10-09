use super::*;

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            warning: 80.0,
            critical: 90.0,
            exceeded: 100.0,
        }
    }
}
impl AlertThresholds {
    pub fn new(warning: f64, critical: f64, exceeded: f64) -> Self {
        Self {
            warning,
            critical,
            exceeded,
        }
    }

    pub fn get_severity(&self, percentage: f64) -> Option<AlertSeverity> {
        if percentage >= self.exceeded {
            Some(AlertSeverity::Exceeded)
        } else if percentage >= self.critical {
            Some(AlertSeverity::Critical)
        } else if percentage >= self.warning {
            Some(AlertSeverity::Warning)
        } else {
            None
        }
    }
}
