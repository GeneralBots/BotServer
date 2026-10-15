//! Chart range resolution (split from `chart_read`).
//!
//! Resolves a chart series reference (`Sheet1!$A$2:$A$5`) against the imported
//! worksheet model into the numeric values and string labels the renderer needs.

use crate::types::{CellData, Worksheet};

/// Resolves a chart range reference against the matching worksheet and
/// returns the cell strings in row-major order.
pub(crate) fn resolve_range_values(
    range: &str,
    default_ws: usize,
    worksheets: &[Worksheet],
) -> Vec<f64> {
    let Some((ws_idx, r0, _c0, r1, c1)) = resolve_ref(range, default_ws, worksheets) else {
        return Vec::new();
    };
    let mut values = Vec::new();
    for r in r0..=r1 {
        for c in _c0..=c1 {
            let key = format!("{r},{c}");
            if let Some(cell) = worksheets[ws_idx].data.get(&key) {
                if let Some(num) = parse_number(cell) {
                    values.push(num);
                }
            }
        }
    }
    values
}

pub(crate) fn resolve_range_labels(
    range: &str,
    default_ws: usize,
    worksheets: &[Worksheet],
) -> Vec<String> {
    let Some((ws_idx, r0, c0, r1, c1)) = resolve_ref(range, default_ws, worksheets) else {
        return Vec::new();
    };
    let mut labels = Vec::new();
    for r in r0..=r1 {
        for c in c0..=c1 {
            let key = format!("{r},{c}");
            if let Some(cell) = worksheets[ws_idx].data.get(&key) {
                labels.push(cell.value.clone().unwrap_or_default());
            }
        }
    }
    labels
}

pub(crate) fn resolve_single_cell(
    range: &str,
    default_ws: usize,
    worksheets: &[Worksheet],
) -> Option<String> {
    let (ws_idx, r0, _c0, _r1, _c1) = resolve_ref(range, default_ws, worksheets)?;
    let key = format!("{r0},{_c0}");
    worksheets[ws_idx].data.get(&key).and_then(|c| c.value.clone())
}

fn resolve_ref(
    range: &str,
    default_ws: usize,
    worksheets: &[Worksheet],
) -> Option<(usize, u32, u32, u32, u32)> {
    let (sheet_part, cell_part) = match range.split_once('!') {
        Some((s, c)) => (Some(s), c),
        None => (None, range),
    };
    let ws_idx = match sheet_part {
        Some(name) => {
            let name = name.trim_matches('\'');
            worksheets
                .iter()
                .position(|w| w.name.eq_ignore_ascii_case(name))
                .unwrap_or(default_ws)
        }
        None => default_ws,
    };
    let (r0, c0, r1, c1) = parse_range_bounds(cell_part)?;
    Some((ws_idx, r0, c0, r1, c1))
}

fn parse_range_bounds(range: &str) -> Option<(u32, u32, u32, u32)> {
    let cell_part = range.split('!').next_back().unwrap_or(range);
    let cell_part = cell_part.replace('$', "");
    let parts: Vec<&str> = cell_part.split(':').collect();
    if parts.len() > 2 {
        return None;
    }
    let (r1, c1) = parse_cell_ref(parts[0])?;
    let (r2, c2) = parts
        .get(1)
        .and_then(|p| parse_cell_ref(p))
        .unwrap_or((r1, c1));
    Some((r1.min(r2), c1.min(c2), r1.max(r2), c1.max(c2)))
}

fn parse_number(cell: &CellData) -> Option<f64> {
    if let Some(botsheet_core::engine::value::CellValue::Number(n)) = &cell.typed {
        return Some(*n);
    }
    cell.value
        .as_deref()
        .and_then(|v| v.trim().parse::<f64>().ok())
}

fn parse_cell_ref(cell_ref: &str) -> Option<(u32, u32)> {
    let mut col_str = String::new();
    let mut row_str = String::new();

    for c in cell_ref.chars() {
        if c.is_ascii_alphabetic() {
            col_str.push(c.to_ascii_uppercase());
        } else if c.is_ascii_digit() {
            row_str.push(c);
        }
    }

    if col_str.is_empty() || row_str.is_empty() {
        return None;
    }

    let col = col_str
        .chars()
        .fold(0u32, |acc, c| acc * 26 + (c as u32 - 'A' as u32 + 1));

    let row: u32 = row_str.parse().ok()?;

    Some((row.saturating_sub(1), col.saturating_sub(1)))
}
