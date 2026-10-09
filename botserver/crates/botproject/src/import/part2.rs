use super::*;

pub(super) fn parse_ms_date(s: &str) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d"))
        .ok()
}
pub(super) fn parse_ms_duration(duration: &Option<String>) -> Option<u32> {
    duration.as_ref().and_then(|d| {
        if d.starts_with("PT") {
            let hours_str = d.trim_start_matches("PT").trim_end_matches('H');
            hours_str.parse::<f64>().ok().map(|h| (h / 8.0).ceil() as u32)
        } else {
            Some(1)
        }
    })
}
pub(super) fn parse_ms_work(work: &Option<String>) -> Option<f32> {
    work.as_ref().and_then(|w| {
        if w.starts_with("PT") {
            let hours_str = w.trim_start_matches("PT").trim_end_matches('H');
            hours_str.parse::<f32>().ok()
        } else {
            None
        }
    })
}
