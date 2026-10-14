//! Worksheet layout write-back (#788, E3).
//!
//! Patches a worksheet XML document with the model's layout state: column
//! widths, row heights, hidden rows, merged cells and frozen panes. Every step
//! is best-effort — a structure that cannot be located leaves that part of the
//! document unchanged rather than erroring.

use crate::types::Worksheet;
use std::collections::BTreeMap;

/// Patches a worksheet XML document with the model's layout state.
pub fn patch_layout(xml: &str, ws: &Worksheet) -> String {
    let mut s = xml.to_string();
    s = patch_cols(&s, ws);
    s = patch_rows(&s, ws);
    s = patch_merges(&s, ws);
    s = patch_frozen(&s, ws);
    s
}

/// Rewrites the `<cols>` block (column widths and hidden flags).
fn patch_cols(xml: &str, ws: &Worksheet) -> String {
    // Union of columns that have a width and columns that are hidden.
    let mut targets: BTreeMap<u32, (Option<u32>, bool)> = BTreeMap::new();
    if let Some(widths) = ws.column_widths.as_ref() {
        for (col, width) in widths {
            targets.entry(*col).or_insert((None, false)).0 = Some(*width);
        }
    }
    if let Some(hidden) = ws.hidden_columns.as_ref() {
        for col in hidden {
            targets.entry(*col).or_insert((None, false)).1 = true;
        }
    }
    if targets.is_empty() {
        return xml.to_string();
    }
    let mut cols = String::from("<cols>");
    for (col, (width, hidden)) in targets {
        // Model columns are 0-based; OOXML `<col min>/<col max>` are 1-based.
        let col1 = col + 1;
        let width_attr = match width {
            Some(w) => format!(r#" width="{w}" customWidth="1""#),
            None => String::new(),
        };
        let hidden_attr = if hidden { r#" hidden="1""# } else { "" };
        cols.push_str(&format!(
            r#"<col min="{col1}" max="{col1}"{width_attr}{hidden_attr}/>"#
        ));
    }
    cols.push_str("</cols>");

    if let Some(re) = crate::re_cache::cached_re!(r"(?s)<cols\b[^>]*>.*?</cols>") {
        if re.is_match(xml) {
            return re.replace(xml, cols.as_str()).to_string();
        }
    }
    if let Some(pos) = xml.find("<sheetData") {
        let mut out = String::with_capacity(xml.len() + cols.len());
        out.push_str(&xml[..pos]);
        out.push_str(&cols);
        out.push_str(&xml[pos..]);
        return out;
    }
    xml.to_string()
}

/// Rewrites `<row>` elements with the model's heights and hidden flags.
fn patch_rows(xml: &str, ws: &Worksheet) -> String {
    let mut s = xml.to_string();
    let mut targets: BTreeMap<u32, (Option<u32>, bool)> = BTreeMap::new();
    if let Some(heights) = ws.row_heights.as_ref() {
        for (r, h) in heights {
            targets.entry(*r).or_insert((None, false)).0 = Some(*h);
        }
    }
    if let Some(hidden) = ws.hidden_rows.as_ref() {
        for r in hidden {
            targets.entry(*r).or_insert((None, false)).1 = true;
        }
    }
    for (row, (height, hidden)) in targets {
        s = patch_row(&s, row, height, hidden);
    }
    s
}

fn patch_row(xml: &str, row: u32, height: Option<u32>, hidden: bool) -> String {
    // Model rows are 0-based; OOXML `<row r>` is 1-based.
    let row1 = row + 1;
    let pattern = format!(
        r#"<row\b[^>]*\br="{row1}"[^>]*>|<row\b[^>]*\br="{row1}"[^>]*/>"#
    );
    let Some(re) = crate::re_cache::dynamic(&pattern) else {
        return xml.to_string();
    };
    let Some(m) = re.find(xml) else {
        return xml.to_string();
    };
    let replacement = rewrite_row_open(m.as_str(), height, hidden);
    let mut out = String::with_capacity(xml.len() + replacement.len());
    out.push_str(&xml[..m.start()]);
    out.push_str(&replacement);
    out.push_str(&xml[m.end()..]);
    out
}

fn rewrite_row_open(open: &str, height: Option<u32>, hidden: bool) -> String {
    let self_closing = open.ends_with("/>");
    let mut body = open
        .trim_end_matches('>')
        .trim_end_matches('/')
        .trim_end()
        .to_string();
    body = strip_attr(&body, "ht");
    body = strip_attr(&body, "customHeight");
    body = strip_attr(&body, "hidden");
    if let Some(h) = height {
        body.push_str(&format!(r#" ht="{h}" customHeight="1""#));
    }
    if hidden {
        body.push_str(r#" hidden="1""#);
    }
    if self_closing {
        format!("{body}/>")
    } else {
        format!("{body}>")
    }
}

/// Removes an attribute (e.g. `ht="30"`) from an opening tag.
fn strip_attr(tag: &str, name: &str) -> String {
    let pattern = format!(r#"\s{name}="[^"]*""#);
    let Some(re) = crate::re_cache::dynamic(&pattern) else {
        return tag.to_string();
    };
    re.replace_all(tag, "").to_string()
}

/// Rewrites the `<mergeCells>` block.
fn patch_merges(xml: &str, ws: &Worksheet) -> String {
    let Some(merges) = ws.merged_cells.as_ref() else {
        return xml.to_string();
    };
    if merges.is_empty() {
        return xml.to_string();
    }
    let mut block = format!(r#"<mergeCells count="{}">"#, merges.len());
    for m in merges {
        let start = format!(
            "{}{}",
            super::xlsx_write::get_col_letter(m.start_col + 1),
            m.start_row + 1
        );
        let end = format!(
            "{}{}",
            super::xlsx_write::get_col_letter(m.end_col + 1),
            m.end_row + 1
        );
        block.push_str(&format!(r#"<mergeCell ref="{start}:{end}"/"#));
    }
    block.push_str("</mergeCells>");

    if let Some(re) = crate::re_cache::cached_re!(r"(?s)<mergeCells\b[^>]*>.*?</mergeCells>") {
        if re.is_match(xml) {
            return re.replace(xml, block.as_str()).to_string();
        }
    }
    if let Some(pos) = xml.find("</sheetData>") {
        let insert = pos + "</sheetData>".len();
        let mut out = String::with_capacity(xml.len() + block.len());
        out.push_str(&xml[..insert]);
        out.push_str(&block);
        out.push_str(&xml[insert..]);
        return out;
    }
    xml.to_string()
}

/// Rewrites (or inserts) the frozen `<pane>` element.
fn patch_frozen(xml: &str, ws: &Worksheet) -> String {
    let frozen_rows = ws.frozen_rows.unwrap_or(0);
    let frozen_cols = ws.frozen_cols.unwrap_or(0);
    if frozen_rows == 0 && frozen_cols == 0 {
        return xml.to_string();
    }
    let top_left = format!(
        "{}{}",
        super::xlsx_write::get_col_letter(frozen_cols + 1),
        frozen_rows + 1
    );
    let active_pane = match (frozen_rows > 0, frozen_cols > 0) {
        (true, true) => "bottomRight",
        (true, false) => "bottomLeft",
        (false, true) => "topRight",
        (false, false) => "topLeft",
    };
    let mut pane = String::from("<pane");
    if frozen_cols > 0 {
        pane.push_str(&format!(r#" xSplit="{frozen_cols}""#));
    }
    if frozen_rows > 0 {
        pane.push_str(&format!(r#" ySplit="{frozen_rows}""#));
    }
    pane.push_str(&format!(
        r#" topLeftCell="{top_left}" activePane="{active_pane}" state="frozen"/>"#
    ));

    if let Some(re) = crate::re_cache::cached_re!(r#"<pane\b[^>]*/>"#) {
        if re.is_match(xml) {
            return re.replace(xml, pane.as_str()).to_string();
        }
    }
    if let Some(re) = crate::re_cache::cached_re!(r#"<sheetView\b[^>]*>"#) {
        if let Some(m) = re.find(xml) {
            let insert = m.end();
            let mut out = String::with_capacity(xml.len() + pane.len());
            out.push_str(&xml[..insert]);
            out.push_str(&pane);
            out.push_str(&xml[insert..]);
            return out;
        }
    }
    xml.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Worksheet;
    use std::collections::HashMap;

    #[test]
    fn cols_are_written_one_based() {
        let ws = Worksheet {
            column_widths: Some(HashMap::from([(0u32, 30u32)])),
            ..Worksheet::default()
        };
        let xml = "<worksheet><sheetData/></worksheet>";
        let out = patch_cols(xml, &ws);
        assert!(
            out.contains(r#"<col min="1" max="1" width="30""#),
            "got: {out}"
        );
    }

    #[test]
    fn rows_are_written_one_based() {
        let ws = Worksheet {
            row_heights: Some(HashMap::from([(0u32, 25u32)])),
            ..Worksheet::default()
        };
        let xml = r#"<worksheet><sheetData><row r="1"><c r="A1"/></row></sheetData></worksheet>"#;
        let out = patch_rows(xml, &ws);
        assert!(out.contains(r#"r="1""#), "got: {out}");
        assert!(out.contains(r#"ht="25""#), "got: {out}");
    }
}
