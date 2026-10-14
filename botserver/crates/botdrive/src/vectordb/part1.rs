
/// Strip HTML tags/entities/whitespace noise from a spreadsheet cell
/// value. The patterns are constants; if one somehow failed to compile
/// the corresponding step is skipped instead of panicking (#1368).
#[cfg(feature = "sheet")]
pub(super) fn sanitize_cell_text(raw: &str) -> String {
    let s1 = crate::re_cache::cached_re!(r"(?is)<(style|script)[^>]*>.*?</\1>")
        .map(|re| re.replace_all(raw, "").to_string())
        .unwrap_or_else(|| raw.to_string());
    let s2 = crate::re_cache::cached_re!(r"<[^>]*>")
        .map(|re| re.replace_all(&s1, "").to_string())
        .unwrap_or_else(|| s1);
    let s3 = s2.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"");
    crate::re_cache::cached_re!(r"\s+")
        .map(|re| re.replace_all(&s3, " ").to_string())
        .unwrap_or(s3)
        .trim()
        .to_string()
}
