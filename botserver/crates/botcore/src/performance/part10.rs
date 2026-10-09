use super::*;

pub fn create_cursor(id: &Uuid, timestamp: &DateTime<Utc>) -> String {
    let data = format!("{}:{}", id, timestamp.timestamp_millis());
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, data.as_bytes())
}
pub fn parse_cursor(cursor: &str) -> Option<(Uuid, DateTime<Utc>)> {
    let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, cursor).ok()?;
    let data = String::from_utf8(decoded).ok()?;
    let parts: Vec<&str> = data.split(':').collect();

    if parts.len() != 2 {
        return None;
    }

    let id = Uuid::parse_str(parts[0]).ok()?;
    let timestamp_ms: i64 = parts[1].parse().ok()?;
    let timestamp = DateTime::from_timestamp_millis(timestamp_ms)?;

    Some((id, timestamp))
}
