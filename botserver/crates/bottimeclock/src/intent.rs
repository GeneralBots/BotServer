//! Canonical request payloads for the time-clock BASIC keywords.
//!
//! `CLOCK IN`, `CLOCK OUT`, `GET BANCO HORAS`, `FERIAS BALANCE` and
//! `BANCO HORAS REPORT` only bind arguments; the shape of what they ask for
//! belongs to this crate (#1506).

use serde::{Deserialize, Serialize};

/// Which balance/report a request asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeClockRequestKind {
    ClockEntry,
    BancoHoras,
    FeriasBalance,
    BancoHorasReport,
}

impl TimeClockRequestKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ClockEntry => "clock_entry",
            Self::BancoHoras => "banco_horas",
            Self::FeriasBalance => "ferias_balance",
            Self::BancoHorasReport => "banco_horas_report",
        }
    }
}

/// Direction of a clock entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockDirection {
    In,
    Out,
}

impl ClockDirection {
    pub fn entry_type(&self) -> &'static str {
        match self {
            Self::In => "clock_in",
            Self::Out => "clock_out",
        }
    }

    pub fn action(&self) -> &'static str {
        match self {
            Self::In => "in",
            Self::Out => "out",
        }
    }
}

/// A time-clock request ready to be dispatched.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeClockRequest {
    pub kind: TimeClockRequestKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_type: Option<&'static str>,
    /// `"now"` — resolved by the runtime that dispatches the request.
    pub timestamp: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance_hours: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_overtime: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_debit: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_available: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_taken: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_remaining: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_period_start: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_hours_worked: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_overtime: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_night_shift: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_absences: Option<i64>,
}

impl TimeClockRequest {
    fn empty(kind: TimeClockRequestKind) -> Self {
        Self {
            kind,
            action: None,
            entry_type: None,
            timestamp: "now",
            person_id: None,
            period: None,
            balance_hours: None,
            pending_overtime: None,
            pending_debit: None,
            days_available: None,
            days_taken: None,
            days_remaining: None,
            next_period_start: None,
            total_hours_worked: None,
            total_overtime: None,
            total_night_shift: None,
            total_absences: None,
        }
    }

    /// `CLOCK IN`
    pub fn clock_in() -> Self {
        let mut r = Self::empty(TimeClockRequestKind::ClockEntry);
        r.action = Some(ClockDirection::In.action());
        r.entry_type = Some(ClockDirection::In.entry_type());
        r
    }

    /// `CLOCK OUT`
    pub fn clock_out() -> Self {
        let mut r = Self::empty(TimeClockRequestKind::ClockEntry);
        r.action = Some(ClockDirection::Out.action());
        r.entry_type = Some(ClockDirection::Out.entry_type());
        r
    }

    /// `GET BANCO HORAS <person>`
    pub fn banco_horas(person_id: impl Into<String>) -> Self {
        let mut r = Self::empty(TimeClockRequestKind::BancoHoras);
        r.person_id = Some(person_id.into());
        r.balance_hours = Some(0.0);
        r.pending_overtime = Some(0.0);
        r.pending_debit = Some(0.0);
        r
    }

    /// `FERIAS BALANCE <person>` — CLT statutory 30-day entitlement.
    pub fn ferias_balance(person_id: impl Into<String>) -> Self {
        let mut r = Self::empty(TimeClockRequestKind::FeriasBalance);
        r.person_id = Some(person_id.into());
        r.days_available = Some(30);
        r.days_taken = Some(0);
        r.days_remaining = Some(30);
        r.next_period_start = Some(None);
        r
    }

    /// `BANCO HORAS REPORT <person>, <period>`
    pub fn banco_horas_report(person_id: impl Into<String>, period: impl Into<String>) -> Self {
        let mut r = Self::empty(TimeClockRequestKind::BancoHorasReport);
        r.person_id = Some(person_id.into());
        r.period = Some(period.into());
        r.total_hours_worked = Some(0.0);
        r.total_overtime = Some(0.0);
        r.total_night_shift = Some(0.0);
        r.total_absences = Some(0);
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_entries_carry_direction() {
        let json = serde_json::to_value(TimeClockRequest::clock_in()).unwrap();
        assert_eq!(json["kind"], "clock_entry");
        assert_eq!(json["action"], "in");
        assert_eq!(json["entry_type"], "clock_in");
        assert_eq!(json["timestamp"], "now");

        let out = serde_json::to_value(TimeClockRequest::clock_out()).unwrap();
        assert_eq!(out["entry_type"], "clock_out");
    }

    #[test]
    fn balance_requests_start_at_zero() {
        let json = serde_json::to_value(TimeClockRequest::banco_horas("person-1")).unwrap();
        assert_eq!(json["kind"], "banco_horas");
        assert_eq!(json["person_id"], "person-1");
        assert_eq!(json["balance_hours"], 0.0);
    }

    #[test]
    fn ferias_balance_reports_thirty_days() {
        let json = serde_json::to_value(TimeClockRequest::ferias_balance("p")).unwrap();
        assert_eq!(json["days_remaining"], 30);
        assert!(json.get("next_period_start").is_some());
    }
}
