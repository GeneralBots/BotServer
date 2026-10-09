use super::*;

impl DocumentProcessor {
    pub fn new(chunk_size: usize, chunk_overlap: usize) -> Self {
        Self {
            chunk_size,
            chunk_overlap,
        }
    }

    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    pub fn chunk_overlap(&self) -> usize {
        self.chunk_overlap
    }

    pub fn is_supported_file(&self, path: &Path) -> bool {
        DocumentFormat::from_extension(path).is_some()
    }

    pub async fn process_document(&self, file_path: &Path) -> Result<Vec<TextChunk>> {
        if !file_path.exists() {
            return Err(anyhow::anyhow!("File not found: {}", file_path.display()));
        }

        let metadata = tokio::fs::metadata(file_path).await?;
        let file_size = metadata.len() as usize;

        if file_size == 0 {
            debug!("Skipping empty file (0 bytes): {}", file_path.display());
            return Ok(Vec::new());
        }

        let format = DocumentFormat::from_extension(file_path)
            .ok_or_else(|| anyhow::anyhow!("Unsupported file format: {}", file_path.display()))?;

        if file_size > format.max_size() {
            return Err(anyhow::anyhow!(
                "File too large: {} bytes (max: {} bytes)",
                file_size,
                format.max_size()
            ));
        }

        info!(
            "Processing document: {} (format: {:?}, size: {} bytes)",
            file_path.display(),
            format,
            file_size
        );

        let text = self.extract_text(file_path, format).await?;
        let text = Self::reformat_pairs(&text);
        let cleaned_text = Self::clean_text(&text);
        let chunks = self.create_chunks(&cleaned_text, file_path);

        info!(
            "Created {} chunks from document: {}",
            chunks.len(),
            file_path.display()
        );

        Ok(chunks)
    }

    async fn extract_text(&self, file_path: &Path, format: DocumentFormat) -> Result<String> {
        let metadata = tokio::fs::metadata(file_path).await?;
        let file_size = metadata.len() as usize;

        if file_size > format.max_size() {
            return Err(anyhow::anyhow!(
                "File too large: {} bytes (max: {} bytes)",
                file_size,
                format.max_size()
            ));
        }

        match format {
            DocumentFormat::TXT | DocumentFormat::MD => {
                if file_size > 10 * 1024 * 1024 {
                    self.extract_large_text_file(file_path).await
                } else {
                    let mut file = tokio::fs::File::open(file_path).await?;
                    let mut contents = String::with_capacity(std::cmp::min(file_size, 1024 * 1024));
                    file.read_to_string(&mut contents).await?;
                    Ok(contents)
                }
            }
        DocumentFormat::PDF => self.extract_pdf_text(file_path).await,
        DocumentFormat::DOCX => self.extract_docx_text(file_path).await,
        DocumentFormat::DOC => self.extract_doc_text(file_path).await,
        DocumentFormat::PPTX => self.extract_pptx_text(file_path).await,
        DocumentFormat::XLSX => self.extract_xlsx_text(file_path).await,
        DocumentFormat::XLS => self.extract_xls_text(file_path).await,
        DocumentFormat::HTML => self.extract_html_text(file_path).await,
            DocumentFormat::CSV => self.extract_csv_text(file_path).await,
            DocumentFormat::JSON => self.extract_json_text(file_path).await,
            DocumentFormat::XML => self.extract_xml_text(file_path).await,
            DocumentFormat::RTF => self.extract_rtf_text(file_path).await,
        }
    }

    async fn extract_large_text_file(&self, file_path: &Path) -> Result<String> {
        use tokio::io::AsyncBufReadExt;

        let file = tokio::fs::File::open(file_path).await?;
        let reader = tokio::io::BufReader::new(file);
        let mut lines = reader.lines();
        let mut content = String::new();
        let mut line_count = 0;
        const MAX_LINES: usize = 100_000;

        while let Some(line) = lines.next_line().await? {
            if line_count >= MAX_LINES {
                warn!("Truncating large file at {} lines: {}", MAX_LINES, file_path.display());
                break;
            }
            content.push_str(&line);
            content.push('\n');
            line_count += 1;

            if line_count % 1000 == 0 {
                tokio::task::yield_now().await;
            }
        }

        Ok(content)
    }

    async fn extract_pdf_text(&self, file_path: &Path) -> Result<String> {
        let file_path_str = file_path.to_string_lossy().to_string();
        let cmd_result = SafeCommand::new("pdftotext")
            .and_then(|c| c.arg("-layout"))
            .and_then(|c| c.arg(&file_path_str))
            .and_then(|c| c.arg("-"));

        let output = match cmd_result {
            Ok(cmd) => cmd.execute_async().await,
            Err(e) => {
                warn!("Failed to build pdftotext command: {}", e);
                return self.extract_pdf_with_library(file_path);
            }
        };

        match output {
            Ok(output) if output.status.success() => {
                info!("Successfully extracted PDF with pdftotext: {}", file_path.display());
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            }
            _ => {
                warn!("pdftotext failed for {}, trying library extraction", file_path.display());
                self.extract_pdf_with_library(file_path)
            }
        }
    }

    pub(super) fn extract_pdf_with_library(&self, file_path: &Path) -> Result<String> {
        let _ = self;
        #[cfg(feature = "drive")]
        {
            use pdf_extract::extract_text;
            match extract_text(file_path) {
                Ok(text) => {
                    info!("Successfully extracted PDF with library: {}", file_path.display());
                    return Ok(text);
                }
                Err(e) => {
                    warn!("PDF library extraction failed: {}", e);
                }
            }
        }
        #[cfg(not(feature = "drive"))]
        let _ = file_path;
        Self::extract_pdf_basic_sync(file_path)
    }

    #[cfg(feature = "drive")]
    pub(super) fn extract_pdf_basic_sync(file_path: &Path) -> Result<String> {
        if let Ok(text) = pdf_extract::extract_text(file_path) {
            if !text.is_empty() {
                return Ok(text);
            }
        }
        Err(anyhow::anyhow!("Could not extract text from PDF"))
    }

    #[cfg(not(feature = "drive"))]
    pub(super) fn extract_pdf_basic_sync(_file_path: &Path) -> Result<String> {
        Err(anyhow::anyhow!("PDF extraction requires 'drive' feature"))
    }

    async fn extract_docx_text(&self, file_path: &Path) -> Result<String> {
        let bytes = tokio::fs::read(file_path).await?;
        let path_display = file_path.display().to_string();
        let result = tokio::task::spawn_blocking(move || -> Result<String> {
            match ooxml_extract::extract_docx_text_from_zip(&bytes) {
                Ok(text) if !text.trim().is_empty() => {
                    log::info!("Extracted DOCX text from ZIP: {path_display}");
                    return Ok(text);
                }
                Ok(_) => log::warn!("DOCX ZIP extraction returned empty text: {path_display}"),
                Err(e) => log::warn!("DOCX ZIP extraction failed for {path_display}: {e}"),
            }

            Err(anyhow::anyhow!("All DOCX extraction methods failed for {path_display}"))
        })
        .await??;

        Ok(result)
    }

    async fn extract_html_text(&self, file_path: &Path) -> Result<String> {
        let contents = tokio::fs::read_to_string(file_path).await?;
        let text = contents
            .split('<')
            .flat_map(|s| s.split('>').skip(1))
            .collect::<Vec<_>>()
            .join(" ");
        Ok(text)
    }

    async fn extract_csv_text(&self, file_path: &Path) -> Result<String> {
        let contents = tokio::fs::read_to_string(file_path).await?;
        let mut text = String::new();
        for line in contents.lines() {
            text.push_str(line);
            text.push('\n');
        }
        Ok(text)
    }

    async fn extract_json_text(&self, file_path: &Path) -> Result<String> {
        let contents = tokio::fs::read_to_string(file_path).await?;
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
            Ok(Self::extract_json_strings(&json))
        } else {
            Ok(contents)
        }
    }

    pub(super) fn extract_json_strings(value: &serde_json::Value) -> String {
        let mut result = String::new();
        match value {
            serde_json::Value::String(s) => {
                result.push_str(s);
                result.push(' ');
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    result.push_str(&Self::extract_json_strings(item));
                }
            }
            serde_json::Value::Object(map) => {
                for (_key, val) in map {
                    result.push_str(&Self::extract_json_strings(val));
                }
            }
            _ => {}
        }
        result
    }

    async fn extract_pptx_text(&self, file_path: &Path) -> Result<String> {
        let bytes = tokio::fs::read(file_path).await?;
        let path_display = file_path.display().to_string();
        let result = tokio::task::spawn_blocking(move || -> Result<String> {
            match ooxml_extract::extract_pptx_text_from_zip(&bytes) {
                Ok(text) if !text.trim().is_empty() => {
                    log::info!("Extracted PPTX text from ZIP: {path_display}");
                    return Ok(text);
                }
                Ok(_) => log::warn!("PPTX ZIP extraction returned empty text: {path_display}"),
                Err(e) => log::warn!("PPTX ZIP extraction failed for {path_display}: {e}"),
            }

            Err(anyhow::anyhow!("All PPTX extraction methods failed for {path_display}"))
        })
        .await??;

        Ok(result)
    }
}
