//! Shared-string table handling for the xlsx preserve-and-passthrough (#788, E1).
//!
//! Text cells in an xlsx are normally stored as `t="s"` with an index into
//! `xl/sharedStrings.xml`. This module parses that table into an
//! `escaped text -> index` map, appends new strings as a cell edit introduces
//! them, and rewrites the table (with its `count`/`uniqueCount` attributes).
//! Keys stay XML-escaped exactly as stored, so a cell writer escapes its value
//! and looks it up without decoding entities.

use std::collections::HashMap;

/// The shared-string table, plus the state needed to append during a merge.
pub(crate) struct SharedStrings {
    index_of: HashMap<String, usize>,
    /// Number of existing unique entries (the index the first append takes).
    pub(crate) unique: usize,
    /// New escaped strings appended during this merge, in order.
    pub(crate) appended: Vec<String>,
    /// Whether the package carries a `sharedStrings.xml` table at all. Without
    /// one there is no part to append to, so text cells stay inline strings.
    has_table: bool,
}

impl SharedStrings {
    pub(crate) fn from_xml(xml: Option<&str>) -> Self {
        let mut index_of = HashMap::new();
        let mut unique = 0usize;
        if let Some(xml) = xml {
            let si_re = crate::re_cache::cached_re!(r"(?s)<si>(.*?)</si>");
            let t_re = crate::re_cache::cached_re!(r"(?s)<t(?: [^>]*)?>(.*?)</t>");
            if let (Some(si_re), Some(t_re)) = (si_re, t_re) {
                for si in si_re.captures_iter(xml) {
                    let mut text = String::new();
                    if let Some(body) = si.get(1) {
                        for t in t_re.captures_iter(body.as_str()) {
                            if let Some(m) = t.get(1) {
                                text.push_str(m.as_str());
                            }
                        }
                    }
                    index_of.entry(text).or_insert(unique);
                    unique += 1;
                }
            }
        }
        SharedStrings {
            index_of,
            unique,
            appended: Vec::new(),
            has_table: xml.is_some(),
        }
    }

    /// Returns the index for `escaped`, appending it when it is new. Returns
    /// `None` when there is no shared-string table to write into.
    pub(crate) fn lookup_or_append(&mut self, escaped: &str) -> Option<usize> {
        if !self.has_table {
            return None;
        }
        if let Some(&idx) = self.index_of.get(escaped) {
            return Some(idx);
        }
        let idx = self.unique + self.appended.len();
        self.appended.push(escaped.to_string());
        Some(idx)
    }
}

/// Appends new `<si>` entries to `xl/sharedStrings.xml` and bumps the
/// `uniqueCount`/`count` attributes. `count` is a preallocation hint (Excel and
/// LibreOffice recompute it), so it is bumped by the append count rather than
/// reconciled per-cell; `uniqueCount` is exact.
pub(crate) fn rewrite_shared_strings(
    xml: &str,
    appended: &[String],
    old_unique: usize,
) -> Result<String, String> {
    let close = xml.rfind("</sst>").ok_or("missing </sst>")?;

    let mut new_si = String::new();
    for escaped in appended {
        new_si.push_str(&format!(
            r#"<si><t xml:space="preserve">{escaped}</t></si>"#
        ));
    }

    let mut out = String::with_capacity(xml.len() + new_si.len());
    out.push_str(&xml[..close]);
    out.push_str(&new_si);
    out.push_str(&xml[close..]);

    let new_unique = old_unique + appended.len();
    out = bump_int_attr(&out, "uniqueCount", new_unique);
    out = bump_int_attr_add(&out, "count", appended.len());
    Ok(out)
}

/// Replaces `attr="N"` with the given value, or inserts the attribute after the
/// opening `<sst` tag when it is absent.
fn bump_int_attr(xml: &str, attr: &str, value: usize) -> String {
    let pattern = format!(r#"\b{attr}="(\d+)""#);
    let Some(re) = crate::re_cache::dynamic(&pattern) else {
        return xml.to_string();
    };
    if re.is_match(xml) {
        re.replace(xml, format!(r#"{attr}="{value}""#)).to_string()
    } else {
        xml.replacen(
            "<sst",
            &format!(r#"<sst {attr}="{value}""#),
            1,
        )
    }
}

/// Like [`bump_int_attr`] but ADDS `delta` to the existing value (used for
/// `count`, whose base is read from the original document).
fn bump_int_attr_add(xml: &str, attr: &str, delta: usize) -> String {
    let pattern = format!(r#"\b{attr}="(\d+)""#);
    let Some(re) = crate::re_cache::dynamic(&pattern) else {
        return xml.to_string();
    };
    if re.is_match(xml) {
        re.replace(xml, |caps: &regex::Captures| {
            let base: usize = caps[1].parse().unwrap_or(0);
            format!(r#"{attr}="{}""#, base + delta)
        })
        .to_string()
    } else {
        xml.replacen("<sst", &format!(r#"<sst {attr}="{delta}""#), 1)
    }
}
