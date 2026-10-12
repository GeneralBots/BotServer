//! Self-contained five-field cron parser and next-occurrence calculator.
//!
//! Supported syntax per field: `*`, lists (`,`), ranges (`-`) and steps
//! (`/`). Fields: minute hour day-of-month month day-of-week, with the Vixie
//! rule that a restricted day-of-month combined with a restricted
//! day-of-week matches when either one matches.

use chrono::{Datelike, Duration, TimeZone, Timelike, Utc};

const MAX_ITERATIONS: usize = 527040;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronExpr {
    expr: String,
    minutes: Vec<u32>,
    hours: Vec<u32>,
    days_of_month: Vec<u32>,
    months: Vec<u32>,
    days_of_week: Vec<u32>,
}

fn parse_field(field: &str, min: u32, max: u32) -> Option<Vec<u32>> {
    let mut values = Vec::new();
    for part in field.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return None;
        }
        let (range_part, step) = match part.split_once('/') {
            Some((r, s)) => {
                let step = s.parse::<u32>().ok()?;
                if step == 0 {
                    return None;
                }
                (r, step)
            }
            None => (part, 1),
        };
        let (lo, hi) = if range_part == "*" {
            (min, max)
        } else if let Some((a, b)) = range_part.split_once('-') {
            let lo = a.trim().parse::<u32>().ok()?;
            let hi = b.trim().parse::<u32>().ok()?;
            (lo, hi)
        } else {
            let v = range_part.parse::<u32>().ok()?;
            if step > 1 {
                (v, max)
            } else {
                (v, v)
            }
        };
        if lo < min || hi > max || lo > hi {
            return None;
        }
        let mut v = lo;
        while v <= hi {
            values.push(v);
            v += step;
        }
    }
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    values.dedup();
    Some(values)
}

/// Maps day-of-week `7` to `0` (Sunday) so both notations are accepted.
///
/// The remap has to happen before the deduplication that `parse_field` performs:
/// folding `7` onto an existing `0` afterwards would leave the vector with a
/// duplicate, and `dow_restricted` compares it against `0..=6`. The mismatch
/// made an unrestricted day-of-week look restricted, which switched `day_matches`
/// into the day-of-month-OR-day-of-week branch and matched every Sunday.
fn normalize_dow(mut values: Vec<u32>) -> Vec<u32> {
    for value in values.iter_mut() {
        if *value == 7 {
            *value = 0;
        }
    }
    // Remapping 7 onto Sunday can collide with an explicit 0, so the duplicate
    // has to be removed after the remap, not before it.
    values.sort_unstable();
    values.dedup();
    values
}

impl CronExpr {
    pub fn parse(expr: &str) -> Result<Self, String> {
        let fields: Vec<&str> = expr.split_whitespace().collect();
        if fields.len() != 5 {
            return Err(format!("expected 5 cron fields, got {}", fields.len()));
        }
        let minutes =
            parse_field(fields[0], 0, 59).ok_or_else(|| "invalid minute field".to_string())?;
        let hours =
            parse_field(fields[1], 0, 23).ok_or_else(|| "invalid hour field".to_string())?;
        let days_of_month =
            parse_field(fields[2], 1, 31).ok_or_else(|| "invalid day-of-month field".to_string())?;
        let months =
            parse_field(fields[3], 1, 12).ok_or_else(|| "invalid month field".to_string())?;
        let days_of_week = parse_field(fields[4], 0, 7)
            .map(normalize_dow)
            .ok_or_else(|| "invalid day-of-week field".to_string())?;
        Ok(Self {
            expr: expr.to_string(),
            minutes,
            hours,
            days_of_month,
            months,
            days_of_week,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.expr
    }

    fn dom_restricted(&self) -> bool {
        self.days_of_month != (1..=31).collect::<Vec<_>>()
    }

    fn dow_restricted(&self) -> bool {
        self.days_of_week != (0..=6).collect::<Vec<_>>()
    }

    fn day_matches(&self, t: chrono::DateTime<Utc>) -> bool {
        let dom_ok = self.days_of_month.contains(&t.day());
        let dow_ok = self.days_of_week.contains(&t.weekday().num_days_from_sunday());
        match (self.dom_restricted(), self.dow_restricted()) {
            (true, true) => dom_ok || dow_ok,
            (true, false) => dom_ok,
            (false, true) => dow_ok,
            (false, false) => true,
        }
    }

    /// Earliest occurrence strictly after `from`, or `None` when none exists
    /// within one leap year of minute iterations.
    pub fn next_after(&self, from: chrono::DateTime<Utc>) -> Option<chrono::DateTime<Utc>> {
        let mut t = floor_minute(from + Duration::minutes(1));
        for _ in 0..MAX_ITERATIONS {
            if !self.months.contains(&t.month()) {
                t = first_of_next_month(t)?;
                continue;
            }
            if !self.day_matches(t) {
                t = midnight_next_day(t)?;
                continue;
            }
            if !self.hours.contains(&t.hour()) {
                t = floor_minute(t + Duration::hours(1));
                continue;
            }
            if !self.minutes.contains(&t.minute()) {
                t += Duration::minutes(1);
                continue;
            }
            return Some(t);
        }
        None
    }
}

fn floor_minute(t: chrono::DateTime<Utc>) -> chrono::DateTime<Utc> {
    t.with_second(0)
        .and_then(|d| d.with_nanosecond(0))
        .unwrap_or(t)
}

fn midnight_next_day(t: chrono::DateTime<Utc>) -> Option<chrono::DateTime<Utc>> {
    let date = (t + Duration::days(1)).date_naive();
    let midnight = date.and_hms_opt(0, 0, 0)?;
    Some(Utc.from_utc_datetime(&midnight))
}

fn first_of_next_month(t: chrono::DateTime<Utc>) -> Option<chrono::DateTime<Utc>> {
    let (year, month) = if t.month() == 12 {
        (t.year() + 1, 1)
    } else {
        (t.year(), t.month() + 1)
    };
    Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0).single()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, hour, min, sec).single().expect("valid test instant")
    }

    #[test]
    fn every_minute_advances_one_step() {
        let c = CronExpr::parse("* * * * *").expect("parse");
        assert_eq!(c.next_after(at(2026, 3, 10, 10, 30, 15)), Some(at(2026, 3, 10, 10, 31, 0)));
    }

    #[test]
    fn daily_expression_rolls_to_next_day() {
        let c = CronExpr::parse("30 2 * * *").expect("parse");
        assert_eq!(c.next_after(at(2026, 3, 10, 12, 0, 0)), Some(at(2026, 3, 11, 2, 30, 0)));
        assert_eq!(c.next_after(at(2026, 3, 10, 1, 0, 0)), Some(at(2026, 3, 10, 2, 30, 0)));
    }

    #[test]
    fn stepped_minutes_match_boundaries_only() {
        let c = CronExpr::parse("*/15 * * * *").expect("parse");
        assert_eq!(c.next_after(at(2026, 1, 5, 8, 16, 0)), Some(at(2026, 1, 5, 8, 30, 0)));
        assert_eq!(c.next_after(at(2026, 1, 5, 8, 59, 59)), Some(at(2026, 1, 5, 9, 0, 0)));
    }

    #[test]
    fn monthly_first_day_crosses_months() {
        let c = CronExpr::parse("0 0 1 * *").expect("parse");
        assert_eq!(c.next_after(at(2026, 1, 2, 0, 0, 0)), Some(at(2026, 2, 1, 0, 0, 0)));
    }

    #[test]
    fn yearly_expression_skips_fast_across_months() {
        let c = CronExpr::parse("0 0 1 1 *").expect("parse");
        assert_eq!(c.next_after(at(2026, 2, 14, 23, 59, 0)), Some(at(2027, 1, 1, 0, 0, 0)));
    }

    #[test]
    fn weekday_only_matches_mondays_for_dow_one() {
        let c = CronExpr::parse("0 9 * * 1").expect("parse");
        // 2026-08-24 is a Monday; after 09:00 the next hit is 2026-08-31.
        assert_eq!(c.next_after(at(2026, 8, 24, 10, 0, 0)), Some(at(2026, 8, 31, 9, 0, 0)));
    }

    #[test]
    fn restricted_dom_and_dow_use_vixie_or_rule() {
        // 13th of any month OR any Friday (dow=5): Vixie OR semantics, so a
        // matching Friday wins even when the day-of-month does not match.
        let c = CronExpr::parse("0 0 13 * 5").expect("parse");
        // 2026-03-13 is a Friday; the following Friday, 2026-03-20, matches on
        // the day-of-week rule alone.
        assert_eq!(c.next_after(at(2026, 3, 14, 0, 0, 0)), Some(at(2026, 3, 20, 0, 0, 0)));
        // 2026-04-13 is a Monday: it matches only through the day-of-month rule,
        // which the OR semantics keeps in play after the Friday run.
        assert_eq!(c.next_after(at(2026, 4, 11, 0, 0, 0)), Some(at(2026, 4, 13, 0, 0, 0)));
    }

    #[test]
    fn list_fields_are_supported() {
        let c = CronExpr::parse("5,35 8-18 * * *").expect("parse");
        assert_eq!(c.next_after(at(2026, 6, 1, 7, 0, 0)), Some(at(2026, 6, 1, 8, 5, 0)));
        // Both minutes of the list are honoured: after 08:05 the next hit is
        // still within hour 8, and only once that list is exhausted does the
        // search move to the next hour.
        assert_eq!(c.next_after(at(2026, 6, 1, 8, 6, 0)), Some(at(2026, 6, 1, 8, 35, 0)));
        // After 08:35 both minutes of hour 8 have passed, so the next hit is the
        // first list entry of the next hour.
        assert_eq!(c.next_after(at(2026, 6, 1, 8, 36, 0)), Some(at(2026, 6, 1, 9, 5, 0)));
    }

    #[test]
    fn invalid_expressions_are_rejected() {
        assert!(CronExpr::parse("* * * *").is_err());
        assert!(CronExpr::parse("60 * * * *").is_err());
        assert!(CronExpr::parse("*/0 * * * *").is_err());
        assert!(CronExpr::parse("a * * * *").is_err());
    }
}
