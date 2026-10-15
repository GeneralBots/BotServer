//! XML helpers for the CalDAV surface (issue #1335).
//!
//! CalDAV is a WebDAV dialect: every response is a `D:multistatus` document and
//! the requests that carry data are `PROPFIND` and `REPORT`. The builders here
//! produce those documents, and the parsers read the small part of a request
//! body the server acts on — the bounds of a `REPORT` time range and the
//! resource hrefs of a `calendar-multiget`.
//!
//! Parsing is deliberately tolerant: a client that sends a slightly different
//! document must still be able to sync, and a malformed body yields an empty
//! result rather than an error, so the caller falls back to its own bounds.

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use uuid::Uuid;

/// Escapes a value for an XML text node or attribute value.
#[must_use]
pub fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Wraps response elements in a `D:multistatus` document.
#[must_use]
pub fn multistatus(responses: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
         <D:multistatus xmlns:D=\"DAV:\" xmlns:C=\"urn:ietf:params:xml:ns:caldav\">\n\
         {responses}\n\
         </D:multistatus>"
    )
}

/// A `D:response` reporting every requested property as found.
#[must_use]
pub fn response(href: &str, props: &str) -> String {
    format!(
        "<D:response>\n\
         <D:href>{href}</D:href>\n\
         <D:propstat>\n\
         <D:prop>\n{props}\n</D:prop>\n\
         <D:status>HTTP/1.1 200 OK</D:status>\n\
         </D:propstat>\n\
         </D:response>",
        href = xml_escape(href)
    )
}

/// A `D:response` reporting a resource that was not found.
#[must_use]
pub fn missing_response(href: &str) -> String {
    format!(
        "<D:response>\n\
         <D:href>{href}</D:href>\n\
         <D:status>HTTP/1.1 404 Not Found</D:status>\n\
         </D:response>",
        href = xml_escape(href)
    )
}

/// Reads the `start` and `end` bounds of the first `C:time-range` element.
///
/// Both are optional: a client may ask for everything before an end, or
/// everything after a start.
#[must_use]
pub fn parse_time_range(body: &str) -> (Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
    let Some(element) = find_element(body, "time-range") else {
        return (None, None);
    };
    (
        find_attribute(&element, "start").and_then(|value| parse_timestamp(&value)),
        find_attribute(&element, "end").and_then(|value| parse_timestamp(&value)),
    )
}

/// Reads the `D:href` values of a `calendar-multiget` request.
#[must_use]
pub fn parse_hrefs(body: &str) -> Vec<String> {
    let mut hrefs = Vec::new();
    let mut search_from = 0;
    while let Some(offset) = body[search_from..].find("href>") {
        let value_start = search_from + offset + "href>".len();
        let Some(end_offset) = body[value_start..].find("</") else {
            break;
        };
        let value = body[value_start..value_start + end_offset].trim();
        if !value.is_empty() {
            hrefs.push(value.to_string());
        }
        search_from = value_start + end_offset;
    }
    hrefs
}

/// Extracts the event identifier from a CalDAV href such as
/// `/caldav/calendars/<calendar>/<event>.ics`.
#[must_use]
pub fn event_id_from_href(href: &str) -> Option<Uuid> {
    let trimmed = href.trim_end_matches('/');
    let last = trimmed.rsplit('/').next()?;
    let without_extension = last.strip_suffix(".ics").unwrap_or(last);
    Uuid::parse_str(without_extension).ok()
}

/// Returns the attributes of the first element whose qualified name ends with
/// `name`, for example `C:time-range` for `name = "time-range"`.
fn find_element(body: &str, name: &str) -> Option<String> {
    let mut search_from = 0;
    while let Some(offset) = body[search_from..].find('<') {
        let tag_start = search_from + offset + 1;
        let tag_end_offset = body[tag_start..].find('>')?;
        let tag_end = tag_start + tag_end_offset;
        let tag = &body[tag_start..tag_end];
        let qualified = tag.split_whitespace().next().unwrap_or_default();
        if qualified.rsplit(':').next() == Some(name) {
            return Some(tag.to_string());
        }
        search_from = tag_end + 1;
    }
    None
}

/// Returns the value of an attribute inside an element's attribute list.
fn find_attribute(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=");
    let value_start = tag.find(&needle)? + needle.len();
    let rest = &tag[value_start..];
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        // Unquoted attribute value: it ends at the next space.
        let value: String = rest.chars().take_while(|c| !c.is_whitespace()).collect();
        return if value.is_empty() { None } else { Some(value) };
    }
    let value = rest[1..].split_once(quote)?.0;
    Some(value.to_string())
}

/// Parses an iCalendar timestamp (`YYYYMMDDTHHMMSSZ`). The minute form and the
/// date-only form are accepted because clients send all three.
fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    let value = value.trim().trim_end_matches('Z').trim_end_matches('z');
    for format in ["%Y%m%dT%H%M%S", "%Y%m%dT%H%M"] {
        if let Ok(parsed) = NaiveDateTime::parse_from_str(value, format) {
            return Some(Utc.from_utc_datetime(&parsed));
        }
    }
    let date = NaiveDate::parse_from_str(value, "%Y%m%d").ok()?;
    let midnight = date.and_hms_opt(0, 0, 0)?;
    Some(Utc.from_utc_datetime(&midnight))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_time_range_bounds() {
        let body = r#"<C:calendar-query><C:filter><C:comp-filter name="VCALENDAR">
            <C:comp-filter name="VEVENT"><C:time-range start="20260901T000000Z" end="20261001T000000Z"/>
            </C:comp-filter></C:comp-filter></C:filter></C:calendar-query>"#;
        let (start, end) = parse_time_range(body);
        assert_eq!(start.map(|value| value.to_rfc3339()), Some("2026-09-01T00:00:00+00:00".to_string()));
        assert_eq!(end.map(|value| value.to_rfc3339()), Some("2026-10-01T00:00:00+00:00".to_string()));
    }

    #[test]
    fn parses_multiget_hrefs() {
        let body = r#"<C:calendar-multiget><D:href>/caldav/calendars/7c9e6679-7425-40de-944b-e07fc1f90ae7/3f2504e0-4f89-11d3-9a0c-0305e82c3301.ics</D:href></C:calendar-multiget>"#;
        let hrefs = parse_hrefs(body);
        assert_eq!(hrefs.len(), 1);
        assert_eq!(
            event_id_from_href(&hrefs[0]).map(|id| id.to_string()),
            Some("3f2504e0-4f89-11d3-9a0c-0305e82c3301".to_string())
        );
    }

    #[test]
    fn escapes_markup_in_values() {
        assert_eq!(xml_escape("a & b <c>"), "a &amp; b &lt;c&gt;");
    }
}
