// Everything below is behind `drive` or `sheet`; the glob carries `Result` and
// `Path`, so it must be gated too or it is unused in a default build.
#[cfg(any(feature = "drive", feature = "sheet"))]
use super::*;

// The glob does not reach `pub(super)` items of a sibling submodule, so the
// sheet-only helper needs its own import — without it `--features sheet` did
// not compile (the feature is never enabled by the workspace build).
#[cfg(feature = "sheet")]
use super::part1::sanitize_cell_text;

#[cfg(feature = "drive")]
pub fn extract_pdf_text_sync(file_path: &Path) -> Result<String> {
    let bytes = std::fs::read(file_path)
        .map_err(|e| anyhow::anyhow!("Failed to read PDF {}: {}", file_path.display(), e))?;
    match pdf_extract::extract_text_from_mem(&bytes) {
        Ok(text) => {
            let cleaned = text
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            Ok(cleaned)
        }
        Err(e) => {
            log::warn!("PDF extraction failed for {}: {}", file_path.display(), e);
            Ok(String::new())
        }
    }
}
#[cfg(feature = "drive")]
pub fn extract_docx_text_sync(file_path: &Path) -> Result<String> {
    let file = std::fs::File::open(file_path)
        .map_err(|e| anyhow::anyhow!("Failed to open DOCX {}: {}", file_path.display(), e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| anyhow::anyhow!("Failed to open DOCX as zip: {}", e))?;

    let mut content = String::new();
    if let Ok(document) = archive.by_name("word/document.xml") {
        let mut reader = std::io::BufReader::new(document);
        let mut in_tag = false;
        let mut current_tag = String::new();
        let mut in_text_tag = false;
        let mut text_buffer = String::new();

        let mut byte_buf = [0u8; 8192];
        loop {
            use std::io::Read;
            let n = reader.read(&mut byte_buf)
                .map_err(|e| anyhow::anyhow!("Failed to read XML chunk: {}", e))?;
            if n == 0 {
                break;
            }
            for &b in &byte_buf[..n] {
                let c = b as char;
                if c == '<' {
                    in_tag = true;
                    current_tag.clear();
                } else if c == '>' {
                    in_tag = false;
                    if current_tag.starts_with("w:t") {
                        in_text_tag = true;
                        text_buffer.clear();
                    } else if current_tag == "/w:t" {
                        in_text_tag = false;
                        content.push_str(&text_buffer);
                    } else if current_tag == "/w:p" {
                        content.push('\n');
                    }
                } else if in_tag {
                    current_tag.push(c);
                } else if in_text_tag {
                    text_buffer.push(c);
                }
            }
        }
    }

    Ok(content)
}
#[cfg(feature = "sheet")]
pub fn extract_xlsx_text_sync(file_path: &Path) -> Result<String> {
    use std::fmt::Write;

    let mut workbook: calamine::Xlsx<_> = calamine::open_workbook(file_path)
        .map_err(|e| anyhow::anyhow!("Failed to open XLSX {}: {}", file_path.display(), e))?;
    let mut content = String::new();

    for sheet_name in workbook.sheet_names() {
        if let Ok(range) = workbook.worksheet_range(&sheet_name) {
            let _ = writeln!(&mut content, "=== {} ===", sheet_name);

            for row in range.rows() {
                let row_text: Vec<String> = row
                    .iter()
                    .map(|cell| match cell {
                        calamine::Data::Empty => String::new(),
                        calamine::Data::String(s) | calamine::Data::DateTimeIso(s) | calamine::Data::DurationIso(s) => {
                            sanitize_cell_text(s)
                        }
                        calamine::Data::Float(f) => f.to_string(),
                        calamine::Data::Int(i) => i.to_string(),
                        calamine::Data::Bool(b) => b.to_string(),
                        calamine::Data::Error(e) => format!("{:?}", e),
                        calamine::Data::DateTime(dt) => dt.to_string(),
                    })
                    .collect();

                let line = row_text.join("\t");
                if !line.trim().is_empty() {
                    content.push_str(&line);
                    content.push('\n');
                }
            }
            content.push('\n');
        }
    }

    Ok(content)
}
