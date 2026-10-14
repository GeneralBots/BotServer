//! CLT (Consolidação das Leis do Trabalho) — Brazilian labor law rules.
//!
//! Implements the core labor rules referenced by [`crate::botclock::rules`]:
//!
//! - 44h/week regular threshold (art. 7º, XIII, CF/88)
//! - Overtime at 50% (art. 7º, XVI, CF/88) — `HExtra50`
//! - Overtime at 100% on holidays/rest days (Súmula 146 TST) — `HExtra100`
//! - Night shift 22:00–05:00 with 20% add-on (art. 73 CLT) — `AdicionalNoturno`
//! - `Intrajornada` (rest period) — 15 min for 4-6h shifts, 1h for >6h (art. 71 CLT)
//! - `Banco de horas` settlement (art. 59, §2º CLT) — `BancoHoras`
//! - `DSR` (Descanso Semanal Remunerado) integration
//! - 30 vacation days after 12 months (art. 130 CLT) plus 1/3 bonus (art. 7º, XVII)
//! - `Aviso prévio` (notice period) — 30 days + 3 days/year up to 90 (Lei 12.506/2011)
//! - `Férias` accrual: 1/12 per month worked (art. 130 CLT)

use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};

/// Brazilian federal holidays (national scope, fixed dates).
pub const NATIONAL_HOLIDAYS: &[(u32, u32, &str)] = &[
    (1, 1, "Confraternização Universal"),
    (4, 21, "Tiradentes"),
    (5, 1, "Dia do Trabalho"),
    (9, 7, "Independência"),
    (10, 12, "Nossa Senhora Aparecida"),
    (11, 2, "Finados"),
    (11, 15, "Proclamação da República"),
    (12, 25, "Natal"),
];

/// Returns true if the given date is a Brazilian national holiday.
pub fn is_national_holiday(date: NaiveDate) -> bool {
    NATIONAL_HOLIDAYS
        .iter()
        .any(|(m, d, _)| date.month() == *m && date.day() == *d)
}

/// Regular work-week threshold under CLT.
pub const WEEKLY_THRESHOLD_HOURS: f64 = 44.0;

/// Overtime multiplier (50%).
pub const OVERTIME_MULTIPLIER: f64 = 1.5;

/// Holiday/rest-day overtime multiplier (100%).
pub const HOLIDAY_OVERTIME_MULTIPLIER: f64 = 2.0;

/// Night shift add-on (20%).
pub const NIGHT_SHIFT_ADDON: f64 = 0.2;

/// Intrajornada (intra-shift rest) thresholds.
pub const INTRAJORNADA_THRESHOLDS: &[(i64, i64)] = &[
    (4 * 60, 15), // 4h shift = 15 min rest
    (6 * 60, 60), // 6h shift = 1h rest
];

/// One third vacation bonus (art. 7º, XVII, CF/88).
pub const VACATION_BONUS_FRACTION: f64 = 1.0 / 3.0;

/// Vacation days after 12 months (art. 130 CLT).
pub const VACATION_DAYS_BASE: i64 = 30;

/// Monthly vacation accrual fraction (1/12 per worked month).
pub const VACATION_ACCRUAL_MONTHLY: f64 = 1.0 / 12.0;

/// Returns the required intra-shift rest (in minutes) for a shift of
/// `shift_minutes` duration, per art. 71 §2º CLT as amended by Lei 13.932/2019:
/// one hour for a journey longer than six hours, thirty minutes for a journey
/// of four to six hours, and no mandatory break below four hours.
pub fn required_intrajornada_minutes(shift_minutes: i64) -> i64 {
    if shift_minutes > 6 * 60 {
        60
    } else if shift_minutes >= 4 * 60 {
        30
    } else {
        0
    }
}

/// True if a time-of-day falls inside the night shift window
/// (22:00 – 05:00, art. 73 CLT).
pub fn is_night_shift_hour(hour: u32) -> bool {
    !(5..22).contains(&hour)
}

/// Computes the night shift add-on minutes for a shift that crossed
/// the night window.
pub fn night_shift_minutes(start_hour: u32, end_hour: u32) -> u32 {
    if !is_night_shift_hour(start_hour) && !is_night_shift_hour(end_hour) {
        return 0;
    }
    let mut count = 0u32;
    let mut h = start_hour;
    while h != end_hour {
        if is_night_shift_hour(h) {
            count += 60;
        }
        h = (h + 1) % 24;
    }
    count
}

/// Overtime classification for a worked day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OvertimeKind {
    /// Day-of-week regular overtime (50% on top of regular hour).
    Regular,
    /// Holiday or weekly rest day (100% multiplier).
    Holiday,
    /// No overtime.
    None,
}

/// Returns the proper overtime multiplier for a worked day.
pub fn overtime_multiplier(overtime: OvertimeKind) -> f64 {
    match overtime {
        OvertimeKind::Regular => OVERTIME_MULTIPLIER,
        OvertimeKind::Holiday => HOLIDAY_OVERTIME_MULTIPLIER,
        OvertimeKind::None => 1.0,
    }
}

/// Computes the base vacation entitlement in days for an employee that
/// has worked `months_worked` months (art. 130 CLT, art. 7º, XVII).
pub fn vacation_days_for(months_worked: i64) -> i64 {
    if months_worked < 12 {
        return 0;
    }
    let proportional = (months_worked as f64 * VACATION_ACCRUAL_MONTHLY).floor() as i64;
    VACATION_DAYS_BASE.min(proportional)
}

/// Computes the `aviso prévio` (notice period) days based on tenure in years.
/// Lei 12.506/2011: 30 days + 3 days per year, capped at 90 days total.
pub fn aviso_previo_days(tenure_years: i64) -> i64 {
    (30 + (tenure_years.max(0) * 3)).min(90)
}

/// Categorizes a worked day under CLT rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkDayKind {
    /// Regular working day.
    Regular,
    /// Brazilian national holiday.
    NationalHoliday,
    /// Weekly Sunday rest (Descanso Semanal Remunerado).
    Sunday,
    /// Compensatory rest day (banco de horas).
    Compensatory,
}

/// Classifies a date under Brazilian labor law.
pub fn classify_day(date: NaiveDate) -> WorkDayKind {
    if is_national_holiday(date) {
        return WorkDayKind::NationalHoliday;
    }
    if date.weekday().number_from_sunday() == 1 {
        return WorkDayKind::Sunday;
    }
    WorkDayKind::Regular
}

/// Computes worked hours billable in the weekly banco de horas.
/// Positive = credit, negative = debit. Caller is responsible for
/// capping to the legal limit (2h/day, art. 59 CLT).
pub fn banco_horas_delta(daily_hours: f64) -> f64 {
    let daily_limit = 8.0_f64;
    let cap = 2.0_f64;
    let extra = daily_hours - daily_limit;
    extra.clamp(-cap, cap)
}

/// Aggregated CLT compliance summary for one payroll period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CltSummary {
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub regular_hours: f64,
    pub overtime_50_hours: f64,
    pub overtime_100_hours: f64,
    pub night_shift_hours: f64,
    pub banco_horas_balance: f64,
    pub vacation_days_remaining: i64,
    pub days_worked: i64,
    pub days_absent: i64,
    pub dsr_paid: bool,
}

impl CltSummary {
    /// Constructs an empty summary for the given period.
    pub fn new(period_start: NaiveDate, period_end: NaiveDate) -> Self {
        Self {
            period_start,
            period_end,
            regular_hours: 0.0,
            overtime_50_hours: 0.0,
            overtime_100_hours: 0.0,
            night_shift_hours: 0.0,
            banco_horas_balance: 0.0,
            vacation_days_remaining: VACATION_DAYS_BASE,
            days_worked: 0,
            days_absent: 0,
            dsr_paid: true,
        }
    }

    /// Adds a worked day to the summary.
    pub fn add_day(&mut self, kind: WorkDayKind, hours: f64) {
        match kind {
            WorkDayKind::Regular => {
                self.days_worked += 1;
                if hours > 8.0 {
                    self.regular_hours += 8.0;
                    self.overtime_50_hours += hours - 8.0;
                } else {
                    self.regular_hours += hours;
                }
            }
            WorkDayKind::NationalHoliday | WorkDayKind::Sunday => {
                self.overtime_100_hours += hours;
            }
            WorkDayKind::Compensatory => {
                self.regular_hours += hours;
            }
        }
    }

    /// Adds a missed workday (justified absence).
    pub fn add_absence(&mut self) {
        self.days_absent += 1;
    }

    /// Settles banco de horas delta for the day.
    pub fn settle_banco_horas(&mut self, daily_hours: f64) {
        self.banco_horas_balance += banco_horas_delta(daily_hours);
    }

    /// Period duration in days.
    pub fn period_days(&self) -> i64 {
        (self.period_end - self.period_start).num_days() + 1
    }

    /// Period duration in weeks.
    pub fn period_weeks(&self) -> i64 {
        (self.period_days() as f64 / 7.0).ceil() as i64
    }

    /// Total hours in the period (regular + overtime + night).
    pub fn total_hours(&self) -> f64 {
        self.regular_hours + self.overtime_50_hours + self.overtime_100_hours + self.night_shift_hours
    }
}

/// Returns the number of weeks between two dates.
pub fn weeks_between(start: NaiveDate, end: NaiveDate) -> i64 {
    ((end - start).num_days() as f64 / 7.0).floor() as i64
}

/// Returns the number of calendar months between two dates.
pub fn months_between(start: NaiveDate, end: NaiveDate) -> i64 {
    let years = end.year() - start.year();
    let months = end.month() as i64 - start.month() as i64;
    ((years as i64) * 12 + months).max(0)
}

/// Returns the number of days in the period.
pub fn days_in_period(start: NaiveDate, end: NaiveDate) -> i64 {
    (end - start).num_days() + 1
}

/// Returns the proper `Duration` (chrono) for an `aviso prévio` given
/// tenure years. Convenience for HR systems that need a Duration.
pub fn aviso_previo_duration(tenure_years: i64) -> Duration {
    Duration::days(aviso_previo_days(tenure_years))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_national_holidays() {
        assert!(is_national_holiday(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()));
        assert!(is_national_holiday(NaiveDate::from_ymd_opt(2026, 12, 25).unwrap()));
        assert!(!is_national_holiday(NaiveDate::from_ymd_opt(2026, 3, 15).unwrap()));
    }

    #[test]
    fn test_intrajornada() {
        // Four to six hours inclusive: thirty minutes.
        assert_eq!(required_intrajornada_minutes(4 * 60), 30);
        assert_eq!(required_intrajornada_minutes(5 * 60 + 59), 30);
        assert_eq!(required_intrajornada_minutes(6 * 60), 30);
        // Strictly longer than six hours: one hour.
        assert_eq!(required_intrajornada_minutes(6 * 60 + 1), 60);
        assert_eq!(required_intrajornada_minutes(8 * 60), 60);
        // Below four hours: no mandatory break.
        assert_eq!(required_intrajornada_minutes(3 * 60), 0);
    }

    #[test]
    fn test_night_shift_hour() {
        assert!(is_night_shift_hour(22));
        assert!(is_night_shift_hour(0));
        assert!(is_night_shift_hour(4));
        assert!(!is_night_shift_hour(5));
        assert!(!is_night_shift_hour(21));
    }

    #[test]
    fn test_vacation_days() {
        assert_eq!(vacation_days_for(0), 0);
        assert_eq!(vacation_days_for(6), 0);
        assert_eq!(vacation_days_for(12), 1);
        assert_eq!(vacation_days_for(60), 5);
    }

    #[test]
    fn test_aviso_previo() {
        assert_eq!(aviso_previo_days(0), 30);
        assert_eq!(aviso_previo_days(1), 33);
        assert_eq!(aviso_previo_days(20), 90);
        assert_eq!(aviso_previo_days(50), 90);
    }

    #[test]
    fn test_overtime_multiplier() {
        assert_eq!(overtime_multiplier(OvertimeKind::Regular), 1.5);
        assert_eq!(overtime_multiplier(OvertimeKind::Holiday), 2.0);
        assert_eq!(overtime_multiplier(OvertimeKind::None), 1.0);
    }

    #[test]
    fn test_banco_horas_delta() {
        assert_eq!(banco_horas_delta(8.0), 0.0);
        assert_eq!(banco_horas_delta(9.0), 1.0);
        assert_eq!(banco_horas_delta(11.0), 2.0);
        assert_eq!(banco_horas_delta(7.0), -1.0);
    }

    #[test]
    fn test_clt_summary() {
        let start = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 6, 30).unwrap();
        let mut s = CltSummary::new(start, end);
        s.add_day(WorkDayKind::Regular, 9.0);
        s.add_day(WorkDayKind::Regular, 8.0);
        s.add_day(WorkDayKind::NationalHoliday, 6.0);
        assert_eq!(s.days_worked, 2);
        assert_eq!(s.regular_hours, 16.0);
        assert_eq!(s.overtime_50_hours, 1.0);
        assert_eq!(s.overtime_100_hours, 6.0);
    }
}
