use super::*;

impl DocumentProcessor {
    async fn fallback_text_extraction(&self, file_path: &Path) -> Result<String> {
        match tokio::fs::read_to_string(file_path).await {
            Ok(contents) => Ok(contents),
            Err(_) => {
                let bytes = tokio::fs::read(file_path).await?;
                Ok(String::from_utf8_lossy(&bytes).to_string())
            }
        }
    }

    /// Post-process extracted text to pair names with numbers when they appear
    /// on separate consecutive lines (common in PDF table extractions).
    pub(super) fn reformat_pairs(text: &str) -> String {
        let lines: Vec<&str> = text.lines().map(|l| l.trim()).collect();
        let mut result: Vec<String> = Vec::new();
        let mut name_buf: Vec<&str> = Vec::new();
        let mut num_buf: Vec<&str> = Vec::new();

        let is_number_line = |l: &str| -> bool {
            let cleaned = l.trim().replace([' ', '-', '*', '/', '.'], "");
            !cleaned.is_empty() && cleaned.chars().all(|c| c.is_ascii_digit())
        };

        let is_section_header = |l: &str| -> bool {
            l.chars().filter(|c| c.is_ascii_uppercase() || c.is_whitespace()).count() as f64 / l.len().max(1) as f64 > 0.6
                && l.len() > 3
                && l.chars().any(|c| c.is_ascii_uppercase())
                && !l.contains(':')
        };

        let flush = |names: &[&str], nums: &[&str], out: &mut Vec<String>| {
            if names.is_empty() && nums.is_empty() {
                return;
            }
            if names.is_empty() || nums.is_empty() {
                for l in names.iter().chain(nums.iter()) {
                    out.push(l.to_string());
                }
                return;
            }
            let count = names.len().max(nums.len());
            for i in 0..count {
                let name = names.get(i).unwrap_or(&"");
                let num = nums.get(i).unwrap_or(&"");
                if !name.is_empty() && !num.is_empty() {
                    out.push(format!("{}: {}", name, num));
                } else {
                    if !name.is_empty() { out.push(name.to_string()); }
                    if !num.is_empty() { out.push(num.to_string()); }
                }
            }
        };

        for line in &lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if is_section_header(trimmed) {
                flush(&name_buf, &num_buf, &mut result);
                name_buf.clear();
                num_buf.clear();
                result.push(trimmed.to_string());
                continue;
            }
            if is_number_line(trimmed) {
                if !name_buf.is_empty() {
                    num_buf.push(trimmed);
                } else {
                    result.push(trimmed.to_string());
                }
            } else {
                if !num_buf.is_empty() {
                    flush(&name_buf, &num_buf, &mut result);
                    name_buf.clear();
                    num_buf.clear();
                }
                name_buf.push(trimmed);
            }
        }
        flush(&name_buf, &num_buf, &mut result);

        result.join("\n")
    }

    pub(super) fn clean_text(text: &str) -> String {
        let cleaned = text
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        cleaned
            .chars()
            .filter(|c| !c.is_control() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
        .join(" ")
    }

    pub(super) fn create_chunks(&self, text: &str, file_path: &Path) -> Vec<TextChunk> {
        let chars: Vec<char> = text.chars().collect();
        let total_chars = chars.len();
        let mut start = 0;
        let mut chunk_index = 0;

        let step_size = self.chunk_size.saturating_sub(self.chunk_overlap);
        let total_chunks = if step_size > 0 {
            total_chars.div_ceil(step_size)
        } else {
            1
        };

        const MAX_CHUNKS: usize = 1000;
        let max_chunks_to_create = std::cmp::min(total_chunks, MAX_CHUNKS);
        let mut chunks = Vec::with_capacity(max_chunks_to_create);

        while start < total_chars && chunk_index < max_chunks_to_create {
            let end = std::cmp::min(start + self.chunk_size, total_chars);

            let mut chunk_end = end;
            if end < total_chars {
                let search_start = std::cmp::max(start, end.saturating_sub(200));
                // Prefer newline boundary to keep lines intact
                let mut found_newline = false;
                for i in (search_start..end).rev() {
                    if chars[i] == '\n' {
                        chunk_end = i + 1;
                        found_newline = true;
                        break;
                    }
                }
                if !found_newline {
                    // Fall back to any whitespace
                    for i in (search_start..end).rev() {
                        if chars[i].is_whitespace() {
                            chunk_end = i + 1;
                            break;
                        }
                    }
                }
            }

            let chunk_content: String = chars[start..chunk_end].iter().collect();

            if chunk_content.trim().len() < 10 {
                start = chunk_end;
                continue;
            }

            chunks.push(TextChunk {
                content: chunk_content,
                metadata: ChunkMetadata {
                    document_path: file_path.to_string_lossy().to_string(),
                    document_title: file_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string()),
                    chunk_index,
                    total_chunks: max_chunks_to_create,
                    start_char: start,
                    end_char: chunk_end,
                    page_number: None,
                },
            });

            chunk_index += 1;

            start = if chunk_end >= self.chunk_overlap {
                chunk_end - self.chunk_overlap
            } else {
                chunk_end
            };

            // Align to line boundary: advance to the next newline to avoid splitting lines
            if start < total_chars && start > 0 {
                // Find the next newline after the overlap-adjusted start
                if let Some(pos) = chars[start..std::cmp::min(start + 200, total_chars)]
                    .iter()
                    .position(|&c| c == '\n')
                {
                    start += pos + 1;
                }
            }

            if start >= total_chars {
                break;
            }
        }

        if chunk_index >= MAX_CHUNKS {
            warn!("Truncated chunking at {} chunks for: {}", MAX_CHUNKS, file_path.display());
        }

        chunks
    }

    pub async fn process_kb_folder(
        &self,
        kb_path: &Path,
    ) -> Result<HashMap<String, Vec<TextChunk>>> {
        if !kb_path.exists() {
            return Err(anyhow::anyhow!(
                "Knowledge base folder not found: {}",
                kb_path.display()
            ));
        }

        info!("Processing knowledge base folder: {}", kb_path.display());

        let mut results = HashMap::new();
        const BATCH_SIZE: usize = 10;

        let files = self.collect_supported_files(kb_path).await?;
        info!("Found {} supported files to process", files.len());

        for batch in files.chunks(BATCH_SIZE) {
            let mut batch_results = HashMap::new();

            for file_path in batch {
                match self.process_document(file_path).await {
                    Ok(chunks) => {
                        if !chunks.is_empty() {
                            batch_results.insert(file_path.to_string_lossy().to_string(), chunks);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to process document {}: {}", file_path.display(), e);
                    }
                }

                tokio::task::yield_now().await;
            }

            results.extend(batch_results);

            if results.len() % (BATCH_SIZE * 2) == 0 {
                results.shrink_to_fit();
            }

            info!("Processed batch, total documents: {}", results.len());
        }

        info!("Completed processing {} documents in knowledge base", results.len());
        Ok(results)
    }

    async fn collect_supported_files(&self, dir: &Path) -> Result<Vec<std::path::PathBuf>> {
        let mut files = Vec::new();
        self.collect_files_recursive(dir, &mut files, 0).await?;
        Ok(files)
    }

    async fn collect_files_recursive(
        &self,
        dir: &Path,
        files: &mut Vec<std::path::PathBuf>,
        depth: usize,
    ) -> Result<()> {
        if depth > 10 {
            warn!("Skipping deep directory to prevent stack overflow: {}", dir.display());
            return Ok(());
        }

        let mut entries = tokio::fs::read_dir(dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let metadata = entry.metadata().await?;

            if metadata.is_dir() {
                Box::pin(self.collect_files_recursive(&path, files, depth + 1)).await?;
            } else if self.is_supported_file(&path) {
                if metadata.len() > 50 * 1024 * 1024 {
                    warn!("Skipping large file: {} ({})", path.display(), metadata.len());
                    continue;
                }
                files.push(path);
            }
        }

        Ok(())
    }
}
