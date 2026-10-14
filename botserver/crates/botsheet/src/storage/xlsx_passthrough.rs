//! Zip-level xlsx preserve-and-passthrough (#788).
//!
//! The umya-spreadsheet round-trip cannot preserve every part Excel writes
//! (notably pivot tables). This module instead opens the original `.xlsx` as a
//! zip, rewrites ONLY the worksheet cell data (`xl/worksheets/sheetN.xml`) and
//! the workbook's sheet list for the changes the model owns, and copies every
//! other entry byte-for-byte — charts, drawings, pivot tables, VML, macros,
//! custom XML, styles and shared strings all survive an edit round-trip
//! untouched. Worksheet rename/add/delete live in [`xlsx_workbook`].

use crate::types::{CellData, Spreadsheet, Worksheet};
use botsheet_core::engine::value::CellValue;
use std::io::{Cursor, Read, Write};

use super::xlsx_shared_strings::{rewrite_shared_strings, SharedStrings};

/// Rewrites the edited cells into the ORIGINAL xlsx package.
///
/// Preserve-and-passthrough: only `xl/worksheets/sheetN.xml` cell data and the
/// workbook sheet list (rename/add/delete) are rewritten; every other zip entry
/// is copied verbatim. This is what makes a pivot table (or chart, or image)
/// Sheet cannot render survive being edited around.
pub fn merge_into_original(
    original_bytes: &[u8],
    sheet: &Spreadsheet,
) -> Result<Vec<u8>, String> {
    // 1. Read every part of the original package, preserving order.
    let mut entries = read_zip_entries(original_bytes)?;

    // 2. Resolve which original worksheet part each model sheet binds to
    //    (name-first, positional fallback for renames; new sheets get a fresh
    //    part, deleted sheets are dropped).
    let resolution = super::xlsx_workbook::resolve_bindings(&entries, sheet)?;

    // Shared-string table: an edited text cell keeps the Excel-native `t="s"`
    // storage — reusing an existing entry, or appending a new one. A missing
    // or unparseable table simply falls back to inline strings (never an error,
    // never a corrupted package).
    let shared_xml = entry_text(&entries, "xl/sharedStrings.xml").ok();
    let mut shared = SharedStrings::from_xml(shared_xml.as_deref());

    // 3. Rewrite only the worksheet cell data; everything else stays verbatim.
    for (index, worksheet) in sheet.worksheets.iter().enumerate() {
        let binding = &resolution.bindings[index];
        let (xml, is_new) = if binding.is_new {
            (fresh_worksheet_xml(), true)
        } else {
            let Some(xml) = entries
                .iter()
                .find(|(n, _)| n == &binding.target)
                .map(|(_, d)| d.clone())
            else {
                continue;
            };
            (xml, false)
        };
        let cells = patch_worksheet_cells(&xml, worksheet, &mut shared)?;
        let text = String::from_utf8_lossy(&cells);
        let patched = super::xlsx_layout_patch::patch_layout(&text, worksheet).into_bytes();
        if is_new {
            entries.push((binding.target.clone(), patched));
        } else if let Some(entry) = entries.iter_mut().find(|(n, _)| n == &binding.target) {
            entry.1 = patched;
        }
    }

    // 4. Persist any strings appended during the patch back into the table.
    if !shared.appended.is_empty() {
        if let Some(original) = shared_xml.as_deref() {
            if let Ok(rewritten) = rewrite_shared_strings(original, &shared.appended, shared.unique)
            {
                if let Some(entry) = entries
                    .iter_mut()
                    .find(|(n, _)| n == "xl/sharedStrings.xml")
                {
                    entry.1 = rewritten.into_bytes();
                }
            } else {
                // Extremely defensive: if the table could not be rewritten, the
                // cells already reference indexes beyond its end. Invalidate
                // the append by falling back is not possible after the fact, so
                // refuse to write a corrupt package instead.
                return Err("sharedStrings.xml rewrite failed — aborting save".to_string());
            }
        }
    }

    // 5. Apply workbook-level changes: rename/add/delete sheets, rewiring
    //    workbook.xml, relationships and content-types.
    super::xlsx_workbook_patch::apply_workbook_changes(&mut entries, &resolution)?;

    // 6. Write the new package.
    write_zip(entries)
}

/// A minimal, namespace-correct worksheet part for a freshly added sheet.
fn fresh_worksheet_xml() -> Vec<u8> {
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheetData/></worksheet>"
        .as_bytes()
        .to_vec()
}

fn read_zip_entries(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes.to_vec()))
        .map_err(|e| format!("open original xlsx zip: {e}"))?;
    let mut entries = Vec::with_capacity(archive.len());
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("read zip entry {i}: {e}"))?;
        let name = entry.name().to_string();
        let mut data = Vec::new();
        entry
            .read_to_end(&mut data)
            .map_err(|e| format!("read zip entry {name}: {e}"))?;
        entries.push((name, data));
    }
    Ok(entries)
}

fn write_zip(entries: Vec<(String, Vec<u8>)>) -> Result<Vec<u8>, String> {
    let mut out = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut out);
        for (name, data) in entries {
            writer
                .start_file(&name, zip::write::SimpleFileOptions::default())
                .map_err(|e| format!("zip start {name}: {e}"))?;
            writer
                .write_all(&data)
                .map_err(|e| format!("zip write {name}: {e}"))?;
        }
        writer.finish().map_err(|e| format!("zip finish: {e}"))?;
    }
    Ok(out.into_inner())
}

fn entry_text(entries: &[(String, Vec<u8>)], name: &str) -> Result<String, String> {
    entries
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, d)| String::from_utf8_lossy(d).to_string())
        .ok_or_else(|| format!("missing {name}"))
}

/// Rewrites the cells owned by the model into a worksheet's XML.
fn patch_worksheet_cells(
    xml: &[u8],
    worksheet: &Worksheet,
    shared: &mut SharedStrings,
) -> Result<Vec<u8>, String> {
    let mut s = String::from_utf8_lossy(xml).to_string();
    for (key, cell) in &worksheet.data {
        let (row, col) = parse_cell_key(key)?;
        let reference = format!("{}{}", super::xlsx_write::get_col_letter(col + 1), row + 1);
        s = replace_or_insert_cell(&s, &reference, row + 1, cell, shared)?;
    }
    Ok(s.into_bytes())
}

fn parse_cell_key(key: &str) -> Result<(u32, u32), String> {
    let parts: Vec<&str> = key.split(',').collect();
    if parts.len() != 2 {
        return Err(format!("bad cell key {key}"));
    }
    let row = parts[0].parse::<u32>().map_err(|_| format!("bad row in {key}"))?;
    let col = parts[1].parse::<u32>().map_err(|_| format!("bad col in {key}"))?;
    Ok((row, col))
}

/// Builds the replacement `<c>` element for a cell, preserving the original
/// style index when one was found on the existing element.
fn build_cell_xml(
    reference: &str,
    cell: &CellData,
    style_attr: Option<&str>,
    shared: &mut SharedStrings,
    original: Option<&str>,
) -> String {
    let style = style_attr
        .filter(|s| !s.is_empty())
        .map(|s| format!(" {s}"))
        .unwrap_or_default();

    if let Some(formula) = &cell.formula {
        // When the formula is unchanged, keep the original `<c>` element
        // verbatim so shared/array/data-table formulas and their cached `<v>`
        // survive (the lossy model cannot re-emit those attributes).
        if let Some(original) = original {
            if let Some(reused) = reuse_formula_cell(original, formula) {
                return reused;
            }
        }
        let body = formula.strip_prefix('=').unwrap_or(formula);
        return format!(
            r#"<c r="{reference}"{style}><f>{}</f></c>"#,
            xml_escape(body)
        );
    }

    if let Some(typed) = &cell.typed {
        return match typed {
            CellValue::Number(n) => format!(r#"<c r="{reference}"{style}><v>{n}</v></c>"#),
            CellValue::Bool(b) => {
                let v = if *b { "1" } else { "0" };
                format!(r#"<c r="{reference}"{style} t="b"><v>{v}</v></c>"#)
            }
            CellValue::Text(s) => text_cell(reference, s, &style, shared),
            CellValue::Error(e) => format!(r#"<c r="{reference}"{style} t="e"><v>{}</v></c>"#, xml_escape(e)),
            CellValue::Empty => format!(r#"<c r="{reference}"{style}/>"#),
        };
    }

    if let Some(value) = &cell.value {
        if let Ok(n) = value.parse::<f64>() {
            return format!(r#"<c r="{reference}"{style}><v>{n}</v></c>"#);
        }
        if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false") {
            let v = if value.eq_ignore_ascii_case("true") { "1" } else { "0" };
            return format!(r#"<c r="{reference}"{style} t="b"><v>{v}</v></c>"#);
        }
        return text_cell(reference, value, &style, shared);
    }

    format!(r#"<c r="{reference}"{style}/>"#)
}

/// Writes a text cell using the Excel-native shared-string form (`t="s"`),
/// reusing an existing entry or appending a new one. Falls back to an inline
/// string when the package has no shared-string table.
fn text_cell(
    reference: &str,
    text: &str,
    style: &str,
    shared: &mut SharedStrings,
) -> String {
    let escaped = xml_escape(text);
    match shared.lookup_or_append(&escaped) {
        Some(idx) => format!(r#"<c r="{reference}"{style} t="s"><v>{idx}</v></c>"#),
        None => inline_str_cell(reference, text, style),
    }
}

fn inline_str_cell(reference: &str, text: &str, style: &str) -> String {
    format!(
        r#"<c r="{reference}"{style} t="inlineStr"><is><t xml:space="preserve">{}</t></is></c>"#,
        xml_escape(text)
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Decodes the entities `xml_escape` produces (and Excel's own), `&amp;` last.
fn xml_unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Returns the original cell element unchanged when its `<f>` body matches the
/// model's formula (so shared/array attributes and the cached value survive).
fn reuse_formula_cell(original: &str, formula: &str) -> Option<String> {
    let f_re = crate::re_cache::cached_re!(r"(?s)<f\b[^>]*>(.*?)</f>")?;
    let caps = f_re.captures(original)?;
    let old_body = caps.get(1).map(|m| m.as_str()).unwrap_or("");
    let expected = formula.strip_prefix('=').unwrap_or(formula);
    if xml_unescape(old_body) == expected {
        Some(original.to_string())
    } else {
        None
    }
}

/// Extracts the `s="..."` style attribute from an existing `<c>` element so a
/// replacement keeps the cell's original formatting.
fn extract_style_attr(cell_xml: &str) -> Option<String> {
    let open_end = cell_xml.find('>').unwrap_or(cell_xml.len());
    let open = &cell_xml[..open_end];
    let re = crate::re_cache::cached_re!(r#"\bs="([^"]*)""#)?;
    re.captures(open).map(|c| format!(r#"s="{}""#, &c[1]))
}

/// Replaces an existing `<c r="REF">` element or inserts a new one.
fn replace_or_insert_cell(
    xml: &str,
    reference: &str,
    row: u32,
    cell: &CellData,
    shared: &mut SharedStrings,
) -> Result<String, String> {
    let esc = regex::escape(reference);
    let pattern = format!(
        r#"(?s)<c\b[^>]*\br="{esc}"[^>]*?/>|<c\b[^>]*\br="{esc}"[^>]*>.*?</c>"#
    );
    let re = crate::re_cache::dynamic(&pattern).ok_or_else(|| "cell regex".to_string())?;

    if let Some(m) = re.find(xml) {
        let style_attr = extract_style_attr(m.as_str());
        let replacement = build_cell_xml(reference, cell, style_attr.as_deref(), shared, Some(m.as_str()));
        let mut out = String::with_capacity(xml.len() + replacement.len());
        out.push_str(&xml[..m.start()]);
        out.push_str(&replacement);
        out.push_str(&xml[m.end()..]);
        return Ok(out);
    }

    // No existing cell: insert into the matching row, else create the row.
    let replacement = build_cell_xml(reference, cell, None, shared, None);
    let row_pattern = format!(r#"<row\b[^>]*\br="{row}"[^>]*>"#);
    let re_row = crate::re_cache::dynamic(&row_pattern).ok_or_else(|| "row regex".to_string())?;
    if let Some(row_open) = re_row.find(xml) {
        if let Some(close) = xml[row_open.end()..].find("</row>") {
            let close_pos = row_open.end() + close;
            let mut out = String::with_capacity(xml.len() + replacement.len());
            out.push_str(&xml[..close_pos]);
            out.push_str(&replacement);
            out.push_str(&xml[close_pos..]);
            return Ok(out);
        }
    }

    // Empty (self-closing) sheetData: expand it in place.
    if xml.contains("<sheetData/>") {
        let expanded = format!("<sheetData><row r=\"{row}\">{replacement}</row></sheetData>");
        return Ok(xml.replacen("<sheetData/>", &expanded, 1));
    }

    let new_row = format!(r#"<row r="{row}">{replacement}</row>"#);
    if let Some(pos) = xml.find("</sheetData>") {
        let mut out = String::with_capacity(xml.len() + new_row.len());
        out.push_str(&xml[..pos]);
        out.push_str(&new_row);
        out.push_str(&xml[pos..]);
        return Ok(out);
    }

    Err("worksheet XML has no sheetData".to_string())
}
