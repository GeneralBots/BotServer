//! XLSX chart extraction: the structured reader (umya-spreadsheet) drops
//! charts when loading a workbook, so this module walks the raw .xlsx
//! package (drawings + chart parts) and rebuilds ChartConfig objects so the
//! Sheets UI can render charts loaded from Drive files.
//!
//! Chart XML parsing lives in [`chart_parse`]; range resolution in
//! [`chart_ranges`].

use crate::types::{ChartConfig, ChartDataset, ChartOptions, ChartPosition, Worksheet};
use quick_xml::events::Event;
use std::collections::HashMap;
use std::io::Cursor;
use uuid::Uuid;

mod chart_parse;
mod chart_ranges;

use self::chart_parse::{parse_chart, ParsedChart};
use self::chart_ranges::{resolve_range_labels, resolve_range_values, resolve_single_cell};

const PALETTE: [&str; 8] = [
    "#3b82f6", "#ef4444", "#22c55e", "#eab308", "#a855f7", "#06b6d4", "#f97316", "#ec4899",
];

const DEFAULT_WIDTH: u32 = 300;
const DEFAULT_HEIGHT: u32 = 180;
const CELL_WIDTH_PX: u32 = 64;
const CELL_HEIGHT_PX: u32 = 20;

/// Extracts charts from an .xlsx package. Returns one entry per worksheet
/// (index-aligned with `worksheets`); entries are empty when the sheet has
/// no charts.
pub fn extract_charts(
    bytes: &[u8],
    worksheets: &[Worksheet],
) -> Result<Vec<Vec<ChartConfig>>, String> {
    let files = read_zip_entries(bytes)?;
    let mut result: Vec<Vec<ChartConfig>> = vec![Vec::new(); worksheets.len()];

    for (ws_index, _ws) in worksheets.iter().enumerate() {
        let sheet_num = ws_index + 1;
        let (Some(sheet_xml), Some(rels_xml)) = (
            files.get(&format!("xl/worksheets/sheet{sheet_num}.xml")),
            files.get(&format!("xl/worksheets/_rels/sheet{sheet_num}.xml.rels")),
        ) else {
            continue;
        };

        let sheet_rels = parse_rels(rels_xml);
        let Some(drawing_rid) = find_drawing_rid(sheet_xml) else {
            continue;
        };
        let Some(drawing_target) = sheet_rels.get(&drawing_rid) else {
            continue;
        };
        let Some(drawing_path) = normalize_path("xl/worksheets", drawing_target) else {
            continue;
        };
        let Some(drawing_xml) = files.get(&drawing_path) else {
            continue;
        };

        let drawing_rels_path = format!(
            "xl/drawings/_rels/{}.rels",
            drawing_path.rsplit('/').next().unwrap_or("")
        );
        let drawing_rels = files
            .get(&drawing_rels_path)
            .map(|r| parse_rels(r))
            .unwrap_or_default();

        let anchored = parse_drawing(drawing_xml, &drawing_rels);
        if anchored.is_empty() {
            continue;
        }

        let mut charts = Vec::new();
        for (chart_path, position) in anchored {
            let Some(chart_xml) = files.get(&chart_path) else {
                continue;
            };
            let parsed = parse_chart(chart_xml);
            charts.push(to_chart_config(&parsed, ws_index, worksheets, position));
        }
        if !charts.is_empty() {
            result[ws_index] = charts;
        }
    }

    Ok(result)
}

fn to_chart_config(
    parsed: &ParsedChart,
    ws_index: usize,
    worksheets: &[Worksheet],
    position: ChartPosition,
) -> ChartConfig {
    let mut datasets = Vec::new();
    for (i, series) in parsed.series.iter().enumerate() {
        let label = if series.name.contains('!') {
            resolve_single_cell(&series.name, ws_index, worksheets)
                .unwrap_or_default()
        } else {
            series.name.clone()
        };
        let data = resolve_range_values(&series.val_ref, ws_index, worksheets);
        datasets.push(ChartDataset {
            label,
            data,
            color: PALETTE[i % PALETTE.len()].to_string(),
            background_color: None,
        });
    }

    let mut labels = Vec::new();
    if let Some(first) = parsed.series.first() {
        labels = resolve_range_labels(&first.cat_ref, ws_index, worksheets);
    }

    ChartConfig {
        id: Uuid::new_v4().to_string(),
        chart_type: parsed.chart_type.clone(),
        title: parsed.title.clone(),
        data_range: parsed.series.first().map(|s| s.val_ref.clone()).unwrap_or_default(),
        label_range: parsed.series.first().map(|s| s.cat_ref.clone()).unwrap_or_default(),
        position,
        options: ChartOptions {
            show_legend: parsed.legend_pos.is_some(),
            show_grid: true,
            stacked: false,
            legend_position: parsed.legend_pos.clone(),
            x_axis_title: None,
            y_axis_title: None,
        },
        datasets,
        labels,
    }
}

fn read_zip_entries(bytes: &[u8]) -> Result<HashMap<String, Vec<u8>>, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| format!("Failed to open xlsx zip: {e}"))?;
    let mut files = HashMap::new();
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry {i}: {e}"))?;
        let name = entry.name().to_string();
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut data)
            .map_err(|e| format!("Failed to read zip entry {name}: {e}"))?;
        files.insert(name, data);
    }
    Ok(files)
}

fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|&c| c == b':').next().unwrap_or(name)
}

fn attr_value(attr: &quick_xml::events::attributes::Attribute) -> String {
    attr.unescape_value()
        .map(|v| v.into_owned())
        .unwrap_or_else(|_| String::from_utf8_lossy(&attr.value).into_owned())
}

/// Parses an OOXML `.rels` document into a map of relationship id -> target.
fn parse_rels(xml: &[u8]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut reader = quick_xml::Reader::from_reader(xml);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e))
                if local_name(e.name().as_ref()) == b"Relationship" =>
            {
                let mut id = String::new();
                let mut target = String::new();
                for attr in e.attributes().flatten() {
                    match local_name(attr.key.as_ref()) {
                        b"Id" => id = attr_value(&attr),
                        b"Target" => target = attr_value(&attr),
                        _ => {}
                    }
                }
                if !id.is_empty() && !target.is_empty() {
                    map.insert(id, target);
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    map
}

/// Finds the `r:id` of the `<drawing>` element in a sheet part.
fn find_drawing_rid(sheet_xml: &[u8]) -> Option<String> {
    let mut reader = quick_xml::Reader::from_reader(sheet_xml);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e))
                if local_name(e.name().as_ref()) == b"drawing" =>
            {
                for attr in e.attributes().flatten() {
                    let value = attr_value(&attr);
                    if value.starts_with("rId") {
                        return Some(value);
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    None
}

/// Normalizes a package-relative target against a base directory into a zip
/// entry path. Targets starting with `/` are absolute package paths and are
/// used as-is (openpyxl writes such targets, e.g. `/xl/charts/chart1.xml`).
fn normalize_path(base_dir: &str, target: &str) -> Option<String> {
    let mut parts: Vec<&str> = if let Some(rest) = target.strip_prefix('/') {
        rest.split('/').filter(|s| !s.is_empty()).collect()
    } else {
        base_dir.split('/').filter(|s| !s.is_empty()).collect()
    };
    if target.starts_with('/') {
        if parts.is_empty() {
            return None;
        }
        return Some(parts.join("/"));
    }
    for comp in target.split('/') {
        match comp {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            c => parts.push(c),
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

/// Extracts (chart file path, anchor position) pairs from a drawing part.
fn parse_drawing(xml: &[u8], rels: &HashMap<String, String>) -> Vec<(String, ChartPosition)> {
    struct Anchor {
        from_col: u32,
        from_row: u32,
        to_col: Option<u32>,
        to_row: Option<u32>,
        rid: Option<String>,
    }

    let mut out = Vec::new();
    let mut reader = quick_xml::Reader::from_reader(xml);
    let mut buf = Vec::new();
    let mut anchor: Option<Anchor> = None;
    let mut section: Option<bool> = None; // true = "from", false = "to"
    let mut in_coord = false; // currently inside <col>/<row> (not colOff/rowOff)
    let mut coord_is_col = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                match local_name(e.name().as_ref()) {
                    b"twoCellAnchor" | b"oneCellAnchor" => {
                        anchor = Some(Anchor {
                            from_col: 0,
                            from_row: 0,
                            to_col: None,
                            to_row: None,
                            rid: None,
                        });
                    }
                    b"from" => section = Some(true),
                    b"to" => section = Some(false),
                    b"col" if anchor.is_some() && section.is_some() => {
                        in_coord = true;
                        coord_is_col = true;
                    }
                    b"row" if anchor.is_some() && section.is_some() => {
                        in_coord = true;
                        coord_is_col = false;
                    }
                    b"colOff" | b"rowOff" => in_coord = false,
                    b"chart" => {
                        if let Some(ref mut a) = anchor {
                            for attr in e.attributes().flatten() {
                                let value = attr_value(&attr);
                                if value.starts_with("rId") {
                                    a.rid = Some(value);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref t)) => {
                if let (Some(ref mut a), Some(is_from)) = (anchor.as_mut(), section) {
                    if in_coord {
                        if let Ok(text) = t.unescape() {
                            let value = text.trim().parse::<u32>().unwrap_or(0);
                            if coord_is_col {
                                if is_from {
                                    a.from_col = value;
                                } else {
                                    a.to_col = Some(value);
                                }
                            } else if is_from {
                                a.from_row = value;
                            } else {
                                a.to_row = Some(value);
                            }
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                match local_name(e.name().as_ref()) {
                    b"from" => section = None,
                    b"to" => section = None,
                    b"col" | b"row" => in_coord = false,
                    b"twoCellAnchor" | b"oneCellAnchor" => {
                        if let Some(a) = anchor.take() {
                            if let Some(chart_target) = a.rid.as_ref().and_then(|r| rels.get(r)) {
                                if let Some(path) = normalize_path("xl/drawings", chart_target) {
                                    let width = match a.to_col {
                                        Some(to_col) if to_col > a.from_col => {
                                            (to_col - a.from_col) * CELL_WIDTH_PX
                                        }
                                        _ => DEFAULT_WIDTH,
                                    };
                                    let height = match a.to_row {
                                        Some(to_row) if to_row > a.from_row => {
                                            (to_row - a.from_row) * CELL_HEIGHT_PX
                                        }
                                        _ => DEFAULT_HEIGHT,
                                    };
                                    out.push((
                                        path,
                                        ChartPosition {
                                            row: a.from_row,
                                            col: a.from_col,
                                            width: width.max(120),
                                            height: height.max(90),
                                        },
                                    ));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    out
}

#[cfg(test)]
#[path = "chart_read/tests.rs"]
mod tests;
