use super::*;

pub(super) fn parse_duckduckgo_html(html: &str, max_results: usize) -> Vec<WebSearchResult> {
    let mut results = Vec::new();

    let mut current_title = String::new();
    let mut current_url = String::new();
    let mut current_snippet = String::new();

    for line in html.lines() {
        let line = line.trim();

        if line.contains("class=\"result__a\"") {
            if let Some(href_start) = line.find("href=\"") {
                let start = href_start + 6;
                if let Some(href_end) = line[start..].find('"') {
                    let raw_url = &line[start..start + href_end];
                    current_url = decode_ddg_url(raw_url);
                }
            }

            if let Some(title_start) = line.find('>') {
                let after_tag = &line[title_start + 1..];
                if let Some(title_end) = after_tag.find('<') {
                    current_title = html_decode(&after_tag[..title_end]);
                }
            }
        }

        if line.contains("class=\"result__snippet\"") {
            if let Some(snippet_start) = line.find('>') {
                let after_tag = &line[snippet_start + 1..];
                let snippet_text = strip_html_inline(after_tag);
                current_snippet = html_decode(&snippet_text);
            }

            if !current_title.is_empty() && !current_url.is_empty() {
                let domain = extract_domain(&current_url);
                results.push(WebSearchResult {
                    title: current_title.clone(),
                    url: current_url.clone(),
                    snippet: current_snippet.clone(),
                    source: domain.clone(),
                    favicon: Some(format!(
                        "https://www.google.com/s2/favicons?domain={}",
                        domain
                    )),
                    published_date: None,
                });

                current_title.clear();
                current_url.clear();
                current_snippet.clear();

                if results.len() >= max_results {
                    break;
                }
            }
        }
    }

    if results.is_empty() {
        results = parse_duckduckgo_fallback(html, max_results);
    }

    results
}
pub(super) fn parse_duckduckgo_fallback(html: &str, max_results: usize) -> Vec<WebSearchResult> {
    let mut results = Vec::new();

    let parts: Vec<&str> = html.split("class=\"result ").collect();

    for part in parts.iter().skip(1).take(max_results) {
        let mut title = String::new();
        let mut url = String::new();
        let mut snippet = String::new();

        if let Some(a_start) = part.find("class=\"result__a\"") {
            let section = &part[a_start..];

            if let Some(href_pos) = section.find("href=\"") {
                let start = href_pos + 6;
                if let Some(end) = section[start..].find('"') {
                    url = decode_ddg_url(&section[start..start + end]);
                }
            }

            if let Some(text_start) = section.find('>') {
                let after = &section[text_start + 1..];
                if let Some(text_end) = after.find('<') {
                    title = html_decode(&after[..text_end]);
                }
            }
        }

        if let Some(snippet_start) = part.find("class=\"result__snippet\"") {
            let section = &part[snippet_start..];
            if let Some(text_start) = section.find('>') {
                let after = &section[text_start + 1..];
                let text = strip_html_inline(after);
                snippet = html_decode(&text);
                if let Some(end) = snippet.find("</") {
                    snippet.truncate(end);
                }
            }
        }

        if !title.is_empty() && !url.is_empty() {
            let domain = extract_domain(&url);
            results.push(WebSearchResult {
                title,
                url: url.clone(),
                snippet,
                source: domain.clone(),
                favicon: Some(format!(
                    "https://www.google.com/s2/favicons?domain={}",
                    domain
                )),
                published_date: None,
            });
        }
    }

    results
}
pub(super) fn decode_ddg_url(raw_url: &str) -> String {
    if raw_url.starts_with("//duckduckgo.com/l/?uddg=") {
        let encoded_part = raw_url.trim_start_matches("//duckduckgo.com/l/?uddg=");
        if let Some(amp_pos) = encoded_part.find('&') {
            let url_part = &encoded_part[..amp_pos];
            return urlencoding::decode(url_part)
                .map(|s| s.to_string())
                .unwrap_or_else(|_| raw_url.to_string());
        }
        return urlencoding::decode(encoded_part)
            .map(|s| s.to_string())
            .unwrap_or_else(|_| raw_url.to_string());
    }

    if raw_url.starts_with("http") {
        return raw_url.to_string();
    }

    format!("https:{}", raw_url)
}
pub(super) fn extract_domain(url: &str) -> String {
    let without_protocol = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");

    if let Some(slash_pos) = without_protocol.find('/') {
        without_protocol[..slash_pos].to_string()
    } else {
        without_protocol.to_string()
    }
}
pub(super) fn strip_html_inline(s: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;

    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }

    result.trim().to_string()
}
pub(super) fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&#x27;", "'")
        .replace("&#x2F;", "/")
}
pub(super) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
pub(super) fn generate_related_queries(query: &str) -> Vec<String> {
    let base_words: Vec<&str> = query.split_whitespace().collect();

    let mut related = Vec::new();

    related.push(format!("what is {}", query));
    related.push(format!("{} explained", query));
    related.push(format!("{} examples", query));
    related.push(format!("how does {} work", query));
    related.push(format!("{} vs alternatives", query));

    if base_words.len() > 2 {
        let shortened: String = base_words[..2].join(" ");
        related.push(shortened);
    }

    related.into_iter().take(5).collect()
}
pub(super) async fn get_instant_answer(query: &str) -> Option<(String, String)> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;

    let encoded = urlencoding::encode(query);
    let url = format!(
        "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
        encoded
    );

    let response = client.get(&url).send().await.ok()?;
    let json: serde_json::Value = response.json().await.ok()?;

    if let Some(abstract_text) = json.get("AbstractText").and_then(|v| v.as_str()) {
        if !abstract_text.is_empty() {
            let answer_type = json
                .get("Type")
                .and_then(|v| v.as_str())
                .unwrap_or("A")
                .to_string();
            return Some((abstract_text.to_string(), answer_type));
        }
    }

    if let Some(answer) = json.get("Answer").and_then(|v| v.as_str()) {
        if !answer.is_empty() {
            return Some((answer.to_string(), "answer".to_string()));
        }
    }

    if let Some(definition) = json.get("Definition").and_then(|v| v.as_str()) {
        if !definition.is_empty() {
            return Some((definition.to_string(), "definition".to_string()));
        }
    }

    None
}
