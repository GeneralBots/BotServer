//! Number format engine (#785).
//!
//! Parses a spreadsheet number format code (`#,##0.00`, `0%`, `$#,##0.00`,
//! `yyyy-mm-dd`, `0.00E+00`, fractions) and renders a typed number to its
//! display string. The grid previously stored formats without applying them;
//! this is the engine that makes `R$ 1.234,50` display correctly.

use super::format_render::{
    detect_date_format, render_fraction, render_scientific, serial_to_datetime,
};
use super::value::CellValue;

/// The parsed structure of a number format.
#[derive(Clone, Debug, PartialEq)]
pub struct NumberFormat {
    /// Whether the format is a date/time code rather than a numeric code.
    pub is_date: bool,
    /// Number of leading integer digits enforced by `0`.
    pub min_integer_digits: usize,
    /// Number of decimal digits enforced by `0` after the point.
    pub min_decimal_digits: usize,
    /// Maximum decimal digits (`0` + `#` + `?` after the point); trailing
    /// digits beyond `min` are trimmed so `0.##` shows `1.5`, not `2`.
    pub max_decimal_digits: usize,
    /// Whether a thousands separator is requested (`#,##0`).
    pub use_thousands: bool,
    /// Whether the value is rendered as a percentage (`0%`).
    pub percent: bool,
    /// Currency symbol prefix such as `R$ ` or `$`.
    pub currency: Option<String>,
    /// Fixed scale factor (1 for normal, 100 for percent).
    pub scale: f64,
    /// For date formats, the format string in chrono syntax where derivable.
    pub date_format: Option<String>,
    /// Scientific notation (`0.00E+00`, `#.##E-00`).
    pub scientific: bool,
    /// Fraction denominator digit count (`# ?/?` → 1, `# ??/??` → 2).
    pub fraction_denominator: Option<usize>,
    /// Accounting format (`_($* #,##0.00_)`): `_`/`*` markers are stripped and
    /// negatives render parenthesised.
    pub accounting: bool,
    /// Whether a negative renders as `(1,234.50)` rather than `-1,234.50`.
    pub neg_in_parens: bool,
    /// Raw zero-section code (third `;` section): how a zero renders, e.g.
    /// `"-"` in `0.00;-0.00;"-"`.
    pub zero_code: Option<String>,
    /// Raw text-section code (fourth `;` section): how text renders, e.g.
    /// `"Total: "@`.
    pub text_code: Option<String>,
}

impl Default for NumberFormat {
    fn default() -> Self {
        NumberFormat {
            is_date: false,
            min_integer_digits: 1,
            min_decimal_digits: 0,
            max_decimal_digits: 0,
            use_thousands: false,
            percent: false,
            currency: None,
            scale: 1.0,
            date_format: None,
            scientific: false,
            fraction_denominator: None,
            accounting: false,
            neg_in_parens: false,
            zero_code: None,
            text_code: None,
        }
    }
}

/// Locale for numeric display: which characters separate thousands and
/// decimals. The format code itself is locale-independent; only the rendering
/// swaps the separators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NumberLocale {
    pub decimal_sep: char,
    pub thousands_sep: char,
}

impl NumberLocale {
    /// en-US: `1,234.50`.
    pub const EN: NumberLocale = NumberLocale {
        decimal_sep: '.',
        thousands_sep: ',',
    };
    /// pt-BR: `1.234,50`.
    pub const PT: NumberLocale = NumberLocale {
        decimal_sep: ',',
        thousands_sep: '.',
    };
}

impl Default for NumberLocale {
    fn default() -> Self {
        NumberLocale::EN
    }
}

/// Parses a format code into its parts. Unknown formats fall back to General.
pub fn parse_format(code: &str) -> NumberFormat {
    let mut fmt = NumberFormat::default();
    let code = code.trim();
    if code.is_empty() || code.eq_ignore_ascii_case("general") {
        return fmt;
    }

    let date_codes = ["yyyy", "yy", "mm", "dd", "hh", "h:", "ss", "mmm", "ddd"];
    if date_codes.iter().any(|c| code.to_ascii_lowercase().contains(c)) {
        fmt.is_date = true;
        fmt.date_format = detect_date_format(code);
        return fmt;
    }

    // Scientific notation (`0.00E+00`, `#.##E-00`).
    let upper = code.to_ascii_uppercase();
    if upper.contains("E+") || upper.contains("E-") {
        fmt.scientific = true;
        let mantissa = code.split(['E', 'e']).next().unwrap_or("0");
        fmt.min_decimal_digits = mantissa
            .split_once('.')
            .map(|(_, frac)| {
                frac.chars()
                    .take_while(|c| *c == '0' || *c == '#')
                    .filter(|c| *c == '0')
                    .count()
            })
            .unwrap_or(0);
        return fmt;
    }

    // Fractions (`# ?/?`, `# ??/??`): the `?` count after the slash sets the
    // denominator precision.
    if let Some(slash) = code.rfind('/') {
        let denom = &code[slash + 1..];
        let qmarks = denom.chars().filter(|c| *c == '?').count();
        if qmarks > 0 {
            fmt.fraction_denominator = Some(qmarks);
            return fmt;
        }
    }

    // Accounting: `_`/`*` markers and parenthesised negative sections. Strip
    // the alignment markers and parse the positive section's numeric pattern.
    fmt.accounting = code.contains('_') || code.contains('*');
    let sections: Vec<&str> = code.split(';').collect();
    fmt.neg_in_parens = sections.get(1).map(|s| s.contains('(')).unwrap_or(false);
    if fmt.accounting {
        fmt.neg_in_parens = true;
    }
    // Zero and text sections drive how `0` and text values render.
    fmt.zero_code = sections.get(2).map(|s| s.to_string());
    fmt.text_code = sections.get(3).map(|s| s.to_string());
    let mut rest = strip_alignment(sections.first().copied().unwrap_or(code));
    // Locale/colour token `[$-409]` / `[$€-407]`: dropped, the number pattern
    // follows the bracket.
    rest = strip_locale_token(&rest).to_string();
    // Currency prefix.
    if let Some((sym, tail)) = split_currency(&rest) {
        fmt.currency = Some(sym);
        rest = tail;
    }
    // Percent.
    if rest.contains('%') {
        fmt.percent = true;
        fmt.scale = 100.0;
    }
    // Thousands separators.
    if rest.contains(',') {
        fmt.use_thousands = true;
    }
    // Decimal places: `0` forces a digit, `#`/`?` are optional. `min` is the
    // forced count; `max` is the total width before trailing zeros are trimmed.
    if let Some((_, frac)) = rest.split_once('.') {
        let frac: String = frac.chars().take_while(|c| *c == '0' || *c == '#' || *c == '?').collect();
        fmt.min_decimal_digits = frac.chars().filter(|c| *c == '0').count();
        fmt.max_decimal_digits = frac
            .chars()
            .filter(|c| *c == '0' || *c == '#' || *c == '?')
            .count();
    }
    // Minimum integer digits.
    let int_part = rest.split(['.', '%', ',']).next().unwrap_or("");
    fmt.min_integer_digits = int_part.chars().filter(|c| *c == '0').count().max(1);

    fmt
}

/// Strips accounting alignment markers from a format section: `_` reserves the
/// width of the following character and `*` repeats it to fill — both are
/// alignment hints, not value content. Parentheses are negative-section markers.
fn strip_alignment(section: &str) -> String {
    let chars: Vec<char> = section.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '_' | '*' => i += 1, // consume the marker and its width/fill char
            '(' | ')' => {}
            c => out.push(c),
        }
        i += 1;
    }
    out
}

/// Extracts a leading currency prefix from a format section: a quoted symbol
/// (`"R$" #,##0.00`) or a bare symbol (`R$ `, `$`, `€`, `£`). Returns
/// `(symbol, rest_of_code)`.
fn split_currency(code: &str) -> Option<(String, String)> {
    // Quoted symbol: `"R$" #,##0.00`, `"USD "0.00`.
    if let Some(rest) = code.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            let sym = rest[..end].trim_end();
            if !sym.is_empty() {
                return Some((sym.to_string(), rest[end + 1..].to_string()));
            }
        }
    }
    for sym in ["R$ ", "R$", "$ ", "$", "€ ", "€", "£ ", "£"] {
        if code.starts_with(sym) {
            return Some((
                sym.trim_end().to_string(),
                code.strip_prefix(sym).unwrap_or_default().to_string(),
            ));
        }
    }
    None
}

/// Strips a leading `[...]` token (locale code `[$-409]` / `[$€-407]` or a
/// colour like `[Red]`), returning the rest of the section. The token carries
/// no display content for the grid, so it is dropped.
fn strip_locale_token(code: &str) -> &str {
    if let Some(rest) = code.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            return &rest[end + 1..];
        }
    }
    code
}

/// Renders a typed value with the given format code (en-US separators).
pub fn apply_format(value: &CellValue, code: &str) -> String {
    apply_format_locale(value, code, NumberLocale::EN)
}

/// Locale-aware variant; see [`render_number_locale`].
pub fn apply_format_locale(value: &CellValue, code: &str, locale: NumberLocale) -> String {
    let fmt = parse_format(code);
    match value {
        CellValue::Number(n) => render_number_locale(*n, &fmt, locale),
        CellValue::Text(t) => {
            if let Some(text_code) = &fmt.text_code {
                return render_text_section(text_code, t);
            }
            t.clone()
        }
        CellValue::Bool(b) => if *b { "TRUE".to_string() } else { "FALSE".to_string() },
        CellValue::Empty => String::new(),
        CellValue::Error(e) => e.clone(),
    }
}

/// Test-only EN-locale shorthand; production callers use the locale variants.
#[cfg(test)]
fn render_number(n: f64, fmt: &NumberFormat) -> String {
    render_number_locale(n, fmt, NumberLocale::EN)
}

/// Locale-aware rendering: `NumberLocale::PT` swaps the thousands/decimal
/// separators (`1.234,50`), the remaining E7 item for pt-BR display.
fn render_number_locale(n: f64, fmt: &NumberFormat, locale: NumberLocale) -> String {
    if n == 0.0 {
        if let Some(zero_code) = &fmt.zero_code {
            return render_section_literal(zero_code);
        }
    }
    if fmt.is_date {
        if let (Some(df), Some(dt)) = (fmt.date_format.as_deref(), serial_to_datetime(n)) {
            return dt.format(df).to_string();
        }
        return super::value::format_number(n);
    }
    if fmt.scientific {
        return render_scientific(n, fmt.min_decimal_digits);
    }
    if let Some(denom_digits) = fmt.fraction_denominator {
        return render_fraction(n, denom_digits);
    }
    // Percent scales by 100: 0.125 with "0.0%" renders as "12.5%".
    let scaled = n * fmt.scale;
    let neg = scaled < 0.0;
    let abs = scaled.abs();
    let factor = 10f64.powi(fmt.max_decimal_digits as i32);
    let rounded = (abs * factor).round() / factor;

    let mut int_part = rounded.trunc() as i64;
    let mut frac_part = (rounded.fract() * factor).round() as u64;

    // Rounding carry (e.g. 0.999 → 1.000 at three decimals).
    if frac_part == factor as u64 {
        int_part += 1;
        frac_part = 0;
    }

    let mut int_str = int_part.to_string();
    if fmt.use_thousands {
        int_str = group_thousands(&int_str, locale.thousands_sep);
    }
    while int_str.len() < fmt.min_integer_digits {
        int_str.insert(0, '0');
    }

    let mut out = String::new();
    if let Some(ref cur) = fmt.currency {
        out.push_str(cur);
        out.push(' ');
    }
    if neg {
        out.push(if fmt.neg_in_parens { '(' } else { '-' });
    }
    out.push_str(&int_str);
    // Render `max` decimals then trim trailing zeros down to the forced `min`:
    // `0.00` always shows two, `0.##` shows up to two (`1.5`, `1`, `1.25`).
    if fmt.max_decimal_digits > 0 {
        let mut frac_str = format!("{:0width$}", frac_part, width = fmt.max_decimal_digits);
        while frac_str.len() > fmt.min_decimal_digits && frac_str.ends_with('0') {
            frac_str.pop();
        }
        if !frac_str.is_empty() {
            out.push(locale.decimal_sep);
            out.push_str(&frac_str);
        }
    }
    if neg && fmt.neg_in_parens {
        out.push(')');
    }
    if fmt.percent {
        out.push('%');
    }
    out
}

fn group_thousands(int_str: &str, sep: char) -> String {
    let bytes = int_str.as_bytes();
    let len = bytes.len();
    let mut out = String::new();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push(sep);
        }
        out.push(*b as char);
    }
    out
}

/// Renders a literal format section (the zero or text section): strips colour
/// tokens (`[Red]`), quote delimiters, alignment markers (`_x`, `*x`) and
/// space placeholders (`?`), leaving the displayed literal. The accounting
/// zero section `_-* "-"??_-` therefore renders as `-`.
fn render_section_literal(code: &str) -> String {
    let chars: Vec<char> = code.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '"' => {}
            '[' => {
                // Skip a colour/condition token up to its closing bracket.
                while i < chars.len() && chars[i] != ']' {
                    i += 1;
                }
            }
            '_' | '*' => i += 1, // alignment marker + the width/fill char
            '?' => {}
            c => out.push(c),
        }
        i += 1;
    }
    out.trim().to_string()
}

/// Renders the text section (fourth `;` section): substitutes `@` with the
/// cell text (`"Total: "@` → `Total: 42`). A section without `@` is literal.
fn render_text_section(code: &str, text: &str) -> String {
    let literal = render_section_literal(code);
    if literal.contains('@') {
        literal.replace('@', text)
    } else if literal.is_empty() || literal.eq_ignore_ascii_case("general") {
        text.to_string()
    } else {
        literal
    }
}

/// Applies a stored cell format to a cell value, falling back to the plain
/// number display when the code is empty or General.
pub fn display_cell(value: &CellValue, format_code: Option<&str>) -> String {
    display_cell_locale(value, format_code, NumberLocale::EN)
}

/// Locale-aware variant; see [`render_number_locale`].
pub fn display_cell_locale(
    value: &CellValue,
    format_code: Option<&str>,
    locale: NumberLocale,
) -> String {
    match format_code {
        Some(code) if !code.trim().is_empty() && !code.eq_ignore_ascii_case("general") => {
            apply_format_locale(value, code, locale)
        }
        _ => value.display(),
    }
}


/// Applies each cell's stored number format to its display `value`, keeping the
/// raw `typed` value untouched so arithmetic and editing stay lossless (#785).
/// Operates on a response clone; the persisted model keeps raw values.
pub fn apply_formats_to_sheet(sheet: &mut crate::types::Spreadsheet) {
    apply_formats_to_sheet_locale(sheet, NumberLocale::EN);
}

/// Locale-aware variant; see [`render_number_locale`].
pub fn apply_formats_to_sheet_locale(sheet: &mut crate::types::Spreadsheet, locale: NumberLocale) {
    for ws in &mut sheet.worksheets {
        for cell in ws.data.values_mut() {
            if let (Some(typed), Some(code)) = (cell.typed.clone(), cell.format.clone()) {
                cell.value = Some(display_cell_locale(&typed, Some(&code), locale));
            }
        }
    }
}

#[cfg(test)]
#[path = "formats_tests.rs"]
mod tests;