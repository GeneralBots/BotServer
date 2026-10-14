use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TimeEvent {
    CheckIn,
    CheckOut,
    BreakStart,
    BreakEnd,
}

impl TimeEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CheckIn => "check_in",
            Self::CheckOut => "check_out",
            Self::BreakStart => "break_start",
            Self::BreakEnd => "break_end",
        }
    }

    pub fn from_label(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "check_out" => Self::CheckOut,
            "break_start" => Self::BreakStart,
            "break_end" => Self::BreakEnd,
            _ => Self::CheckIn,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRecord {
    pub id: Uuid,
    pub employee_id: String,
    pub event_type: TimeEvent,
    pub timestamp: DateTime<Utc>,
    pub location: Option<String>,
    pub notes: Option<String>,
}

impl TimeRecord {
    pub fn new(employee_id: &str, event: TimeEvent) -> Self {
        Self {
            id: Uuid::new_v4(),
            employee_id: employee_id.to_string(),
            event_type: event,
            timestamp: Utc::now(),
            location: None,
            notes: None,
        }
    }

    pub fn with_location(mut self, location: &str) -> Self {
        self.location = Some(location.to_string());
        self
    }

    pub fn with_notes(mut self, notes: &str) -> Self {
        self.notes = Some(notes.to_string());
        self
    }
}

pub struct TimeTracker {
    records: Vec<TimeRecord>,
}

impl TimeTracker {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    pub fn record(&mut self, employee_id: &str, event: TimeEvent) -> &TimeRecord {
        let idx = self.records.len();
        self.records.push(TimeRecord::new(employee_id, event));
        &self.records[idx]
    }

    pub fn record_with_details(
        &mut self,
        employee_id: &str,
        event: TimeEvent,
        location: Option<&str>,
        notes: Option<&str>,
    ) -> &TimeRecord {
        let idx = self.records.len();
        let mut rec = TimeRecord::new(employee_id, event);
        rec.location = location.map(|l| l.to_string());
        rec.notes = notes.map(|n| n.to_string());
        self.records.push(rec);
        &self.records[idx]
    }

    pub fn get_today_records(&self, employee_id: &str) -> Vec<&TimeRecord> {
        let today_start = Utc::now().date_naive().and_hms_opt(0, 0, 0)
            .map(|d| d.and_utc())
            .unwrap_or(Utc::now());
        self.records.iter()
            .filter(|r| r.employee_id == employee_id && r.timestamp >= today_start)
            .collect()
    }

    pub fn get_records_in_range(
        &self,
        employee_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<&TimeRecord> {
        self.records.iter()
            .filter(|r| r.employee_id == employee_id && r.timestamp >= start && r.timestamp <= end)
            .collect()
    }

    pub fn daily_report(&self, employee_id: &str) -> String {
        let records = self.get_today_records(employee_id);
        if records.is_empty() {
            return format!("No records for {} today.", employee_id);
        }

        let mut report = String::new();
        report.push_str(&format!("=== Daily Report: {} ===\n", employee_id));
        for rec in &records {
            let event_str = rec.event_type.as_str();
            let time_str = rec.timestamp.format("%H:%M:%S").to_string();
            report.push_str(&format!("  {} - {}\n", time_str, event_str));
            if let Some(loc) = &rec.location {
                report.push_str(&format!("    Location: {}\n", loc));
            }
        }

        let total_seconds = self.calculate_work_hours(employee_id);
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        report.push_str(&format!("\nTotal work time: {}h {}m\n", hours, minutes));
        report
    }

    fn calculate_work_hours(&self, employee_id: &str) -> i64 {
        let records = self.get_today_records(employee_id);
        let mut total_seconds: i64 = 0;
        let mut last_check_in: Option<DateTime<Utc>> = None;
        let mut on_break = false;

        for rec in &records {
            match rec.event_type {
                TimeEvent::CheckIn => {
                    last_check_in = Some(rec.timestamp);
                }
                TimeEvent::CheckOut => {
                    if let Some(check_in) = last_check_in {
                        total_seconds += (rec.timestamp - check_in).num_seconds();
                        last_check_in = None;
                    }
                }
                TimeEvent::BreakStart => {
                    if let Some(check_in) = last_check_in {
                        let worked = (rec.timestamp - check_in).num_seconds();
                        total_seconds += worked;
                    }
                    on_break = true;
                    last_check_in = None;
                }
                TimeEvent::BreakEnd => {
                    on_break = false;
                    last_check_in = Some(rec.timestamp);
                }
            }
        }

        if let (Some(check_in), false) = (last_check_in, on_break) {
            total_seconds += (Utc::now() - check_in).num_seconds();
        }

        total_seconds.max(0)
    }
}

impl Default for TimeTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_new() {
        let r = TimeRecord::new("emp001", TimeEvent::CheckIn);
        assert_eq!(r.employee_id, "emp001");
        assert_eq!(r.event_type, TimeEvent::CheckIn);
    }

    #[test]
    fn test_tracker_flow() {
        let mut tt = TimeTracker::new();
        tt.record("emp001", TimeEvent::CheckIn);
        tt.record("emp001", TimeEvent::CheckOut);
        let records = tt.get_today_records("emp001");
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn test_daily_report() {
        let mut tt = TimeTracker::new();
        tt.record("emp001", TimeEvent::CheckIn);
        let report = tt.daily_report("emp001");
        assert!(report.contains("check_in"));
        assert!(report.contains("emp001"));
    }
}
