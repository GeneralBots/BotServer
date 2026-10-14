use chrono::{NaiveDate, Utc};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::models::Attendance;

type Storage = Arc<Mutex<HashMap<Uuid, Attendance>>>;

#[derive(Clone)]
pub struct AttendanceService {
    storage: Storage,
}

impl AttendanceService {
    pub fn new() -> Self {
        AttendanceService {
            storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn clock_in(&self, employee_id: Uuid) -> Result<Attendance, String> {
        let today = Utc::now().date_naive();
        let store = self.storage.lock().map_err(|e| format!("Lock error: {e}"))?;
        let already = store.values().any(|a| a.employee_id == employee_id && a.date == today && a.clock_out.is_none());
        if already {
            return Err("Employee already clocked in today".to_string());
        }
        drop(store);
        let id = Uuid::new_v4();
        let now = Utc::now();
        let attendance = Attendance {
            id,
            employee_id,
            date: today,
            clock_in: Some(now),
            clock_out: None,
            hours_worked: 0.0,
            overtime: 0.0,
        };
        let mut store = self.storage.lock().map_err(|e| format!("Lock error: {e}"))?;
        store.insert(id, attendance.clone());
        Ok(attendance)
    }

    pub fn clock_out(&self, employee_id: Uuid) -> Result<Attendance, String> {
        let today = Utc::now().date_naive();
        let mut store = self.storage.lock().map_err(|e| format!("Lock error: {e}"))?;
        let record = store
            .values_mut()
            .find(|a| a.employee_id == employee_id && a.date == today && a.clock_out.is_none())
            .ok_or_else(|| "No active clock-in found for today".to_string())?;
        let now = Utc::now();
        record.clock_out = Some(now);
        if let Some(clock_in) = record.clock_in {
            let duration = now.signed_duration_since(clock_in);
            let hours = duration.num_minutes() as f64 / 60.0;
            record.hours_worked = hours.min(24.0);
            record.overtime = if hours > 8.0 { hours - 8.0 } else { 0.0 };
        }
        Ok(record.clone())
    }

    pub fn get_daily_summary(&self, employee_id: Uuid, date: NaiveDate) -> Result<Attendance, String> {
        let store = self.storage.lock().map_err(|e| format!("Lock error: {e}"))?;
        store
            .values()
            .find(|a| a.employee_id == employee_id && a.date == date)
            .cloned()
            .ok_or_else(|| format!("No attendance record for {employee_id} on {date}"))
    }

    pub fn get_range(&self, employee_id: Uuid, from: NaiveDate, to: NaiveDate) -> Result<Vec<Attendance>, String> {
        let store = self.storage.lock().map_err(|e| format!("Lock error: {e}"))?;
        let mut records: Vec<Attendance> = store
            .values()
            .filter(|a| a.employee_id == employee_id && a.date >= from && a.date <= to)
            .cloned()
            .collect();
        records.sort_by_key(|a| a.date);
        Ok(records)
    }

    pub fn calculate_hours_in_range(&self, employee_id: Uuid, from: NaiveDate, to: NaiveDate) -> Result<f64, String> {
        let records = self.get_range(employee_id, from, to)?;
        let total: f64 = records.iter().map(|a| a.hours_worked).sum();
        Ok(total)
    }

    pub fn calculate_overtime_in_range(&self, employee_id: Uuid, from: NaiveDate, to: NaiveDate) -> Result<f64, String> {
        let records = self.get_range(employee_id, from, to)?;
        let total: f64 = records.iter().map(|a| a.overtime).sum();
        Ok(total)
    }
}

impl Default for AttendanceService {
    fn default() -> Self {
        Self::new()
    }
}
