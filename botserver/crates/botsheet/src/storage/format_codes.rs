use std::collections::HashMap;
use std::io::Read;

/// Returns built-in number format codes (ISO 29500 / OOXML part 1 §18.8.30).
fn builtin_format_codes() -> HashMap<u32, String> {
    let mut m = HashMap::new();
    m.insert(0, "General".into());
    m.insert(1, "0".into());
    m.insert(2, "0.00".into());
    m.insert(3, "#,##0".into());
    m.insert(4, "#,##0.00".into());
    m.insert(5, "$#,##0_);($#,##0)".into());
    m.insert(6, "$#,##0_);[Red]($#,##0)".into());
    m.insert(7, "$#,##0.00_);($#,##0.00)".into());
    m.insert(8, "$#,##0.00_);[Red]($#,##0.00)".into());
    m.insert(9, "0%".into());
    m.insert(10, "0.00%".into());
    m.insert(11, "0.00E+00".into());
    m.insert(12, "# ?/?".into());
    m.insert(13, "# ??/??".into());
    m.insert(14, "m/d/yyyy".into());
    m.insert(15, "d-mmm-yy".into());
    m.insert(16, "d-mmm".into());
    m.insert(17, "mmm-yy".into());
    m.insert(18, "h:mm AM/PM".into());
    m.insert(19, "h:mm:ss AM/PM".into());
    m.insert(20, "h:mm".into());
    m.insert(21, "h:mm:ss".into());
    m.insert(22, "m/d/yyyy h:mm".into());
    m.insert(37, "#,##0_);(#,##0)".into());
    m.insert(38, "#,##0_);[Red](#,##0)".into());
    m.insert(39, "#,##0.00_);(#,##0.00)".into());
    m.insert(40, "#,##0.00_);[Red](#,##0.00)".into());
    m.insert(45, "mm:ss".into());
    m.insert(46, "[h]:mm:ss".into());
    m.insert(47, "mm:ss.0".into());
    m.insert(48, "##0.0E+0".into());
    m.insert(49, "@".into());
    m
}

/// "A" → 0, "Z" → 25, "AA" → 26, "AZ" → 51, "BA" → 52
fn col_to_index(col: &str) -> u32 {
    col.chars()
        .fold(0u32, |acc, c| acc * 26 + (c.to_ascii_uppercase() as u32 - b'A' as u32 + 1))
        .saturating_sub(1)
}

/// Extracts per-cell format codes from an xlsx file.
///
/// Returns a map: `sheet_index -> HashMap<"row,col" -> format_code_string>`.
/// Only cells with a non-General, non-empty format code are included.
pub fn extract_cell_format_codes(bytes: &[u8]) -> Result<HashMap<u32, HashMap<String, String>>, String> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| format!("Failed to open xlsx ZIP: {e}"))?;

    let mut styles_content = String::new();
    match archive.by_name("xl/styles.xml") {
        Ok(mut f) => { f.read_to_string(&mut styles_content).map_err(|e| format!("Failed to read styles.xml: {e}"))?; }
        Err(_) => return Ok(HashMap::new()),
    }

    // ── Step 1: Build numFmtId → formatCode map ──
    let mut fmt_id_to_code = builtin_format_codes();

    // <numFmt numFmtId="164" formatCode="$#,##0.00"/>
    let re_nf = crate::re_cache::cached_re!(r#"numFmtId="(\d+)"\s+formatCode="([^"]*)""#)
        .ok_or_else(|| "Regex error".to_string())?;
    for cap in re_nf.captures_iter(&styles_content) {
        if let Ok(id) = cap[1].parse::<u32>() {
            fmt_id_to_code.insert(id, cap[2].to_string());
        }
    }
    // <numFmt formatCode="$#,##0.00" numFmtId="164"/>
    let re_nf_rev = crate::re_cache::cached_re!(r#"formatCode="([^"]*)"\s+numFmtId="(\d+)""#)
        .ok_or_else(|| "Regex error".to_string())?;
    for cap in re_nf_rev.captures_iter(&styles_content) {
        if let Ok(id) = cap[2].parse::<u32>() {
            fmt_id_to_code.insert(id, cap[1].to_string());
        }
    }

    // ── Step 2: Build cellXf index → numFmtId mapping ──
    // Each <xf> tag within <cellXfs> is indexed by position (0-based).
    let mut xf_to_fmtid: Vec<Option<u32>> = Vec::new();

    if let Some(xfs_pos) = styles_content.find("<cellXfs") {
        if let Some(end_offset) = styles_content[xfs_pos..].find("</cellXfs>") {
            let xfs_end = xfs_pos + end_offset;
            let xfs_section = &styles_content[xfs_pos..xfs_end];

            let mut pos = 0usize;
            let re_nfid = crate::re_cache::cached_re!(r#"numFmtId="(\d+)""#)
                .ok_or_else(|| "Regex error".to_string())?;

            while let Some(xf_start) = xfs_section[pos..].find("<xf") {
                let tag_begin = pos + xf_start;
                let tail = &xfs_section[tag_begin..];
                let tag_len = tail.find("/>").or_else(|| tail.find('>'))
                    .map(|e| e + 2).unwrap_or(0);
                let tag = &tail[..tag_len.min(tail.len())];

                let fmt_id = re_nfid.captures(tag)
                    .and_then(|c| c[1].parse::<u32>().ok());

                xf_to_fmtid.push(fmt_id);

                if tag_len == 0 {
                    break;
                }
                pos = tag_begin + tag_len;
            }
        }
    }

    // ── Step 3: For each worksheet, extract per-cell style indices and resolve format codes ──
    let mut result: HashMap<u32, HashMap<String, String>> = HashMap::new();

    // Collect worksheet XML paths in order
    let sheet_name_re = crate::re_cache::cached_re!(r#"xl/worksheets/sheet(\d+)\.xml$"#)
        .ok_or_else(|| "Regex error".to_string())?;
    let mut sheet_files: Vec<(u32, String)> = Vec::new();
    for i in 0..archive.len() {
        let name = archive.name_for_index(i).unwrap_or("").to_string();
        if sheet_name_re.is_match(&name) {
            if let Some(caps) = sheet_name_re.captures(&name) {
                if let Ok(num) = caps[1].parse::<u32>() {
                    sheet_files.push((num, name));
                }
            }
        }
    }
    sheet_files.sort_by_key(|(num, _)| *num);

    let re_cell = crate::re_cache::cached_re!(r#"<c\s+[^>]*r="([A-Z]+)(\d+)"[^>]*>"#)
        .ok_or_else(|| "Regex error".to_string())?;
    // The s attribute may be at any position within the <c> tag
    let re_s_attr = crate::re_cache::cached_re!(r#"s="(\d+)""#)
        .ok_or_else(|| "Regex error".to_string())?;

    for (_sheet_num, sheet_path) in &sheet_files {
        let _ = _sheet_num;
        let mut sheet_xml = String::new();
        match archive.by_name(sheet_path) {
            Ok(mut f) => { f.read_to_string(&mut sheet_xml).map_err(|e| format!("Failed to read {sheet_path}: {e}"))?; }
            Err(_) => continue,
        }

        let mut cell_formats: HashMap<String, String> = HashMap::new();

        for cap in re_cell.captures_iter(&sheet_xml) {
            let col_idx = col_to_index(&cap[1]);
            let row_idx: u32 = cap[2].parse::<u32>().unwrap_or(1).saturating_sub(1);
            let key = format!("{row_idx},{col_idx}");

            // Extract s attribute from the full <c...> match
            let full_tag = cap.get(0).map(|m| m.as_str()).unwrap_or("");
            let xf_idx: Option<usize> = re_s_attr.captures(full_tag)
                .and_then(|c| c[1].parse::<usize>().ok());

            if let Some(xfi) = xf_idx {
                let num_fmt_id = xf_to_fmtid.get(xfi).copied().flatten();
                if let Some(fid) = num_fmt_id {
                    if let Some(fmt_code) = fmt_id_to_code.get(&fid) {
                        if fmt_code != "General" && !fmt_code.is_empty() {
                            cell_formats.insert(key, fmt_code.to_string());
                        }
                    }
                }
            }
        }

        if !cell_formats.is_empty() {
            let sheet_idx = result.len() as u32;
            result.insert(sheet_idx, cell_formats);
        }
    }

    Ok(result)
}

/// Merges extracted format codes into worksheet data cells.
///
/// Iterates over worksheets and applies format codes from the format map.
pub fn apply_format_codes(
    worksheets: &mut [crate::types::Worksheet],
    format_map: &HashMap<u32, HashMap<String, String>>,
) {
    for (ws_idx, ws) in worksheets.iter_mut().enumerate() {
        let sheet_formats = match format_map.get(&(ws_idx as u32)) {
            Some(m) => m,
            None => continue,
        };

        for (key, cell) in ws.data.iter_mut() {
            if let Some(fmt_code) = sheet_formats.get(key) {
                if cell.format.is_none() {
                    cell.format = Some(fmt_code.to_string());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_col_to_index() {
        assert_eq!(col_to_index("A"), 0);
        assert_eq!(col_to_index("Z"), 25);
        assert_eq!(col_to_index("AA"), 26);
        assert_eq!(col_to_index("AZ"), 51);
        assert_eq!(col_to_index("BA"), 52);
    }

    #[test]
    fn test_builtin_format_codes() {
        let codes = builtin_format_codes();
        assert_eq!(codes.get(&0).map(|s| s.as_str()), Some("General"));
        assert_eq!(codes.get(&14).map(|s| s.as_str()), Some("m/d/yyyy"));
        assert_eq!(codes.get(&9).map(|s| s.as_str()), Some("0%"));
        assert_eq!(codes.get(&49).map(|s| s.as_str()), Some("@"));
    }
}
