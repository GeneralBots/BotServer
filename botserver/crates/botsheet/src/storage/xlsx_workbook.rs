//! Workbook-level round-trip (E4): worksheet rename, add and delete.
//!
//! The model keeps no stable per-sheet id, so reconciliation is name-first
//! (handles add/delete) with a positional fallback for renames. Patching the
//! package parts lives in [`xlsx_workbook_patch`].

use crate::types::{Spreadsheet, Worksheet};
use std::collections::HashMap;

const WORKSHEET_REL_SUFFIX: &str = "/worksheet";

#[derive(Clone)]
struct OrigSheet {
    name: String,
    sheet_id: String,
    rid: String,
    raw: String,
}

/// How a model worksheet binds to a worksheet part in the original package.
pub struct SheetBinding {
    pub name: String,
    /// Zip part path of the worksheet (existing, or to be created).
    pub target: String,
    /// True when this worksheet has no original part (a freshly added sheet).
    pub is_new: bool,
    pub(crate) rid: String,
    pub(crate) sheet_id: String,
    pub(crate) orig_name: String,
    pub(crate) sheet_state: Option<String>,
    /// Original `<sheet .../>` tag, reused verbatim (minus a rename) when set.
    pub(crate) orig_raw: Option<String>,
}

/// Result of reconciling the model against the original workbook's sheets.
pub struct WorkbookResolution {
    /// Bindings aligned to `sheet.worksheets` order.
    pub bindings: Vec<SheetBinding>,
    /// Worksheet parts of deleted sheets.
    pub deleted_targets: Vec<String>,
    /// True when the workbook has chart/dialog/macro sheets the model cannot
    /// represent — structural changes are skipped to avoid dropping them.
    pub(crate) preserve_structure: bool,
}

/// Resolves the model's worksheets against the original workbook (pure read).
pub fn resolve_bindings(
    entries: &[(String, Vec<u8>)],
    sheet: &Spreadsheet,
) -> Result<WorkbookResolution, String> {
    let workbook_xml = entry_text(entries, "xl/workbook.xml")
        .map_err(|_| "workbook missing xl/workbook.xml".to_string())?;
    let orig = parse_sheets(&workbook_xml)?;
    let rels_xml = entry_text(entries, "xl/_rels/workbook.xml.rels").unwrap_or_default();
    let rid_to_target = parse_worksheet_rels(&rels_xml);

    // Chart/dialog/macro sheets are not modelled; preserve the whole structure
    // and only patch cells for sheets matched by name (never drop them).
    let has_non_worksheet = !rels_xml.is_empty()
        && orig
            .iter()
            .any(|o| !o.rid.is_empty() && !rid_to_target.contains_key(&o.rid));
    if has_non_worksheet {
        return Ok(preserve_only_bindings(&orig, &rid_to_target, sheet));
    }

    let mut max_sheet_num = max_sheet_number(entries);
    let mut max_rid_num = max_rid_number(&rels_xml);
    let mut max_sheet_id = orig
        .iter()
        .filter_map(|s| s.sheet_id.parse::<u64>().ok())
        .max()
        .unwrap_or(0);

    let mut used = vec![false; orig.len()];
    let mut bindings: Vec<SheetBinding> = Vec::with_capacity(sheet.worksheets.len());

    // Pass 1 — exact name match (add/delete keep surviving names stable).
    for ws in &sheet.worksheets {
        if let Some(i) = orig
            .iter()
            .enumerate()
            .find(|(i, o)| !used[*i] && o.name == ws.name)
            .map(|(i, _)| i)
        {
            used[i] = true;
            bindings.push(bind_reused(
                &orig[i],
                &rid_to_target,
                &ws.name,
                ws.sheet_state.as_deref(),
                i,
            ));
        } else {
            bindings.push(SheetBinding::placeholder(ws));
        }
    }

    // Pass 2 — positional fallback for renames; leftover model sheets stay new.
    let mut unused: Vec<usize> = (0..orig.len()).filter(|i| !used[*i]).collect();
    for binding in bindings.iter_mut() {
        if !binding.is_new || !binding.target.is_empty() {
            continue;
        }
        if let Some(oi) = unused.first().copied() {
            unused.remove(0);
            let name = binding.name.clone();
            let sheet_state = binding.sheet_state.clone();
            *binding = bind_reused(&orig[oi], &rid_to_target, &name, sheet_state.as_deref(), oi);
            used[oi] = true;
        }
    }

    // Remaining model sheets are new: allocate a fresh part, rId and sheetId.
    for binding in bindings.iter_mut() {
        if binding.is_new && binding.target.is_empty() {
            max_sheet_num += 1;
            max_rid_num += 1;
            max_sheet_id += 1;
            binding.target = format!("xl/worksheets/sheet{max_sheet_num}.xml");
            binding.rid = format!("rId{max_rid_num}");
            binding.sheet_id = max_sheet_id.to_string();
        }
    }

    let deleted_targets: Vec<String> = orig
        .iter()
        .enumerate()
        .filter(|(i, _)| !used[*i])
        .filter_map(|(_, o)| rid_to_target.get(&o.rid).map(|t| part_from_target(t)))
        .collect();

    Ok(WorkbookResolution {
        bindings,
        deleted_targets,
        preserve_structure: false,
    })
}

/// Binds model sheets to originals by exact name only; unmatched sheets are
/// skipped and nothing is deleted, so unmodelled sheets survive untouched.
fn preserve_only_bindings(
    orig: &[OrigSheet],
    rid_to_target: &HashMap<String, String>,
    sheet: &Spreadsheet,
) -> WorkbookResolution {
    let mut bindings = Vec::with_capacity(sheet.worksheets.len());
    for ws in &sheet.worksheets {
        let hit = orig.iter().enumerate().find(|(_, o)| {
            (o.rid.is_empty() || rid_to_target.contains_key(&o.rid)) && o.name == ws.name
        });
        match hit {
            Some((i, o)) => {
                bindings.push(bind_reused(o, rid_to_target, &ws.name, ws.sheet_state.as_deref(), i))
            }
            None => bindings.push(SheetBinding::skip(&ws.name)),
        }
    }
    WorkbookResolution {
        bindings,
        deleted_targets: Vec::new(),
        preserve_structure: true,
    }
}

// === Parsing ===

fn parse_sheets(workbook_xml: &str) -> Result<Vec<OrigSheet>, String> {
    let sheet_re = crate::re_cache::cached_re!(r#"<sheet\b[^>]*/?>"#).ok_or_else(|| "sheet regex".to_string())?;
    let name_re = crate::re_cache::cached_re!(r#"name="([^"]*)""#).ok_or_else(|| "name regex".to_string())?;
    let sid_re = crate::re_cache::cached_re!(r#"sheetId="([^"]*)""#).ok_or_else(|| "sid regex".to_string())?;
    let rid_re = crate::re_cache::cached_re!(r#"r:id="([^"]*)""#).ok_or_else(|| "rid regex".to_string())?;

    Ok(sheet_re
        .find_iter(workbook_xml)
        .map(|tag| {
            let raw = tag.as_str().to_string();
            OrigSheet {
                name: first_capture(&name_re, &raw).unwrap_or_default(),
                sheet_id: first_capture(&sid_re, &raw).unwrap_or_default(),
                rid: first_capture(&rid_re, &raw).unwrap_or_default(),
                raw,
            }
        })
        .collect())
}

fn parse_worksheet_rels(rels_xml: &str) -> HashMap<String, String> {
    let Some(rel_re) = crate::re_cache::cached_re!(r#"<Relationship\b[^>]*/?>"#) else {
        return HashMap::new();
    };
    let Some(id_re) = crate::re_cache::cached_re!(r#"Id="([^"]*)""#) else {
        return HashMap::new();
    };
    let Some(type_re) = crate::re_cache::cached_re!(r#"Type="([^"]*)""#) else {
        return HashMap::new();
    };
    let Some(target_re) = crate::re_cache::cached_re!(r#"Target="([^"]*)""#) else {
        return HashMap::new();
    };

    let mut map = HashMap::new();
    for tag in rel_re.find_iter(rels_xml) {
        let raw = tag.as_str();
        if !first_capture(&type_re, raw)
            .unwrap_or_default()
            .ends_with(WORKSHEET_REL_SUFFIX)
        {
            continue;
        }
        if let (Some(id), Some(target)) = (first_capture(&id_re, raw), first_capture(&target_re, raw))
        {
            map.insert(id, target);
        }
    }
    map
}

// === Binding helpers ===

fn bind_reused(
    orig: &OrigSheet,
    rid_to_target: &HashMap<String, String>,
    name: &str,
    sheet_state: Option<&str>,
    index: usize,
) -> SheetBinding {
    let target = rid_to_target
        .get(&orig.rid)
        .map(|t| part_from_target(t))
        .unwrap_or_else(|| format!("xl/worksheets/sheet{}.xml", index + 1));
    SheetBinding {
        name: name.to_string(),
        target,
        is_new: false,
        rid: orig.rid.clone(),
        sheet_id: orig.sheet_id.clone(),
        orig_name: orig.name.clone(),
        sheet_state: sheet_state.map(|s| s.to_string()),
        orig_raw: Some(orig.raw.clone()),
    }
}

impl SheetBinding {
    fn placeholder(ws: &Worksheet) -> Self {
        SheetBinding {
            name: ws.name.clone(),
            target: String::new(),
            is_new: true,
            rid: String::new(),
            sheet_id: String::new(),
            orig_name: String::new(),
            sheet_state: ws.sheet_state.clone(),
            orig_raw: None,
        }
    }

    /// A binding with no part: the sheet is neither patched nor added.
    fn skip(name: &str) -> Self {
        SheetBinding {
            name: name.to_string(),
            target: String::new(),
            is_new: false,
            rid: String::new(),
            sheet_id: String::new(),
            orig_name: String::new(),
            sheet_state: None,
            orig_raw: None,
        }
    }
}

/// Normalizes a relationship `Target` (relative to `xl/`) into a zip part path.
pub(crate) fn part_from_target(target: &str) -> String {
    let trimmed = target.trim_start_matches('/');
    if trimmed.starts_with("xl/") {
        trimmed.to_string()
    } else {
        format!("xl/{trimmed}")
    }
}

fn max_sheet_number(entries: &[(String, Vec<u8>)]) -> u64 {
    let Some(re) = crate::re_cache::cached_re!(r"^xl/worksheets/sheet(\d+)\.xml$") else {
        return 0;
    };
    entries
        .iter()
        .filter_map(|(n, _)| re.captures(n).and_then(|c| c.get(1)))
        .filter_map(|m| m.as_str().parse::<u64>().ok())
        .max()
        .unwrap_or(0)
}

fn max_rid_number(rels_xml: &str) -> u64 {
    let Some(re) = crate::re_cache::cached_re!(r#"Id="rId(\d+)""#) else {
        return 0;
    };
    re.captures_iter(rels_xml)
        .filter_map(|c| c.get(1))
        .filter_map(|m| m.as_str().parse::<u64>().ok())
        .max()
        .unwrap_or(0)
}

// === Small helpers ===

fn entry_text(entries: &[(String, Vec<u8>)], name: &str) -> Result<String, String> {
    entries
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, d)| String::from_utf8_lossy(d).to_string())
        .ok_or_else(|| format!("missing {name}"))
}

pub(crate) fn first_capture(re: &regex::Regex, s: &str) -> Option<String> {
    re.captures(s).and_then(|c| c.get(1)).map(|m| m.as_str().to_string())
}
