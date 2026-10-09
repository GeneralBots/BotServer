use super::*;

pub(super) fn is_placeholder_value(val: &str) -> bool {
    let lower = val.trim().to_lowercase();
    lower.is_empty() || lower == "none" || lower == "null" || lower == "n/a"
}
