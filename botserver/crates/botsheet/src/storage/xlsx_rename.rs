//! Sheet-rename reference rewriting (E4).
//!
//! Renaming a worksheet changes its name in `xl/workbook.xml`, but every
//! formula and defined name that references the sheet by its old name must be
//! updated too — otherwise Excel resolves the stale name to `#REF!`. This
//! module rewrites only formula bodies (`<f>…</f>`) and defined-name bodies
//! (`<definedName>…</definedName>`), so cell values and shared strings are
//! never touched.

/// Rewrites cross-sheet references in worksheet formula bodies and workbook
/// defined names for each `(old_name, new_name)` rename.
pub fn rename_sheet_references(entries: &mut [(String, Vec<u8>)], renames: &[(String, String)]) {
    for (name, data) in entries.iter_mut() {
        let patched = if name.starts_with("xl/worksheets/") && name.ends_with(".xml") {
            rewrite_formula_bodies(data, renames)
        } else if name == "xl/workbook.xml" {
            rewrite_defined_name_bodies(data, renames)
        } else {
            continue;
        };
        *data = patched.into_bytes();
    }
}

fn rewrite_formula_bodies(xml: &[u8], renames: &[(String, String)]) -> String {
    let Some(f_re) = crate::re_cache::cached_re!(r"(?s)(<f\b[^>]*>)(.*?)(</f>)") else {
        return String::from_utf8_lossy(xml).to_string();
    };
    let text = String::from_utf8_lossy(xml);
    f_re
        .replace_all(&text, |caps: &regex::Captures| {
            format!(
                "{}{}{}",
                &caps[1],
                rewrite_refs(&caps[2], renames),
                &caps[3]
            )
        })
        .to_string()
}

fn rewrite_defined_name_bodies(xml: &[u8], renames: &[(String, String)]) -> String {
    let Some(dn_re) = crate::re_cache::cached_re!(r"(?s)(<definedName\b[^>]*>)(.*?)(</definedName>)") else {
        return String::from_utf8_lossy(xml).to_string();
    };
    let text = String::from_utf8_lossy(xml);
    dn_re
        .replace_all(&text, |caps: &regex::Captures| {
            format!(
                "{}{}{}",
                &caps[1],
                rewrite_refs(&caps[2], renames),
                &caps[3]
            )
        })
        .to_string()
}

fn rewrite_refs(text: &str, renames: &[(String, String)]) -> String {
    let mut out = text.to_string();
    for (old, new) in renames {
        out = rewrite_one(&out, old, new);
    }
    out
}

fn rewrite_one(text: &str, old: &str, new: &str) -> String {
    let old_ref = sheet_ref(old);
    let new_ref = sheet_ref(new);
    // Match `Old!` (or `'Old name'!`) only when not preceded by an identifier
    // character, so `MySheet1!` is not rewritten by a `Sheet1` rename.
    let pattern = format!(r"(^|[^A-Za-z0-9_.]){}!", regex::escape(&old_ref));
    let Some(re) = crate::re_cache::dynamic(&pattern) else {
        return text.to_string();
    };
    re.replace_all(text, |caps: &regex::Captures| {
        format!(
            "{}{}!",
            caps.get(1).map(|m| m.as_str()).unwrap_or(""),
            new_ref
        )
    })
    .to_string()
}

/// Renders a sheet name as Excel does in a reference: bare when it is a simple
/// identifier, single-quoted (with `''` escaping) otherwise.
fn sheet_ref(name: &str) -> String {
    let simple = !name.is_empty()
        && name
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic() || c == '_')
            .unwrap_or(false)
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.');
    if simple {
        name.to_string()
    } else {
        format!("'{}'", name.replace('\'', "''"))
    }
}
