use std::sync::LazyLock;

static SANITIZATION_PATTERNS: LazyLock<Vec<(&'static str, &'static str)>> = LazyLock::new(|| {
    vec![
        ("\n", "\\n"),
        ("\r", "\\r"),
        ("\t", "\\t"),
        ("\\", "\\\\"),
        ("\"", "\\\""),
        ("'", "\\'"),
        ("\x00", "\\x00"),
        ("\x1B", "\\x1B"),
    ]
});

pub fn sanitize_for_log(input: &str) -> String {
    let mut result = input.to_string();

    for (pattern, replacement) in SANITIZATION_PATTERNS.iter() {
        result = result.replace(pattern, replacement);
    }

    if result.len() > 10000 {
        // Log messages carry arbitrary text; `String::truncate` panics when the
        // byte budget lands inside a multi-byte character.
        let end = botlib::text::char_boundary_at_or_before(&result, 10000);
        result.truncate(end);
        result.push_str("... [truncated]");
    }

    result
}

pub fn sanitize_log_value<T: std::fmt::Display>(value: T) -> String {
    sanitize_for_log(&value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 10 000-byte budget is a byte count over arbitrary text, so it can
    /// land inside a character: `String::truncate` then panics and takes the
    /// process down. 12 000 bytes of '€' puts byte 10 000 mid-character.
    #[test]
    fn truncation_lands_on_a_char_boundary() {
        let long = "€".repeat(4000);
        assert_eq!(long.len(), 12_000);

        let sanitized = sanitize_for_log(&long);

        assert!(sanitized.ends_with("... [truncated]"));
        // 9 999 bytes kept: the largest char boundary at or before the budget.
        let kept = sanitized.trim_end_matches("... [truncated]");
        assert_eq!(kept.len(), 9_999);
        assert!(kept.chars().all(|c| c == '€'));
    }

    #[test]
    fn short_input_is_returned_untouched() {
        assert_eq!(sanitize_for_log("ação"), "ação");
    }
}
