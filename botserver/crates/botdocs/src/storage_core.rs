use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::state::DocState;
use crate::storage_docx::convert_docx_to_html;
use crate::types::Document;

/// Document id → (bytes, last modification time) of the in-memory cache.
type DocumentCache = Lazy<RwLock<HashMap<String, (Vec<u8>, DateTime<Utc>)>>>;

static DOCUMENT_CACHE: DocumentCache = Lazy::new(|| RwLock::new(HashMap::new()));

const CACHE_TTL_SECS: i64 = 3600;

pub fn get_user_docs_path(user_identifier: &str) -> String {
    let safe_id = user_identifier
        .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
        .to_lowercase();
    format!("users/{safe_id}/docs")
}

pub fn get_current_user_id() -> String {
    "default-user".to_string()
}

pub fn generate_doc_id() -> String {
    Uuid::new_v4().to_string()
}

pub async fn cache_document_bytes(doc_id: &str, bytes: Vec<u8>) {
    let mut cache = DOCUMENT_CACHE.write().await;
    cache.insert(doc_id.to_string(), (bytes, Utc::now()));

    let now = Utc::now();
    cache.retain(|_, (_, modified)| (now - *modified).num_seconds() < CACHE_TTL_SECS);
}

pub async fn get_cached_document_bytes(doc_id: &str) -> Option<Vec<u8>> {
    let cache = DOCUMENT_CACHE.read().await;
    cache.get(doc_id).map(|(bytes, _)| bytes.clone())
}

pub async fn remove_from_cache(doc_id: &str) {
    let mut cache = DOCUMENT_CACHE.write().await;
    cache.remove(doc_id);
}

pub fn count_words(content: &str) -> usize {
    let plain_text = strip_html(content);
    plain_text
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .count()
}

pub fn create_new_document() -> Document {
    let doc_id = generate_doc_id();
    Document {
        id: doc_id,
        title: "Untitled Document".to_string(),
        content: "<p><br></p>".to_string(),
        owner_id: get_current_user_id(),
        storage_path: String::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        collaborators: Vec::new(),
        version: 1,
        track_changes: None,
        comments: None,
        footnotes: None,
        endnotes: None,
        styles: None,
        toc: None,
        track_changes_enabled: false,
    }
}

pub fn strip_html(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }

    result
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
}

pub fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn paragraphs_to_html(paragraphs: &[String]) -> String {
    paragraphs
        .iter()
        .map(|p| format!("<p>{}</p>", escape_html(p)))
        .collect::<Vec<_>>()
        .join("")
}

pub fn html_to_paragraphs(html: &str) -> Vec<String> {
    parse_html_to_paragraphs(html)
        .into_iter()
        .map(|p| p.text)
        .collect()
}

#[derive(Default, Clone)]
pub struct ParagraphData {
    pub text: String,
    pub style: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

pub fn parse_html_to_paragraphs(html: &str) -> Vec<ParagraphData> {
    let mut paragraphs = Vec::new();
    let mut current = ParagraphData::default();
    let mut in_tag = false;
    let mut tag_name = String::new();
    let mut is_closing = false;
    let mut text_buffer = String::new();

    let mut bold_stack: i32 = 0;
    let mut italic_stack: i32 = 0;
    let mut underline_stack: i32 = 0;

    for ch in html.chars() {
        match ch {
            '<' => {
                in_tag = true;
                tag_name.clear();
                is_closing = false;
            }
            '>' => {
                in_tag = false;
                let tag = tag_name.to_lowercase();
                let tag_trimmed = tag.split_whitespace().next().unwrap_or("");

                if is_closing {
                    match tag_trimmed {
                        "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "li"
                        | "blockquote" | "pre" => {
                            if !text_buffer.is_empty() || !current.text.is_empty() {
                                current.text = format!(
                                    "{}{}",
                                    current.text,
                                    decode_html_entities(&text_buffer)
                                );
                                if !current.text.trim().is_empty() {
                                    paragraphs.push(current);
                                }
                                current = ParagraphData::default();
                                text_buffer.clear();
                            }
                        }
                        "b" | "strong" => bold_stack = bold_stack.saturating_sub(1),
                        "i" | "em" => italic_stack = italic_stack.saturating_sub(1),
                        "u" => underline_stack = underline_stack.saturating_sub(1),
                        _ => {}
                    }
                } else {
                    match tag_trimmed {
                        "br" => {
                            text_buffer.push('\n');
                        }
                        "p" | "div" => {
                            if !text_buffer.is_empty() {
                                current.text = format!(
                                    "{}{}",
                                    current.text,
                                    decode_html_entities(&text_buffer)
                                );
                                text_buffer.clear();
                            }
                            current.style = "p".to_string();
                            current.bold = bold_stack > 0;
                            current.italic = italic_stack > 0;
                            current.underline = underline_stack > 0;
                        }
                        "h1" => current.style = "h1".to_string(),
                        "h2" => current.style = "h2".to_string(),
                        "h3" => current.style = "h3".to_string(),
                        "li" => current.style = "li".to_string(),
                        "blockquote" => current.style = "blockquote".to_string(),
                        "pre" | "code" => current.style = "code".to_string(),
                        "b" | "strong" => bold_stack += 1,
                        "i" | "em" => italic_stack += 1,
                        "u" => underline_stack += 1,
                        _ => {}
                    }
                }
                tag_name.clear();
            }
            '/' if in_tag && tag_name.is_empty() => {
                is_closing = true;
            }
            _ if in_tag => {
                tag_name.push(ch);
            }
            _ => {
                text_buffer.push(ch);
            }
        }
    }

    if !text_buffer.is_empty() {
        current.text = format!("{}{}", current.text, decode_html_entities(&text_buffer));
    }
    if !current.text.trim().is_empty() {
        paragraphs.push(current);
    }

    paragraphs
}

pub fn decode_html_entities(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

pub async fn load_docx_from_drive(
    state: &DocState,
    user_identifier: &str,
    file_path: &str,
) -> Result<Document, String> {
    let bytes = state
        .drive
        .get_object(&state.bucket_name, file_path)
        .await
        .map_err(|e| format!("Failed to load DOCX: {e}"))?;

    load_docx_from_bytes(&bytes, user_identifier, file_path).await
}

pub async fn load_docx_from_bytes(
    bytes: &[u8],
    user_identifier: &str,
    file_path: &str,
) -> Result<Document, String> {
    let raw_name = file_path.split('/').next_back().unwrap_or("Untitled");
    let file_name = raw_name
        .strip_suffix(".docx")
        .or_else(|| raw_name.strip_suffix(".doc"))
        .unwrap_or(raw_name);

    let doc_id = generate_doc_id();

    cache_document_bytes(&doc_id, bytes.to_vec()).await;

    let html_content = match crate::ooxml::load_docx_preserving(bytes) {
        Ok(ooxml_doc) => {
            let texts: Vec<String> = ooxml_doc.paragraphs.iter().map(|p| p.text.clone()).collect();
            paragraphs_to_html(&texts)
        }
        Err(_) => convert_docx_to_html(bytes)?,
    };

    Ok(Document {
        id: doc_id,
        title: file_name.to_string(),
        content: html_content,
        owner_id: user_identifier.to_string(),
        storage_path: file_path.to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        collaborators: Vec::new(),
        version: 1,
        track_changes: None,
        comments: None,
        footnotes: None,
        endnotes: None,
        styles: None,
        toc: None,
        track_changes_enabled: false,
    })
}
