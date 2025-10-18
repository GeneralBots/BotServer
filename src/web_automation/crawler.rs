use log::{debug, error, info};
use reqwest::Client;
use scraper::{Html, Selector};
use std::error::Error;
use std::time::Duration;

/// Web crawler for extracting content from web pages
pub struct WebCrawler {
    client: Client,
}

impl WebCrawler {
    /// Create a new web crawler
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .user_agent("Mozilla/5.0 (compatible; GeneralBots/1.0)")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client }
    }

    /// Validate if string is a valid HTTP(S) URL
    pub fn is_valid_url(url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }

    /// Fetch website content via HTTP
    pub async fn fetch_content(&self, url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        debug!("Fetching website content from: {}", url);

        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(format!("HTTP request failed with status: {}", response.status()).into());
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !content_type.contains("text/html") && !content_type.contains("application/xhtml") {
            return Err(format!("URL does not return HTML content: {}", content_type).into());
        }

        let html_content = response.text().await?;
        debug!("Fetched {} bytes of HTML content", html_content.len());

        Ok(html_content)
    }

    /// Extract readable text from HTML
    pub fn extract_text_from_html(
        &self,
        html: &str,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let document = Html::parse_document(html);

        let mut text_parts = Vec::new();

        // Extract title
        let title_selector = Selector::parse("title").unwrap();
        if let Some(title_element) = document.select(&title_selector).next() {
            let title = title_element.text().collect::<String>();
            if !title.trim().is_empty() {
                text_parts.push(format!("Title: {}\n", title.trim()));
            }
        }

        // Extract meta description
        let meta_selector = Selector::parse("meta[name='description']").unwrap();
        if let Some(meta) = document.select(&meta_selector).next() {
            if let Some(description) = meta.value().attr("content") {
                if !description.trim().is_empty() {
                    text_parts.push(format!("Description: {}\n", description.trim()));
                }
            }
        }

        // Extract body content
        let body_selector = Selector::parse("body").unwrap();
        if let Some(body) = document.select(&body_selector).next() {
            self.extract_text_recursive(&body, &mut text_parts);
        } else {
            // Fallback: extract from entire document
            for node in document.root_element().descendants() {
                if let Some(text) = node.value().as_text() {
                    let cleaned = text.trim();
                    if !cleaned.is_empty() {
                        text_parts.push(cleaned.to_string());
                    }
                }
            }
        }

        let combined_text = text_parts.join("\n");

        // Clean up excessive whitespace
        let cleaned = combined_text
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        if cleaned.is_empty() {
            return Err("Failed to extract text from HTML".into());
        }

        Ok(cleaned)
    }

    /// Recursively extract text from HTML element tree
    fn extract_text_recursive(&self, element: &scraper::ElementRef, text_parts: &mut Vec<String>) {
        // Skip excluded elements (script, style, etc.)
        let excluded = ["script", "style", "noscript", "iframe", "svg"];
        if excluded.contains(&element.value().name()) {
            return;
        }

        for child in element.children() {
            if let Some(text) = child.value().as_text() {
                let cleaned = text.trim();
                if !cleaned.is_empty() {
                    text_parts.push(cleaned.to_string());
                }
            } else if child.value().as_element().is_some() {
                if let Some(child_ref) = scraper::ElementRef::wrap(child) {
                    self.extract_text_recursive(&child_ref, text_parts);
                }
            }
        }
    }

    /// Crawl a URL and return extracted text
    pub async fn crawl(&self, url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        info!("Crawling website: {}", url);

        if !Self::is_valid_url(url) {
            return Err("Invalid URL format".into());
        }

        let html_content = self.fetch_content(url).await?;
        let text_content = self.extract_text_from_html(&html_content)?;

        if text_content.trim().is_empty() {
            return Err("No text content found on website".into());
        }

        info!(
            "Successfully crawled website: {} ({} characters)",
            url,
            text_content.len()
        );

        Ok(text_content)
    }
}

impl Default for WebCrawler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_url() {
        assert!(WebCrawler::is_valid_url("https://example.com"));
        assert!(WebCrawler::is_valid_url("http://example.com"));
        assert!(WebCrawler::is_valid_url("https://example.com/path?query=1"));

        assert!(!WebCrawler::is_valid_url("ftp://example.com"));
        assert!(!WebCrawler::is_valid_url("example.com"));
        assert!(!WebCrawler::is_valid_url("//example.com"));
        assert!(!WebCrawler::is_valid_url("file:///etc/passwd"));
    }

    #[test]
    fn test_extract_text_from_html() {
        let crawler = WebCrawler::new();

        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Test Page</title>
                <meta name="description" content="This is a test page">
                <style>body { color: red; }</style>
                <script>console.log('test');</script>
            </head>
            <body>
                <h1>Welcome</h1>
                <p>This is a paragraph.</p>
                <div>
                    <span>Nested content</span>
                </div>
            </body>
            </html>
        "#;

        let result = crawler.extract_text_from_html(html).unwrap();

        assert!(result.contains("Title: Test Page"));
        assert!(result.contains("Description: This is a test page"));
        assert!(result.contains("Welcome"));
        assert!(result.contains("This is a paragraph"));
        assert!(result.contains("Nested content"));
        assert!(!result.contains("console.log"));
        assert!(!result.contains("color: red"));
    }

    #[test]
    fn test_extract_text_empty_html() {
        let crawler = WebCrawler::new();
        let html = "<html><body></body></html>";
        let result = crawler.extract_text_from_html(html);
        assert!(result.is_err());
    }
}
