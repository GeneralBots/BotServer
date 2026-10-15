//! Split from `drive_vectordb.rs` — 117 lines.

use super::*;

pub struct FileContentExtractor;

impl FileContentExtractor {
    pub async fn extract_text(file_path: &PathBuf, mime_type: &str) -> Result<String> {
        match mime_type {
            "text/plain" | "text/markdown" | "text/csv" => Ok(fs::read_to_string(file_path).await?),
            t if t.starts_with("text/") => Ok(fs::read_to_string(file_path).await?),
            "application/pdf" => {
                log::warn!("PDF extraction not available without 'drive' feature");
                Ok(String::new())
            }
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            | "application/msword" => {
                log::info!("Word document extraction for {}", file_path.display());
                Self::extract_docx_text(file_path).await
            }
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            | "application/vnd.ms-excel" => {
                log::warn!("XLSX extraction requires 'sheet' feature");
                Ok(String::new())
            }
            "application/json" => {
                let content = fs::read_to_string(file_path).await?;
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(json) => Ok(serde_json::to_string_pretty(&json)?),
                    Err(_) => Ok(content),
                }
            }
            "text/xml" | "application/xml" | "text/html" => {
                let content = fs::read_to_string(file_path).await?;
                let tag_regex =
                    cached_re!(r"<[^>]+>").ok_or_else(|| anyhow::anyhow!("Invalid tag regex"))?;
                Ok(tag_regex
                    .replace_all(&content, " ")
                    .to_string()
                    .trim()
                    .to_string())
            }
            _ => {
                log::warn!("Unsupported file type for indexing: {}", mime_type);
                Ok(String::new())
            }
        }
    }

    pub(crate) async fn extract_docx_text(file_path: &Path) -> Result<String> {
        let path = file_path.to_path_buf();
        let result = tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&path)?;
            let mut archive = zip::ZipArchive::new(file)?;
            let mut content = String::new();
            if let Ok(mut document) = archive.by_name("word/document.xml") {
                let mut xml_content = String::new();
                std::io::Read::read_to_string(&mut document, &mut xml_content)?;
                let text_regex = cached_re!(r"<w:t[^>]*>([^<]*)</w:t>")
                    .ok_or_else(|| anyhow::anyhow!("Invalid docx text regex"))?;
                content = text_regex
                    .captures_iter(&xml_content)
                    .filter_map(|c| c.get(1).map(|m| m.as_str()))
                    .collect::<Vec<_>>()
                    .join("");
                content = content.split("</w:p>").collect::<Vec<_>>().join("\n");
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

#[cfg(test)]
mod snippet_tests {
    use super::UserDriveVectorDB;

    /// The window edges come from a match position in the lowercased copy, so
    /// either edge can land inside an accented character (#1508).
    #[test]
    fn snippet_around_a_multibyte_match_snaps_to_char_boundaries() {
        let content = "relatório café açúcar mascavo";
        let snippet = UserDriveVectorDB::create_snippet(content, "café", 10);
        assert!(
            snippet.contains("café"),
            "the match must survive: {snippet}"
        );
        assert!(snippet.starts_with("..."), "{snippet}");
    }

    /// The no-match branch cut a byte budget straight out of the content.
    #[test]
    fn snippet_without_a_match_truncates_on_a_char_boundary() {
        let content = "relatório de açúcar";
        let snippet = UserDriveVectorDB::create_snippet(content, "ausente", 6);
        assert_eq!(snippet, "relat...");
    }
}
