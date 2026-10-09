use super::*;

impl FileContentExtractor {
    pub async fn extract_text(file_path: &PathBuf, mime_type: &str) -> Result<String> {
        match mime_type {
            "text/csv" => {
                let path = file_path.clone();
                tokio::task::spawn_blocking(move || {
                    use crate::stream_processor::StreamProcessor;
                    let mut processor = crate::streaming::csv::CsvStreamProcessor;
                    processor.process_stream(&path).map_err(|e| anyhow::anyhow!(e))
                })
                .await?
            }

            "text/plain" | "text/markdown" => {
                let path = file_path.clone();
                tokio::task::spawn_blocking(move || {
                    crate::stream_processor::read_file_streaming(&path).map_err(|e| anyhow::anyhow!(e))
                })
                .await?
            }

            t if t.starts_with("text/") => {
                let path = file_path.clone();
                tokio::task::spawn_blocking(move || {
                    crate::stream_processor::read_file_streaming(&path).map_err(|e| anyhow::anyhow!(e))
                })
                .await?
            }

            "application/pdf" => {
                log::info!("PDF extraction for {}", file_path.display());
                #[cfg(feature = "drive")]
                {
                    let path = file_path.clone();
                    tokio::task::spawn_blocking(move || {
                        use crate::stream_processor::StreamProcessor;
                        let mut processor = crate::streaming::pdf::PdfStreamProcessor;
                        processor.process_stream(&path).map_err(|e| anyhow::anyhow!(e))
                    })
                    .await?
                }
                #[cfg(not(feature = "drive"))]
                {
                    Err(anyhow::anyhow!("PDF extraction requires 'drive' feature"))
                }
            }

            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            | "application/msword" => {
                log::info!("Word document extraction for {}", file_path.display());
                let path = file_path.clone();
                tokio::task::spawn_blocking(move || {
                    use crate::stream_processor::StreamProcessor;
                    let mut processor = crate::streaming::docx::DocxStreamProcessor;
                    processor.process_stream(&path).map_err(|e| anyhow::anyhow!(e))
                })
                .await?
            }

            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            | "application/vnd.ms-excel" => {
                log::info!("Spreadsheet extraction for {}", file_path.display());
                #[cfg(feature = "sheet")]
                {
                    let path = file_path.clone();
                    tokio::task::spawn_blocking(move || {
                        use crate::stream_processor::StreamProcessor;
                        let mut processor = crate::streaming::xlsx::XlsxStreamProcessor;
                        processor.process_stream(&path).map_err(|e| anyhow::anyhow!(e))
                    })
                    .await?
                }
                #[cfg(not(feature = "sheet"))]
                {
                    log::warn!("XLSX extraction requires 'sheet' feature");
                    Ok(String::new())
                }
            }

            "application/json" => {
                let path = file_path.clone();
                let content = tokio::task::spawn_blocking(move || {
                    crate::stream_processor::read_file_streaming(&path).map_err(|e| anyhow::anyhow!(e))
                })
                .await??;

                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(json) => Ok(serde_json::to_string_pretty(&json)?),
                    Err(_) => Ok(content),
                }
            }

            "text/xml" | "application/xml" | "text/html" => {
                let path = file_path.clone();
                let content = tokio::task::spawn_blocking(move || {
                    crate::stream_processor::read_file_streaming(&path).map_err(|e| anyhow::anyhow!(e))
                })
                .await??;

                let tag_regex = regex::Regex::new(r"<[^>]+>").map_err(|e| anyhow::anyhow!("Invalid regex: {}", e))?;
                let text = tag_regex.replace_all(&content, " ").to_string();
                Ok(text.trim().to_string())
            }

            "text/rtf" | "application/rtf" => {
                let path = file_path.clone();
                let content = tokio::task::spawn_blocking(move || {
                    crate::stream_processor::read_file_streaming(&path).map_err(|e| anyhow::anyhow!(e))
                })
                .await??;

                let control_regex = regex::Regex::new(r"\\[a-z]+[\-0-9]*[ ]?").map_err(|e| anyhow::anyhow!("Invalid regex: {}", e))?;
                let group_regex = regex::Regex::new(r"[\{\}]").map_err(|e| anyhow::anyhow!("Invalid regex: {}", e))?;

                let mut text = control_regex.replace_all(&content, " ").to_string();
                text = group_regex.replace_all(&text, "").to_string();

                Ok(text.trim().to_string())
            }

            _ => {
                log::warn!("Unsupported file type for indexing: {}", mime_type);
                Ok(String::new())
            }
        }
}

#[allow(dead_code)]
#[cfg(feature = "drive")]
pub(super) async fn extract_pdf_text(file_path: &PathBuf) -> Result<String> {
    let bytes = fs::read(file_path).await?;

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

#[allow(dead_code)]
pub(super) async fn extract_docx_text(file_path: &Path) -> Result<String> {
    let path = file_path.to_path_buf();

    let result = tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&path)?;
        let mut archive = zip::ZipArchive::new(file)?;

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
                let n = reader.read(&mut byte_buf)?;
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

        Ok::<String, anyhow::Error>(content)
    })
    .await?;

    match result {
        Ok(text) => Ok(text),
        Err(e) => {
            log::warn!("DOCX extraction failed for {}: {}", file_path.display(), e);
            Ok(String::new())
        }
    }
}

#[cfg(feature = "sheet")]
pub(super) async fn extract_xlsx_text(file_path: &Path) -> Result<String> {
    let path = file_path.to_path_buf();

    let result = tokio::task::spawn_blocking(move || {
        let mut workbook: calamine::Xlsx<_> = calamine::open_workbook(&path)?;
        let mut content = String::new();

        for sheet_name in workbook.sheet_names() {
            if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                use std::fmt::Write;
                let _ = writeln!(&mut content, "=== {} ===", sheet_name);

                for row in range.rows() {
                    let row_text: Vec<String> = row
                        .iter()
                        .map(|cell| match cell {
                            calamine::Data::Empty => String::new(),
                            calamine::Data::String(s)
                            | calamine::Data::DateTimeIso(s)
                            | calamine::Data::DurationIso(s) => {
                                // Remove HTML tags and formatting artifacts
                                sanitize_cell_text(s)
                            },
                            calamine::Data::Float(f) => f.to_string(),
                            calamine::Data::Int(i) => i.to_string(),
                            calamine::Data::Bool(b) => b.to_string(),
                            calamine::Data::Error(e) => format!("{e:?}"),
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

        Ok::<String, anyhow::Error>(content)
    })
    .await?;

    match result {
        Ok(text) => Ok(text),
        Err(e) => {
            log::warn!("XLSX extraction failed for {}: {}", file_path.display(), e);
            Ok(String::new())
        }
    }
}

pub fn should_index(mime_type: &str, file_size: u64) -> bool {
    if file_size > 10 * 1024 * 1024 {
        return false;
    }

    matches!(
        mime_type,
        "text/plain"
            | "text/markdown"
            | "text/csv"
            | "text/html"
            | "application/json"
            | "text/x-python"
            | "text/x-rust"
            | "text/javascript"
            | "text/x-java"
    )
}

}

// --- Sync extraction functions (shared with streaming module) ---
