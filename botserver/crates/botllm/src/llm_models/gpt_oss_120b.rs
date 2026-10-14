use super::{ModelHandler, ProcessedChunk};
use log;

/// Handler for GPT-OSS 120B model with thinking tags filtering
#[derive(Debug)]
pub struct GptOss120bHandler {}

impl Default for GptOss120bHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl GptOss120bHandler {
    pub fn new() -> Self {
        Self {}
    }
}

/// Extract content outside thinking tags
/// If everything is inside thinking tags, extract from inside them
fn strip_think_tags(content: &str) -> String {
    if content.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let mut in_thinking = false;
    let mut thinking_content = String::new();
    let chars: Vec<char> = content.chars().collect();
    let mut pos = 0;

    while pos < chars.len() {
        if !in_thinking {
            if pos + 10 <= chars.len() && chars[pos..pos+10].iter().collect::<String>() == "<thinking>" {
                in_thinking = true;
                thinking_content.clear();
                pos += 10;
                continue;
            } else if pos + 9 <= chars.len() && chars[pos..pos+9].iter().collect::<String>() == "**start**" {
                in_thinking = true;
                thinking_content.clear();
                pos += 9;
                continue;
            }
        } else {
            if pos + 12 <= chars.len() && chars[pos..pos+12].iter().collect::<String>() == "</thinking>" {
                in_thinking = false;
                pos += 12;
                continue;
            } else if pos + 7 <= chars.len() && chars[pos..pos+7].iter().collect::<String>() == "**end**" {
                in_thinking = false;
                pos += 7;
                continue;
            } else {
                thinking_content.push(chars[pos]);
                pos += 1;
                continue;
            }
        }

        result.push(chars[pos]);
        pos += 1;
    }

    // If we got content outside thinking tags, return it
    if !result.trim().is_empty() {
        return result;
    }

    // If everything was inside thinking tags, return that content
    if !thinking_content.trim().is_empty() {
        log::debug!("gpt_oss_120b: All content was in thinking tags, returning thinking content");
        return thinking_content;
    }

    // Fallback: try regex extraction
    if let Some(re) = crate::re_cache::cached_re!(r"<thinking>(.*?)</thinking>") {
        let mut extracted = String::new();
        for cap in re.captures_iter(content) {
            if let Some(m) = cap.get(1) {
                if !extracted.is_empty() {
                    extracted.push(' ');
                }
                extracted.push_str(m.as_str());
            }
        }
        if !extracted.is_empty() {
            return extracted;
        }
    }

    // Last resort: return original
    log::warn!("gpt_oss_120b: Could not extract meaningful content, returning original");
    content.to_string()
}

impl ModelHandler for GptOss120bHandler {
    fn is_analysis_complete(&self, buffer: &str) -> bool {
        buffer.contains("**end**") || buffer.contains("</thinking>")
    }

    fn process_content(&self, content: &str) -> String {
        let cleaned = strip_think_tags(content);
        if cleaned.is_empty() {
            return String::new();
        }
        // Strip reasoning/chain-of-thought before first HTML structural tag.
        let html_start = cleaned.find("<div")
            .or_else(|| cleaned.find("<h2"))
            .or_else(|| cleaned.find("<h3"))
            .or_else(|| cleaned.find("<p>"))
            .or_else(|| cleaned.find("<table"))
            .or_else(|| cleaned.find("<blockquote"))
            .or_else(|| cleaned.find("<hr"));
        match html_start {
            Some(pos) if pos > 0 => cleaned[pos..].to_string(),
            _ => cleaned,
        }
    }

    fn process_content_streaming(&self, chunk: &str, _state: &mut String) -> ProcessedChunk {
        // Forward each chunk as-is for responsive streaming.
        // The final content is cleaned via process_content at the end.
        ProcessedChunk { content: chunk.to_string(), reasoning: String::new() }
    }

    fn has_analysis_markers(&self, buffer: &str) -> bool {
        buffer.contains("**start**") || buffer.contains("<thinking>")
    }

    fn skip_reasoning_content(&self) -> bool {
        false
    }
}
