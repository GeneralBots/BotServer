use super::*;

impl DocumentProcessor {
    #[cfg(feature = "kb-extraction")]
    pub(super) fn strip_html_tags(s: &str) -> String {
        // #1368 — constant patterns are valid by construction; on the
        // impossible parse error, skip the step instead of panicking. Each
        // pattern is compiled once per process (#1493).
        let s1 = crate::re_cache::cached_re!(
            r"(?is)<(style)[^>]*>.*?</style>|<(script)[^>]*>.*?</script>"
        )
        .map(|re| re.replace_all(s, "").to_string())
        .unwrap_or_else(|| s.to_string());
        let s2 = crate::re_cache::cached_re!(r"<[^>]*>")
            .map(|re| re.replace_all(&s1, "").to_string())
            .unwrap_or(s1);
        let s3 = s2.replace("&nbsp;", " ")
                   .replace("&amp;", "&")
                   .replace("&lt;", "<")
                   .replace("&gt;", ">")
                   .replace("&quot;", "\"");
        crate::re_cache::cached_re!(r"\s+")
            .map(|re| re.replace_all(&s3, " ").to_string())
            .unwrap_or(s3)
            .trim()
            .to_string()
    }

    #[cfg(feature = "kb-extraction")]
    pub(crate) async fn extract_xlsx_text(&self, file_path: &Path) -> Result<String> {
        let path = file_path.to_path_buf();
        let result = tokio::task::spawn_blocking(move || -> Result<String> {
            use calamine::{open_workbook, Reader, Xlsx};
            use std::fmt::Write;

            let mut workbook: Xlsx<_> = open_workbook(&path)
                .map_err(|e| anyhow::anyhow!("Failed to open XLSX: {e}"))?;

            let mut content = String::new();
            for sheet_name in workbook.sheet_names() {
                if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                    let _ = writeln!(&mut content, "=== {} ===", sheet_name);

                    for row in range.rows() {
                        let row_text: Vec<String> = row
                            .iter()
                            .map(|cell| match cell {
                                calamine::Data::Empty => String::new(),
                                calamine::Data::String(s)
                                | calamine::Data::DateTimeIso(s)
                                | calamine::Data::DurationIso(s) => {
                                    Self::strip_html_tags(s)
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

            Ok(content)
        })
        .await??;

        if result.trim().is_empty() {
            warn!("XLSX extraction produced empty text: {}", file_path.display());
        } else {
            info!("Extracted XLSX with calamine library: {}", file_path.display());
        }

        Ok(result)
    }

    #[cfg(not(feature = "kb-extraction"))]
    pub(crate) async fn extract_xlsx_text(&self, file_path: &Path) -> Result<String> {
        self.fallback_text_extraction(file_path).await
    }

    #[cfg(feature = "kb-extraction")]
    pub(crate) async fn extract_xls_text(&self, file_path: &Path) -> Result<String> {
        let path = file_path.to_path_buf();
        let result = tokio::task::spawn_blocking(move || -> Result<String> {
            use calamine::{open_workbook, Reader, Xls};
            use std::fmt::Write;

            let mut workbook: Xls<_> = open_workbook(&path)
                .map_err(|e| anyhow::anyhow!("Failed to open XLS: {e}"))?;

            let mut content = String::new();
            for sheet_name in workbook.sheet_names() {
                if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                    let _ = writeln!(&mut content, "=== {} ===", sheet_name);

                    for row in range.rows() {
                        let row_text: Vec<String> = row
                            .iter()
                            .map(|cell| match cell {
                                calamine::Data::Empty => String::new(),
                                calamine::Data::String(s)
                                | calamine::Data::DateTimeIso(s)
                                | calamine::Data::DurationIso(s) => {
                                    Self::strip_html_tags(s)
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

            Ok(content)
        })
        .await??;

        if result.trim().is_empty() {
            warn!("XLS extraction produced empty text: {}", file_path.display());
        } else {
            info!("Extracted XLS with calamine library: {}", file_path.display());
        }

        Ok(result)
    }

    #[cfg(not(feature = "kb-extraction"))]
    pub(crate) async fn extract_xls_text(&self, file_path: &Path) -> Result<String> {
        self.fallback_text_extraction(file_path).await
    }

    #[cfg(feature = "kb-extraction")]
    pub(crate) async fn extract_doc_text(&self, file_path: &Path) -> Result<String> {
        let path = file_path.to_path_buf();
        let result = tokio::task::spawn_blocking(move || -> Result<String> {
            use std::io::Read;

            let mut file = std::fs::File::open(&path)
                .map_err(|e| anyhow::anyhow!("Failed to open DOC: {e}"))?;
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;

            let ole_reader = ole::Reader::new(std::io::Cursor::new(bytes.as_slice()))
                .map_err(|e| anyhow::anyhow!("Failed to parse OLE structure: {e}"))?;

            let mut word_doc_data: Option<Vec<u8>> = None;
            for entry in ole_reader.iterate() {
                let name = entry.name().to_lowercase();
                if name == "worddocument" {
                    if let Ok(mut slice) = ole_reader.get_entry_slice(entry) {
                        let mut stream_data = Vec::with_capacity(slice.len());
                        if slice.read_to_end(&mut stream_data).is_ok() {
                            word_doc_data = Some(stream_data);
                            break;
                        }
                    }
                }
            }

            if let Some(stream_data) = word_doc_data {
                let text = super::part5::extract_text_from_word_document(&stream_data);
                if !text.trim().is_empty() {
                    return Ok(text);
                }
            }

            let mut all_text = String::new();
            for entry in ole_reader.iterate() {
                let name = entry.name().to_lowercase();
                if name == "worddocument" || name == "1table" || name == "0table" || name.contains("compobj") {
                    continue;
                }
                if let Ok(mut slice) = ole_reader.get_entry_slice(entry) {
                    let mut stream_data = Vec::with_capacity(slice.len());
                    if slice.read_to_end(&mut stream_data).is_ok() {
                        let text = String::from_utf8_lossy(&stream_data);
                        let printable: String = text
                            .chars()
                            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '.' || *c == ',')
                            .collect();
                        if !printable.trim().is_empty() {
                            all_text.push_str(&printable);
                            all_text.push('\n');
                        }
                    }
                }
            }

            if all_text.trim().is_empty() {
                return Err(anyhow::anyhow!("Could not extract text from DOC file"));
            }

            Ok(all_text)
        })
        .await??;

        if result.trim().is_empty() {
            warn!("DOC extraction produced empty text: {}", file_path.display());
        } else {
            info!("Extracted DOC with OLE parser: {}", file_path.display());
        }

        Ok(result)
    }

    #[cfg(not(feature = "kb-extraction"))]
    pub(crate) async fn extract_doc_text(&self, file_path: &Path) -> Result<String> {
        self.fallback_text_extraction(file_path).await
    }

    pub(crate) async fn extract_xml_text(&self, file_path: &Path) -> Result<String> {
        let bytes = tokio::fs::read(file_path).await?;
        let result = tokio::task::spawn_blocking(move || -> Result<String> {
            use quick_xml::events::Event;
            use quick_xml::Reader;

            let mut reader = Reader::from_reader(bytes.as_slice());
            let mut text = String::new();
            let mut buf = Vec::new();

            loop {
                match reader.read_event_into(&mut buf) {
                    Ok(Event::Text(t)) => {
                        if let Ok(s) = t.unescape() {
                            let s = s.trim();
                            if !s.is_empty() {
                                if !text.is_empty() {
                                    text.push(' ');
                                }
                                text.push_str(s);
                            }
                        }
                    }
                    Ok(Event::Eof) => break,
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "XML parsing error at position {}: {e}",
                            reader.error_position()
                        ));
                    }
                    _ => {}
                }
                buf.clear();
            }

            Ok(text)
        })
        .await??;

        if result.trim().is_empty() {
            warn!("XML extraction produced empty text: {}", file_path.display());
            return self.fallback_text_extraction(file_path).await;
        }

        info!("Extracted XML with quick-xml: {}", file_path.display());
        Ok(result)
    }

    pub(crate) async fn extract_rtf_text(&self, file_path: &Path) -> Result<String> {
        let bytes = tokio::fs::read(file_path).await?;
        let result = tokio::task::spawn_blocking(move || -> Result<String> {
            let content = String::from_utf8_lossy(&bytes);
            let text = rtf::strip_rtf_commands(&content);
            Ok(text)
        })
        .await??;

        if result.trim().is_empty() {
            warn!("RTF extraction produced empty text: {}", file_path.display());
            return self.fallback_text_extraction(file_path).await;
        }

        info!("Extracted RTF text: {}", file_path.display());
        Ok(result)
    }
}
