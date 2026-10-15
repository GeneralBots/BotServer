use chrono::{DateTime, Duration, Utc};

pub fn format_document_list_item(
    id: &str,
    title: &str,
    updated_at: DateTime<Utc>,
    word_count: usize,
) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "title": title,
        "updated_at": updated_at.to_rfc3339(),
        "updated_relative": format_relative_time(updated_at),
        "word_count": word_count
    })
}

pub fn format_document_content(
    id: &str,
    title: &str,
    content: &str,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "title": title,
        "content": content,
        "created_at": created_at.to_rfc3339(),
        "updated_at": updated_at.to_rfc3339(),
        "word_count": count_words(content)
    })
}

pub fn format_error(message: &str) -> serde_json::Value {
    serde_json::json!({ "error": message, "success": false })
}

pub fn format_relative_time(dt: DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = now.signed_duration_since(dt);

    if diff < Duration::minutes(1) {
        "just now".to_string()
    } else if diff < Duration::hours(1) {
        let mins = diff.num_minutes();
        format!("{} minute{} ago", mins, if mins == 1 { "" } else { "s" })
    } else if diff < Duration::days(1) {
        let hours = diff.num_hours();
        format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
    } else if diff < Duration::days(7) {
        let days = diff.num_days();
        format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
    } else if diff < Duration::days(30) {
        let weeks = diff.num_weeks();
        format!("{} week{} ago", weeks, if weeks == 1 { "" } else { "s" })
    } else {
        dt.format("%b %d, %Y").to_string()
    }
}

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
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

pub fn count_words(text: &str) -> usize {
    let plain_text = strip_html(text);
    plain_text
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .count()
}

pub fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }

    let truncated: String = text.chars().take(max_chars).collect();
    if let Some(last_space) = truncated.rfind(' ') {
        format!("{}...", &truncated[..last_space])
    } else {
        format!("{}...", truncated)
    }
}

pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

pub fn generate_document_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn get_user_docs_path(user_id: &str) -> String {
    format!("users/{}/docs", user_id)
}
