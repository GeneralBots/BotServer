use regex::Regex;
use std::sync::OnceLock;

/// Organization masking rule: `(prefix_keep, mask_len, suffix_keep)`.
///
/// A credential-shaped token is rewritten as its first `prefix_keep`
/// characters, followed by `mask_len` asterisks, followed by its last
/// `suffix_keep` characters.
pub type RedactRule = (usize, usize, usize);

const MIN_DIGIT_RUN: usize = 13;
const MIN_ENTROPY_TOKEN: usize = 24;
const ENTROPY_THRESHOLD_BITS: f64 = 3.5;

const DIGIT_RUN_PATTERN: &str = r"[0-9]{13,}";
const ENTROPY_TOKEN_PATTERN: &str = r"[A-Za-z0-9+/=_\-]{24,}";
const EMAIL_PATTERN: &str = r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}";
const WORDISH_TOKEN_PATTERN: &str = r"[A-Za-z0-9._\-/+]+";

fn compile(pattern: &'static str) -> Option<Regex> {
    match Regex::new(pattern) {
        Ok(re) => Some(re),
        Err(e) => {
            tracing::error!("botconnectors: redaction pattern '{pattern}' failed to compile: {e}");
            None
        }
    }
}

fn digit_run_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile(DIGIT_RUN_PATTERN)).as_ref()
}

fn entropy_token_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile(ENTROPY_TOKEN_PATTERN)).as_ref()
}

fn email_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile(EMAIL_PATTERN)).as_ref()
}

fn wordish_token_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile(WORDISH_TOKEN_PATTERN)).as_ref()
}

/// Shannon entropy in bits per character over the byte histogram.
fn entropy_bits_per_char(token: &str) -> f64 {
    let bytes = token.as_bytes();
    let len = bytes.len();
    if len == 0 {
        return 0.0;
    }
    let mut counts = [0usize; 256];
    for b in bytes {
        counts[*b as usize] += 1;
    }
    let mut entropy = 0.0f64;
    for count in counts.iter().filter(|c| **c > 0) {
        let p = *count as f64 / len as f64;
        entropy -= p * p.log2();
    }
    entropy
}

fn mask_keep_last4(digits: &str) -> String {
    // `DIGIT_RUN_PATTERN` is ASCII-only today, so a byte cut cannot split a
    // character here. The offsets are still taken on char boundaries and the
    // mask counts characters, so widening the pattern (a multi-byte digit
    // range, or `\w+` for wordish tokens) can neither panic nor miscount.
    let tail_start = botlib::text::char_boundary_at_or_before(digits, digits.len() - 4);
    let masked = digits[..tail_start].chars().count();
    format!("{}{}", "*".repeat(masked), &digits[tail_start..])
}

fn looks_credential_shaped(token: &str) -> bool {
    token.bytes().any(|b| b.is_ascii_digit()) && token.bytes().any(|b| b.is_ascii_alphabetic())
}

fn apply_org_rules(input: &str, rules: &[RedactRule]) -> String {
    if rules.is_empty() {
        return input.to_string();
    }
    match wordish_token_re() {
        Some(re) => re
            .replace_all(input, |caps: &regex::Captures| {
                let token: &str = caps.get(0).map(|m| m.as_str()).unwrap_or("");
                for &(prefix_keep, mask_len, suffix_keep) in rules {
                    let min_len = prefix_keep + mask_len + suffix_keep;
                    if token.len() >= min_len && looks_credential_shaped(token) {
                        // `WORDISH_TOKEN_PATTERN` is ASCII-only today; the cuts
                        // are char-boundary safe so a widened pattern cannot
                        // panic on a multi-byte token.
                        let prefix = botlib::text::truncate_at_char_boundary(token, prefix_keep);
                        let suffix_start = botlib::text::char_boundary_at_or_before(
                            token,
                            token.len() - suffix_keep,
                        );
                        return format!(
                            "{}{}{}",
                            prefix,
                            "*".repeat(mask_len),
                            &token[suffix_start..]
                        );
                    }
                }
                token.to_string()
            })
            .into_owned(),
        None => input.to_string(),
    }
}

/// Redact sensitive material from free text before it is indexed or returned.
///
/// Built-in passes, applied in order:
/// 1. e-mail addresses — local part fully masked, domain preserved;
/// 2. digit runs of at least [`MIN_DIGIT_RUN`] characters — masked keeping the last four digits;
/// 3. high-entropy tokens of at least [`MIN_ENTROPY_TOKEN`] characters — replaced with `[REDACTED]`;
/// 4. organization rules — credential-shaped tokens rewritten as `prefix***suffix`.
pub fn redact(input: &str, rules: &[RedactRule]) -> String {
    let mut text = input.to_string();

    if let Some(re) = email_re() {
        text = re
            .replace_all(&text, |caps: &regex::Captures| {
                let matched = caps.get(0).map(|m| m.as_str()).unwrap_or("");
                match matched.find('@') {
                    Some(at) => format!("***{}", &matched[at..]),
                    None => matched.to_string(),
                }
            })
            .into_owned();
    }

    if let Some(re) = digit_run_re() {
        text = re.replace_all(&text, |caps: &regex::Captures| {
            let run: &str = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            if run.len() >= MIN_DIGIT_RUN {
                mask_keep_last4(run)
            } else {
                run.to_string()
            }
        }).into_owned();
    }

    if let Some(re) = entropy_token_re() {
        text = re.replace_all(&text, |caps: &regex::Captures| {
            let token: &str = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            if token.chars().count() >= MIN_ENTROPY_TOKEN
                && entropy_bits_per_char(token) >= ENTROPY_THRESHOLD_BITS
            {
                "[REDACTED]".to_string()
            } else {
                token.to_string()
            }
        }).into_owned();
    }

    apply_org_rules(&text, rules)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_redacted(input: &str, expected: &str) {
        assert_eq!(redact(input, &[]), expected);
    }

    #[test]
    fn masks_long_digit_runs_keeping_last_four() {
        assert_redacted("card 4111111111111111 on file", "card ************1111 on file");
    }

    #[test]
    fn keeps_short_digit_runs() {
        assert_redacted("order 123456789012 shipped", "order 123456789012 shipped");
    }

    #[test]
    fn masks_email_local_part_keeps_domain() {
        assert_redacted(
            "contact jane.doe+corp@example.com today",
            "contact ***@example.com today",
        );
    }

    #[test]
    fn masks_high_entropy_tokens() {
        let secret = format!("Zm9vYmFy{tail}", tail = "c2VjcmV0dmFsdWUxMjM0NTY3ODlhYmNkZWY=");
        let out = redact(&format!("token {secret} end"), &[]);
        assert!(out.contains("[REDACTED]"), "got: {out}");
        assert!(!out.contains(&secret));
    }

    #[test]
    fn keeps_low_entropy_long_tokens() {
        let plain = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        assert_redacted(plain, plain);
    }

    #[test]
    fn applies_org_rules_as_prefix_mask_suffix() {
        let rules: Vec<RedactRule> = vec![(4, 8, 3)];
        assert_eq!(redact("key abcd1234efgh5678 done", &rules), "key abcd********678 done");
    }

    #[test]
    fn org_rules_ignore_plain_words() {
        let rules: Vec<RedactRule> = vec![(4, 8, 3)];
        assert_eq!(redact("extraordinary", &rules), "extraordinary");
    }

    /// #1508 — the token pattern is ASCII-only, so non-ASCII text must pass
    /// through the org rules untouched. This pins the pattern contract: if it
    /// is ever widened, this test fails and points at the boundary-safe cuts
    /// below (which exist so that widening cannot panic).
    #[test]
    fn org_rules_leave_non_ascii_text_untouched() {
        let rules: Vec<RedactRule> = vec![(3, 6, 2)];
        let input = "chave ção1éabc2ée123456789 fim";
        assert_eq!(redact(input, &rules), input);
    }

    /// The mask counts characters, not bytes: with a multi-byte digit run the
    /// kept suffix is the last four characters, not the last `len() - 4` bytes.
    #[test]
    fn keep_last4_counts_characters_for_multibyte_runs() {
        let digits = "١٢٣٤٥٦٧٨٩٠١٢٣٤٥";
        let masked = mask_keep_last4(digits);
        assert!(masked.ends_with("٤٥"), "got: {masked}");
        assert_eq!(masked.chars().filter(|c| *c == '*').count(), 13, "got: {masked}");
    }

    #[test]
    fn combined_passes_redact_card_inside_email_body() {
        assert_redacted(
            "mail bob@corp.io paid 378282246310005 ok",
            "mail ***@corp.io paid ***********0005 ok",
        );
    }
}
