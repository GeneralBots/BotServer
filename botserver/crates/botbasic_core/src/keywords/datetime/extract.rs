use botbasic_types::UserSession;
use botbasic_types::BasicRuntime;
use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike};
use log::debug;
use rhai::Engine;
use std::sync::Arc;

/// The date formats accepted on their own.
fn parse_date_only(trimmed: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
        .ok()
        .or_else(|| NaiveDate::parse_from_str(trimmed, "%d/%m/%Y").ok())
        .or_else(|| NaiveDate::parse_from_str(trimmed, "%m/%d/%Y").ok())
        .or_else(|| NaiveDate::parse_from_str(trimmed, "%Y/%m/%d").ok())
}

/// The timestamp formats accepted on their own.
fn parse_datetime_only(trimmed: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S")
        .ok()
        .or_else(|| NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S").ok())
        .or_else(|| NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M").ok())
}

/// A bare date, or the date part of a timestamp.
///
/// The two parsers deliberately do not call each other: they previously fell
/// back on one another, so any string that matched neither a date nor a
/// timestamp (a typo, a sentence, an empty keyword argument) recursed until the
/// stack overflowed and aborted the process.
fn parse_date(date_str: &str) -> Option<NaiveDate> {
    let trimmed = date_str.trim();
    parse_date_only(trimmed).or_else(|| parse_datetime_only(trimmed).map(|dt| dt.date()))
}

/// A timestamp, or a bare date read as midnight.
fn parse_datetime(datetime_str: &str) -> Option<NaiveDateTime> {
    let trimmed = datetime_str.trim();
    parse_datetime_only(trimmed)
        .or_else(|| parse_date_only(trimmed).and_then(|d| d.and_hms_opt(0, 0, 0)))
}

pub fn year_keyword(_state: &Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    engine.register_fn("YEAR", |date_str: &str| -> i64 {
        parse_date(date_str)
            .map(|d| i64::from(d.year()))
            .unwrap_or(0)
    });
    engine.register_fn("year", |date_str: &str| -> i64 {
        parse_date(date_str)
            .map(|d| i64::from(d.year()))
            .unwrap_or(0)
    });

    debug!("Registered YEAR keyword");
}

pub fn month_keyword(_state: &Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    engine.register_fn("MONTH", |date_str: &str| -> i64 {
        parse_date(date_str)
            .map(|d| i64::from(d.month()))
            .unwrap_or(0)
    });
    engine.register_fn("month", |date_str: &str| -> i64 {
        parse_date(date_str)
            .map(|d| i64::from(d.month()))
            .unwrap_or(0)
    });

    debug!("Registered MONTH keyword");
}

pub fn day_keyword(_state: &Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    engine.register_fn("DAY", |date_str: &str| -> i64 {
        parse_date(date_str)
            .map(|d| i64::from(d.day()))
            .unwrap_or(0)
    });
    engine.register_fn("day", |date_str: &str| -> i64 {
        parse_date(date_str)
            .map(|d| i64::from(d.day()))
            .unwrap_or(0)
    });

    debug!("Registered DAY keyword");
}

pub fn hour_keyword(_state: &Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    engine.register_fn("HOUR", |datetime_str: &str| -> i64 {
        parse_datetime(datetime_str)
            .map(|d| i64::from(d.hour()))
            .unwrap_or(0)
    });
    engine.register_fn("hour", |datetime_str: &str| -> i64 {
        parse_datetime(datetime_str)
            .map(|d| i64::from(d.hour()))
            .unwrap_or(0)
    });

    debug!("Registered HOUR keyword");
}

pub fn minute_keyword(_state: &Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    engine.register_fn("MINUTE", |datetime_str: &str| -> i64 {
        parse_datetime(datetime_str)
            .map(|d| i64::from(d.minute()))
            .unwrap_or(0)
    });
    engine.register_fn("minute", |datetime_str: &str| -> i64 {
        parse_datetime(datetime_str)
            .map(|d| i64::from(d.minute()))
            .unwrap_or(0)
    });

    debug!("Registered MINUTE keyword");
}

pub fn second_keyword(_state: &Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    engine.register_fn("SECOND", |datetime_str: &str| -> i64 {
        parse_datetime(datetime_str)
            .map(|d| i64::from(d.second()))
            .unwrap_or(0)
    });
    engine.register_fn("second", |datetime_str: &str| -> i64 {
        parse_datetime(datetime_str)
            .map(|d| i64::from(d.second()))
            .unwrap_or(0)
    });

    debug!("Registered SECOND keyword");
}

pub fn weekday_keyword(_state: &Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    engine.register_fn("WEEKDAY", |date_str: &str| -> i64 {
        parse_date(date_str)
            .map(|d| i64::from(d.weekday().num_days_from_sunday()) + 1)
            .unwrap_or(0)
    });
    engine.register_fn("weekday", |date_str: &str| -> i64 {
        parse_date(date_str)
            .map(|d| i64::from(d.weekday().num_days_from_sunday()) + 1)
            .unwrap_or(0)
    });

    debug!("Registered WEEKDAY keyword");
}

pub fn format_date_keyword(_state: &Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    engine.register_fn("FORMAT_DATE", |date_str: &str, format: &str| -> String {
        format_date_impl(date_str, format)
    });
    engine.register_fn("format_date", |date_str: &str, format: &str| -> String {
        format_date_impl(date_str, format)
    });

    debug!("Registered FORMAT_DATE keyword");
}

pub fn isdate_keyword(_state: &Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    engine.register_fn("ISDATE", |value: &str| -> bool {
        parse_date(value).is_some()
    });
    engine.register_fn("isdate", |value: &str| -> bool {
        parse_date(value).is_some()
    });
    engine.register_fn("IS_DATE", |value: &str| -> bool {
        parse_date(value).is_some()
    });

    debug!("Registered ISDATE keyword");
}

pub fn format_date_impl(date_str: &str, format: &str) -> String {
    if let Some(datetime) = parse_datetime(date_str) {
        let chrono_format = format
            .replace("YYYY", "%Y")
            .replace("yyyy", "%Y")
            .replace("YY", "%y")
            .replace("yy", "%y")
            .replace("MMMM", "%B")
            .replace("MMM", "%b")
            .replace("MM", "%m")
            .replace("DD", "%d")
            .replace("dd", "%d")
            .replace("HH", "%H")
            .replace("hh", "%I")
            .replace("mm", "%M")
            .replace("ss", "%S")
            .replace("AM/PM", "%p")
            .replace("am/pm", "%P");

        datetime.format(&chrono_format).to_string()
    } else {
        date_str.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_parse_date() {
        let date = parse_date("2025-01-22");
        assert!(date.is_some());
        let d = date.unwrap();
        assert_eq!(d.year(), 2025);
        assert_eq!(d.month(), 1);
        assert_eq!(d.day(), 22);
    }

    #[test]
    fn test_parse_datetime() {
        let dt = parse_datetime("2025-01-22 14:30:45");
        assert!(dt.is_some());
        let d = dt.unwrap();
        assert_eq!(d.hour(), 14);
        assert_eq!(d.minute(), 30);
        assert_eq!(d.second(), 45);
    }

    #[test]
    fn test_invalid_date() {
        // Inputs that match neither a date nor a timestamp must return `None`
        // instead of recursing between the two parsers.
        for input in ["invalid", "", "   ", "not a date at all", "2026-13-45"] {
            assert!(parse_date(input).is_none(), "{input} should not parse");
            assert!(parse_datetime(input).is_none(), "{input} should not parse");
        }
    }

    #[test]
    fn a_date_is_read_as_midnight_and_a_timestamp_keeps_its_time() {
        assert_eq!(
            parse_datetime("2026-06-15"),
            parse_date("2026-06-15").and_then(|d| d.and_hms_opt(0, 0, 0))
        );
        assert_eq!(parse_date("2026-06-15 10:30:00"), parse_date("2026-06-15"));
        assert!(parse_datetime("2026-06-15 10:30").is_some());
    }
}
