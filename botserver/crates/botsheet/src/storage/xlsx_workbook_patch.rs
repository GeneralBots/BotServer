//! Workbook.xml / rels / content-types patching (E4).
//!
//! Applies a resolved [`WorkbookResolution`] to the zip entry list: rewrites
//! the `<sheets>` block, drops relationships and overrides of deleted sheets,
//! adds those of new sheets, and removes deleted worksheet parts. Also updates
//! formula and defined-name references when a sheet is renamed.

use std::collections::HashSet;

use super::xlsx_workbook::{first_capture, part_from_target, SheetBinding, WorkbookResolution};

const WORKSHEET_REL_TYPE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";
const WORKSHEET_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";

/// Rewrites workbook.xml / rels / content-types to match, removes deleted parts.
pub fn apply_workbook_changes(
    entries: &mut Vec<(String, Vec<u8>)>,
    resolution: &WorkbookResolution,
) -> Result<(), String> {
    if resolution.preserve_structure {
        return Ok(());
    }

    // A rename must also update every formula/defined-name reference to the
    // old sheet name, otherwise Excel shows #REF! for the stale references.
    let renames: Vec<(String, String)> = resolution
        .bindings
        .iter()
        .filter(|b| !b.is_new && b.orig_name != b.name)
        .map(|b| (b.orig_name.clone(), b.name.clone()))
        .collect();
    if !renames.is_empty() {
        crate::storage::xlsx_rename::rename_sheet_references(entries, &renames);
    }

    patch_sheets_block(entries, &resolution.bindings)?;
    patch_rels(entries, &resolution.bindings, &resolution.deleted_targets)?;
    patch_content_types(entries, &resolution.bindings, &resolution.deleted_targets)?;
    if !resolution.deleted_targets.is_empty() {
        entries.retain(|(n, _)| !resolution.deleted_targets.iter().any(|d| d == n));
    }
    Ok(())
}

fn patch_sheets_block(
    entries: &mut [(String, Vec<u8>)],
    bindings: &[SheetBinding],
) -> Result<(), String> {
    let Some(index) = entries.iter().position(|(n, _)| n == "xl/workbook.xml") else {
        return Ok(());
    };
    let xml = String::from_utf8_lossy(&entries[index].1).to_string();
    let block_re =
        crate::re_cache::cached_re!(r"(?s)<sheets\b[^>]*>.*?</sheets>").ok_or_else(|| "sheets regex".to_string())?;
    let Some(m) = block_re.find(&xml) else {
        return Ok(());
    };
    let mut inner = String::new();
    for b in bindings {
        match &b.orig_raw {
            Some(raw) => inner.push_str(&patch_sheet_tag(raw, &b.name, b.sheet_state.as_deref())),
            None => {
                let state = match &b.sheet_state {
                    Some(s) => format!(r#" state="{}""#, escape_attr(s)),
                    None => String::new(),
                };
                inner.push_str(&format!(
                    r#"<sheet name="{}" sheetId="{}" r:id="{}"{}/>"#,
                    escape_attr(&b.name),
                    b.sheet_id,
                    b.rid,
                    state
                ));
            }
        }
    }
    let mut out = String::with_capacity(xml.len() + inner.len());
    out.push_str(&xml[..m.start()]);
    out.push_str("<sheets>");
    out.push_str(&inner);
    out.push_str("</sheets>");
    out.push_str(&xml[m.end()..]);
    entries[index].1 = out.into_bytes();
    Ok(())
}

fn rename_tag(raw: &str, name: &str) -> String {
    let Some(name_re) = crate::re_cache::cached_re!(r#"name="[^"]*""#) else {
        return raw.to_string();
    };
    name_re
        .replace(raw, format!(r#"name="{}""#, escape_attr(name)))
        .to_string()
}

/// Renames a `<sheet>` tag and applies (or removes) its `state` visibility.
fn patch_sheet_tag(raw: &str, name: &str, state: Option<&str>) -> String {
    let renamed = rename_tag(raw, name);
    let stripped = strip_state(&renamed);
    match state {
        Some(s) => {
            let core = stripped.trim_end().trim_end_matches("/>").trim_end();
            format!(r#"{core} state="{}"/>"#, escape_attr(s))
        }
        None => stripped,
    }
}

/// Removes a `state="..."` attribute so it can be re-applied cleanly.
fn strip_state(tag: &str) -> String {
    let Some(re) = crate::re_cache::cached_re!(r#"\sstate="[^"]*""#) else {
        return tag.to_string();
    };
    re.replace_all(tag, "").to_string()
}

fn patch_rels(
    entries: &mut [(String, Vec<u8>)],
    bindings: &[SheetBinding],
    deleted_targets: &[String],
) -> Result<(), String> {
    let Some(index) = entries
        .iter()
        .position(|(n, _)| n == "xl/_rels/workbook.xml.rels")
    else {
        return Ok(());
    };
    let xml = String::from_utf8_lossy(&entries[index].1).to_string();

    let kept_rids: HashSet<&str> = bindings
        .iter()
        .filter(|b| !b.is_new)
        .map(|b| b.rid.as_str())
        .collect();
    let deleted_parts: HashSet<String> = deleted_targets.iter().cloned().collect();

    let rel_re =
        crate::re_cache::cached_re!(r#"<Relationship\b[^>]*/?>"#).ok_or_else(|| "rel regex".to_string())?;
    let id_re = crate::re_cache::cached_re!(r#"Id="([^"]*)""#).ok_or_else(|| "id regex".to_string())?;
    let target_re =
        crate::re_cache::cached_re!(r#"Target="([^"]*)""#).ok_or_else(|| "target regex".to_string())?;

    let mut inner = String::new();
    for tag in rel_re.find_iter(&xml) {
        let raw = tag.as_str();
        let drop = match (first_capture(&id_re, raw), first_capture(&target_re, raw)) {
            (Some(id), Some(target)) => {
                !kept_rids.contains(id.as_str()) && deleted_parts.contains(&part_from_target(&target))
            }
            _ => false,
        };
        if !drop {
            inner.push_str(raw);
        }
    }
    for b in bindings.iter().filter(|b| b.is_new) {
        inner.push_str(&format!(
            r#"<Relationship Id="{}" Type="{}" Target="{}"/>"#,
            b.rid,
            WORKSHEET_REL_TYPE,
            b.target.trim_start_matches("xl/")
        ));
    }

    let block_re = crate::re_cache::cached_re!(r"(?s)<Relationships\b[^>]*>.*?</Relationships>")
        .ok_or_else(|| "rels block regex".to_string())?;
    let Some(m) = block_re.find(&xml) else {
        return Ok(());
    };
    let mut out = String::with_capacity(xml.len() + inner.len());
    out.push_str(&xml[..m.start()]);
    out.push_str("<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">");
    out.push_str(&inner);
    out.push_str("</Relationships>");
    out.push_str(&xml[m.end()..]);
    entries[index].1 = out.into_bytes();
    Ok(())
}

fn patch_content_types(
    entries: &mut [(String, Vec<u8>)],
    bindings: &[SheetBinding],
    deleted_targets: &[String],
) -> Result<(), String> {
    let Some(index) = entries.iter().position(|(n, _)| n == "[Content_Types].xml") else {
        return Ok(());
    };
    let xml = String::from_utf8_lossy(&entries[index].1).to_string();

    let deleted: HashSet<String> = deleted_targets.iter().map(|t| format!("/{t}")).collect();
    let kept: HashSet<String> = bindings
        .iter()
        .filter(|b| !b.is_new)
        .map(|b| format!("/{}", b.target))
        .collect();

    let ov_re = crate::re_cache::cached_re!(r#"<Override\b[^>]*/?>"#).ok_or_else(|| "override regex".to_string())?;
    let part_re =
        crate::re_cache::cached_re!(r#"PartName="([^"]*)""#).ok_or_else(|| "partname regex".to_string())?;

    let mut inner = String::new();
    for tag in ov_re.find_iter(&xml) {
        let raw = tag.as_str();
        let part = first_capture(&part_re, raw).unwrap_or_default();
        if part.contains("/worksheets/sheet") && deleted.contains(&part) && !kept.contains(&part) {
            continue;
        }
        inner.push_str(raw);
    }
    for b in bindings.iter().filter(|b| b.is_new) {
        inner.push_str(&format!(
            r#"<Override PartName="/{}" ContentType="{}"/>"#,
            b.target, WORKSHEET_CONTENT_TYPE
        ));
    }

    let block_re = crate::re_cache::cached_re!(r"(?s)<Types\b[^>]*>.*?</Types>")
        .ok_or_else(|| "types block regex".to_string())?;
    let Some(m) = block_re.find(&xml) else {
        return Ok(());
    };
    let mut out = String::with_capacity(xml.len() + inner.len());
    out.push_str(&xml[..m.start()]);
    out.push_str("<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">");
    out.push_str(&inner);
    out.push_str("</Types>");
    out.push_str(&xml[m.end()..]);
    entries[index].1 = out.into_bytes();
    Ok(())
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
