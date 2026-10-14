//! Scientific, fraction and date/time number rendering (#785, E7).
//!
//! The `0.00E+00`, `# ?/?` and `yyyy-mm-dd hh:mm` style renderers. Kept
//! separate from the format parser so `formats.rs` stays under the file-size
//! ceiling.

/// Renders a number in `0.00E+00` style: a mantissa with the requested decimal
/// places and a two-digit, sign-prefixed exponent.
pub(crate) fn render_scientific(n: f64, decimal_digits: usize) -> String {
    if n == 0.0 {
        return format!("0.{}E+00", "0".repeat(decimal_digits));
    }
    let mut exp = n.abs().log10().floor() as i32;
    let mantissa = n / 10f64.powi(exp);
    let factor = 10f64.powi(decimal_digits as i32);
    let mut rounded = (mantissa * factor).round() / factor;
    // Normalize a rounding carry: 9.999 → 10.00 must become 1.00E+01.
    if rounded.abs() >= 10.0 {
        rounded /= 10.0;
        exp += 1;
    }
    format!("{:.*}E{:+03}", decimal_digits, rounded, exp)
}

/// Renders a number as the closest fraction with a denominator up to
/// `10^denom_digits - 1` (Excel's `# ?/?` and `# ??/??` behaviour).
pub(crate) fn render_fraction(n: f64, denom_digits: usize) -> String {
    let max_denom = 10usize.pow(denom_digits as u32) - 1;
    let whole = n.trunc();
    let frac_abs = (n - whole).abs();

    let mut best_num = 0usize;
    let mut best_den = 1usize;
    let mut best_err = f64::MAX;
    for den in 1..=max_denom.max(1) {
        let num = (frac_abs * den as f64).round() as usize;
        let err = ((num as f64 / den as f64) - frac_abs).abs();
        if err < best_err {
            best_err = err;
            best_num = num;
            best_den = den;
        }
    }

    let sign = if n < 0.0 { "-" } else { "" };
    let whole_part = if whole != 0.0 {
        format!("{} ", whole.abs() as i64)
    } else {
        String::new()
    };
    if best_num == 0 {
        format!("{sign}{}", whole.abs() as i64)
    } else {
        format!("{sign}{whole_part}{best_num}/{best_den}")
    }
}

/// Converts an Excel date serial (days since 1899-12-30, fractional part =
/// time of day) to a [`chrono::NaiveDateTime`]. A serial of `0.5` is noon of
/// the epoch day, not "no date at 00:00" — time-only formats depend on this.
pub(crate) fn serial_to_datetime(serial: f64) -> Option<chrono::NaiveDateTime> {
    let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30)?;
    let days = serial.floor();
    let date = base.checked_add_signed(chrono::Duration::days(days as i64))?;
    let frac = serial - days;
    // Round to the nearest second so 23:59:59.999 does not carry a day.
    let seconds = (frac * 86_400.0).round() as i64;
    let time = chrono::NaiveTime::from_num_seconds_from_midnight_opt(
        seconds.clamp(0, 86_399) as u32,
        0,
    )?;
    Some(date.and_time(time))
}

/// Maps an ECMA-376 date/time format code to a chrono `strftime` pattern.
///
/// `m`/`mm` is a month except when it immediately follows an hour code or
/// immediately precedes seconds (then it is minutes); `am/pm`/`a/p` maps to
/// `%p`. Bracketed elapsed-time markers (`[h]`, `[mm]`, `[ss]`) are dropped —
/// chrono has no elapsed-time mode, so they render as ordinary clock fields.
pub(crate) fn detect_date_format(code: &str) -> Option<String> {
    // Map the AM/PM designator to chrono `%p` before the char scan so the
    // surrounding literal handling is unaffected.
    let code = normalize_am_pm(code);
    let mut out = String::new();
    let mut chars = code.chars().peekable();
    let mut last_was_hour = false;
    while let Some(c) = chars.next() {
        match c.to_ascii_lowercase() {
            'y' => {
                let count = count_run(&mut chars, &['y', 'Y']);
                out.push_str(if count >= 4 { "%Y" } else { "%y" });
                last_was_hour = false;
            }
            'm' => {
                let count = count_run(&mut chars, &['m', 'M']);
                // `m`/`mm` is minutes when it follows an hour code (`h:mm`)
                // or immediately precedes seconds (`mm:ss`); otherwise it is
                // a month (ECMA-376 §18.8.30).
                let precedes_seconds = next_non_sep_is(&chars, &['s', 'S']);
                if last_was_hour || precedes_seconds {
                    out.push_str("%M");
                } else if count >= 3 {
                    out.push_str("%b");
                } else {
                    out.push_str("%m");
                }
                last_was_hour = false;
            }
            'd' => {
                let _ = count_run(&mut chars, &['d', 'D']);
                out.push_str("%d");
                last_was_hour = false;
            }
            'h' => {
                let _ = count_run(&mut chars, &['h', 'H']);
                out.push_str("%H");
                last_was_hour = true;
            }
            's' => {
                let _ = count_run(&mut chars, &['s', 'S']);
                out.push_str("%S");
                last_was_hour = false;
            }
            '[' | ']' => {
                // Elapsed-time brackets carry no chrono meaning; drop them.
            }
            '/' | '-' | ':' | ' ' => out.push(c),
            _ => {
                // Text between codes (including the pre-inserted `%p`):
                // keep as literal so chrono renders it verbatim.
                out.push(c);
            }
        }
    }
    if out.is_empty() { None } else { Some(out) }
}

/// Counts the leading run of characters in `set` from `chars.peek()`, consuming
/// them. The caller has already seen the first character, so the base count is 1.
fn count_run(chars: &mut std::iter::Peekable<std::str::Chars>, set: &[char]) -> usize {
    let mut count = 1;
    while chars
        .peek()
        .is_some_and(|c| set.contains(&c.to_ascii_lowercase()))
    {
        count += 1;
        chars.next();
    }
    count
}

/// Whether the next non-separator character is in `set` (lookahead without
/// consuming), used to decide `mm` = minutes vs month.
fn next_non_sep_is(chars: &std::iter::Peekable<std::str::Chars>, set: &[char]) -> bool {
    let mut clone = chars.clone();
    while let Some(&c) = clone.peek() {
        if c == ':' || c == ' ' || c == '.' {
            clone.next();
        } else {
            return set.contains(&c.to_ascii_lowercase());
        }
    }
    false
}

/// Replaces the AM/PM designator (`am/pm`, `AM/PM`, `a/p`, `A/P`) with chrono's
/// `%p` token, case-insensitively, leaving byte offsets of untouched text intact.
fn normalize_am_pm(code: &str) -> String {
    let mut out = replace_case_insensitive(code, "am/pm", "%p");
    out = replace_case_insensitive(&out, "a/p", "%p");
    out
}

/// Case-insensitive `str::replace` that preserves the original casing of all
/// text outside the matched spans.
fn replace_case_insensitive(haystack: &str, needle: &str, replacement: &str) -> String {
    let lower = haystack.to_ascii_lowercase();
    let mut out = String::with_capacity(haystack.len());
    let mut last = 0;
    for (start, _) in lower.match_indices(needle) {
        out.push_str(&haystack[last..start]);
        out.push_str(replacement);
        last = start + needle.len();
    }
    out.push_str(&haystack[last..]);
    out
}
