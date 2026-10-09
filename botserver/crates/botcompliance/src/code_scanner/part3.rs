use super::*;

impl ScanStats {
    pub fn add_issue(&mut self, severity: &IssueSeverity) {
        match severity {
            IssueSeverity::Critical => self.critical += 1,
            IssueSeverity::High => self.high += 1,
            IssueSeverity::Medium => self.medium += 1,
            IssueSeverity::Low => self.low += 1,
            IssueSeverity::Info => self.info += 1,
        }
        self.total += 1;
    }

    pub fn merge(&mut self, other: &Self) {
        self.critical += other.critical;
        self.high += other.high;
        self.medium += other.medium;
        self.low += other.low;
        self.info += other.info;
        self.total += other.total;
    }
}
