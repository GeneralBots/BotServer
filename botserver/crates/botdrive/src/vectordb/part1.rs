use super::*;

/// Strip HTML tags/entities/whitespace noise from a spreadsheet cell
/// value. The patterns are constants; if one somehow failed to compile
/// the corresponding step is skipped instead of panicking (#1368).
#[cfg(feature = "sheet")]
pub(super) fn sanitize_cell_text(raw: &str) -> String {
    pub(super) fn compile(pattern: &str) -> Option<regex::Regex> {
        match regex::Regex::new(pattern) {
            Ok(re) => Some(re),
            Err(e) => {
                log::error!("sanitize_cell_text pattern invalid ({pattern}): {e}");
                None
            }
        }
    }
    let s1 = compile(r"(?is)<(style|script)[^>]*>.*?</\1>")
        .map(|re| re.replace_all(raw, "").to_string())
        .unwrap_or_else(|| raw.to_string());
    let s2 = compile(r"<[^>]*>")
        .map(|re| re.replace_all(&s1, "").to_string())
        .unwrap_or_else(|| s1);
    let s3 = s2.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"");
    compile(r"\s+")
        .map(|re| re.replace_all(&s3, " ").to_string())
        .unwrap_or(s3)
        .trim()
        .to_string()
}
