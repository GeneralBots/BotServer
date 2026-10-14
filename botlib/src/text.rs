//! Bounded, panic-free text helpers.
//!
//! Slicing a `str` at a byte offset panics when the offset falls inside a
//! multi-byte character, and `botserver` must never abort on user input. Every
//! helper here exists so call sites can stop writing `&s[..n]` / `&s[a..b]`
//! with offsets that come from lengths, budgets or computed positions.

use std::ops::Range;

/// The longest prefix of `s` that is at most `max_bytes` long and ends on a
/// char boundary.
///
/// Returns an empty slice when `max_bytes` is zero, and the whole string when
/// `s` is already short enough. The returned slice always ends on a char
/// boundary, so it is safe to pass to any string API that requires one.
pub fn truncate_at_char_boundary(s: &str, max_bytes: usize) -> &str {
    &s[..char_boundary_at_or_before(s, max_bytes)]
}

/// [`truncate_at_char_boundary`], appending `suffix` when the text was cut.
///
/// Returns the input unchanged when it already fits, so the suffix marks a real
/// truncation rather than appearing on short text.
pub fn truncate_with_suffix(s: &str, max_bytes: usize, suffix: &str) -> String {
    let head = truncate_at_char_boundary(s, max_bytes);
    if head.len() == s.len() {
        s.to_string()
    } else {
        format!("{head}{suffix}")
    }
}

/// The nearest char boundary at or before `index`, with `index` clamped to
/// `s.len()`.
///
/// Returns `0` for an empty string or when `index` is zero. Handy when a byte
/// index came from `.len()`, a budget, or a position computed in a different
/// representation and you need a safe `&s[..n]`.
pub fn char_boundary_at_or_before(s: &str, index: usize) -> usize {
    let mut index = index.min(s.len());
    while index > 0 && !s.is_char_boundary(index) {
        index -= 1;
    }
    index
}

/// `s` with its first character upper-cased.
///
/// Char-aware: `&s[1..]` panics when the first character is multi-byte, which
/// accented names hit immediately.
pub fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// The byte range of the first case-insensitive occurrence of `needle`.
///
/// The comparison folds character by character, and the range is reported in
/// `haystack` offsets. Searching a lower-cased *copy* and reusing its offsets is
/// wrong: folding can change a character's byte length (`İ` becomes two code
/// points, `K` U+212A becomes one), which shifts every offset after it — the
/// source of both wrong snippets and `byte index N is not a char boundary`
/// panics.
pub fn find_ignore_case(haystack: &str, needle: &str) -> Option<Range<usize>> {
    if needle.is_empty() {
        return Some(0..0);
    }

    let wanted: Vec<char> = needle.chars().flat_map(char::to_lowercase).collect();
    for (start, _) in haystack.char_indices() {
        if let Some(end) = folded_prefix_len(&haystack[start..], &wanted) {
            return Some(start..start + end);
        }
    }

    None
}

/// The byte length of the prefix of `rest` whose lower-cased form is exactly
/// `wanted`.
fn folded_prefix_len(rest: &str, wanted: &[char]) -> Option<usize> {
    let mut matched = 0;
    for (offset, ch) in rest.char_indices() {
        for lower in ch.to_lowercase() {
            if matched >= wanted.len() || wanted[matched] != lower {
                return None;
            }
            matched += 1;
        }
        if matched == wanted.len() {
            return Some(offset + ch.len_utf8());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncation_stops_at_a_char_boundary() {
        // "aé" is 3 bytes: 'é' occupies 1..3, so 2 is not a boundary.
        assert_eq!(truncate_at_char_boundary("aé", 3), "aé");
        assert_eq!(truncate_at_char_boundary("aé", 2), "a");
        assert_eq!(truncate_at_char_boundary("aé", 1), "a");
        assert_eq!(truncate_at_char_boundary("aé", 0), "");
        assert_eq!(truncate_at_char_boundary("aé", 99), "aé");
        assert_eq!(truncate_at_char_boundary("", 5), "");
    }

    #[test]
    fn the_suffix_marks_a_real_cut_only() {
        assert_eq!(truncate_with_suffix("hello", 10, "..."), "hello");
        assert_eq!(truncate_with_suffix("hello", 5, "..."), "hello");
        assert_eq!(truncate_with_suffix("hello", 4, "..."), "hell...");
        // "ação" is 6 bytes: a(1) ç(2) ã(2) o(1).
        assert_eq!(truncate_with_suffix("ação", 4, "…"), "aç…");
    }

    #[test]
    fn capitalize_handles_multibyte_and_empty() {
        assert_eq!(capitalize_first("hello"), "Hello");
        assert_eq!(capitalize_first("ção"), "Ção");
        assert_eq!(capitalize_first("École"), "École");
        assert_eq!(capitalize_first(""), "");
    }

    #[test]
    fn ignore_case_search_reports_haystack_offsets() {
        let haystack = "İstanbul error PASSWORD=hunter2";
        let range = find_ignore_case(haystack, "password").expect("match");
        assert_eq!(&haystack[range], "PASSWORD");

        assert_eq!(find_ignore_case("SECRET", "secret"), Some(0..6));
        assert_eq!(find_ignore_case("nothing here", "zzz"), None);
        assert_eq!(find_ignore_case("abc", ""), Some(0..0));
    }

    #[test]
    fn ignore_case_search_survives_folding_that_changes_length() {
        // 'K' (Kelvin, U+212A) is three bytes and folds to one. Offsets taken
        // from a lower-cased copy point inside the '€' and panic on the slice.
        let haystack = "K€ password=x";
        let range = find_ignore_case(haystack, "password").expect("match");
        assert_eq!(&haystack[range], "password");

        // 'İ' folds to two code points, making the lower-cased copy longer.
        let haystack = "İ password=x";
        let range = find_ignore_case(haystack, "password").expect("match");
        assert_eq!(&haystack[range], "password");
    }

    #[test]
    fn ignore_case_search_matches_partial_starts() {
        // A failed partial match must not skip a later start that does match.
        assert_eq!(find_ignore_case("aaab", "aab"), Some(1..4));
        // "SSß " is 5 bytes ('ß' is two), so the match starts at 5.
        assert_eq!(find_ignore_case("SSß secret", "secret"), Some(5..11));
    }
}
