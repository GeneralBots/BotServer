use super::*;

pub fn normalize_etag(etag: &str) -> String {
    etag.trim_matches('"').to_string()
}
