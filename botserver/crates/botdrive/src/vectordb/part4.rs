use super::*;

/// The extraction patterns are constants and `extract_text` runs once per
/// indexed file, so they are compiled once per process instead of per call
/// (#1493).
const MARKUP_TAG_PATTERN: &str = r"<[^>]+>";
const RTF_CONTROL_PATTERN: &str = r"\\[a-z]+[\-0-9]*[ ]?";
const RTF_GROUP_PATTERN: &str = r"[\{\}]";

fn markup_tag_re() -> Option<&'static regex::Regex> {
    static RE: std::sync::OnceLock<Option<regex::Regex>> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(MARKUP_TAG_PATTERN)
            .inspect_err(|e| log::error!("markup tag pattern failed to compile: {e}"))
            .ok()
    })
    .as_ref()
}

fn rtf_control_re() -> Option<&'static regex::Regex> {
    static RE: std::sync::OnceLock<Option<regex::Regex>> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(RTF_CONTROL_PATTERN)
            .inspect_err(|e| log::error!("RTF control pattern failed to compile: {e}"))
            .ok()
    })
    .as_ref()
}

fn rtf_group_re() -> Option<&'static regex::Regex> {
    static RE: std::sync::OnceLock<Option<regex::Regex>> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(RTF_GROUP_PATTERN)
            .inspect_err(|e| log::error!("RTF group pattern failed to compile: {e}"))
            .ok()
    })
    .as_ref()
}

impl FileContentExtractor {
    pub async fn extract_text(file_path: &Path, mime_type: &str) -> Result<String> {
        match mime_type {
            "text/csv" => {
                let path = file_path.to_path_buf();
                tokio::task::spawn_blocking(move || {
                    use crate::stream_processor::StreamProcessor;
                    let mut processor = crate::streaming::csv::CsvStreamProcessor;
                    processor.process_stream(&path).map_err(|e| anyhow::anyhow!(e))
                })
                .await?
            }

            "text/plain" | "text/markdown" => {
                let path = file_path.to_path_buf();
                tokio::task::spawn_blocking(move || {
                    crate::stream_processor::read_file_streaming(&path).map_err(|e| anyhow::anyhow!(e))
                })
                .await?
            }

            t if t.starts_with("text/") => {
                let path = file_path.to_path_buf();
                tokio::task::spawn_blocking(move || {
                    crate::stream_processor::read_file_streaming(&path).map_err(|e| anyhow::anyhow!(e))
                })
                .await?
            }

            "application/pdf" => {
                log::info!("PDF extraction for {}", file_path.display());
                #[cfg(feature = "drive")]
                {
                    let path = file_path.to_path_buf();
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
                let path = file_path.to_path_buf();
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
                    let path = file_path.to_path_buf();
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
                let path = file_path.to_path_buf();
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
                let path = file_path.to_path_buf();
                let content = tokio::task::spawn_blocking(move || {
                    crate::stream_processor::read_file_streaming(&path).map_err(|e| anyhow::anyhow!(e))
                })
                .await??;

                let Some(tag_regex) = markup_tag_re() else {
                    return Err(anyhow::anyhow!("Invalid markup tag pattern"));
                };
                let text = tag_regex.replace_all(&content, " ").to_string();
                Ok(text.trim().to_string())
            }

            "text/rtf" | "application/rtf" => {
                let path = file_path.to_path_buf();
                let content = tokio::task::spawn_blocking(move || {
                    crate::stream_processor::read_file_streaming(&path).map_err(|e| anyhow::anyhow!(e))
                })
                .await??;

                let (Some(control_regex), Some(group_regex)) = (rtf_control_re(), rtf_group_re())
                else {
                    return Err(anyhow::anyhow!("Invalid RTF extraction pattern"));
                };

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
}

// --- Sync extraction functions (shared with streaming module) ---
