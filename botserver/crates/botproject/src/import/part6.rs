

pub(super) fn parse_date_flexible(s: &str) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .or_else(|_| chrono::NaiveDate::parse_from_str(s, "%m/%d/%Y"))
        .or_else(|_| chrono::NaiveDate::parse_from_str(s, "%d/%m/%Y"))
        .ok()
}
