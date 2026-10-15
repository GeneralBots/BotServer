use super::utils_format::{html_escape, strip_html};
use crate::re_cache::cached_re;

pub fn html_to_markdown(html: &str) -> String {
    let mut md = html.to_string();
    md = md.replace("<strong>", "**").replace("</strong>", "**");
    md = md.replace("<b>", "**").replace("</b>", "**");
    md = md.replace("<em>", "*").replace("</em>", "*");
    md = md.replace("<i>", "*").replace("</i>", "*");
    md = md.replace("<u>", "_").replace("</u>", "_");
    md = md.replace("<h1>", "# ").replace("</h1>", "\n");
    md = md.replace("<h2>", "## ").replace("</h2>", "\n");
    md = md.replace("<h3>", "### ").replace("</h3>", "\n");
    md = md.replace("<h4>", "#### ").replace("</h4>", "\n");
    md = md.replace("<h5>", "##### ").replace("</h5>", "\n");
    md = md.replace("<h6>", "###### ").replace("</h6>", "\n");
    md = md.replace("<br>", "\n").replace("<br/>", "\n").replace("<br />", "\n");
    md = md.replace("<p>", "").replace("</p>", "\n\n");
    md = md.replace("<li>", "- ").replace("</li>", "\n");
    md = md.replace("<ul>", "").replace("</ul>", "\n");
    md = md.replace("<ol>", "").replace("</ol>", "\n");
    md = md.replace("<blockquote>", "> ").replace("</blockquote>", "\n");
    md = md.replace("<code>", "`").replace("</code>", "`");
    md = md.replace("<code>", "`").replace("</code>", "`");
    md = md.replace("<pre>", "```\n").replace("</pre>", "\n```\n");
    md = md.replace("<hr>", "\n---\n").replace("<hr/>", "\n---\n");

    strip_html(&md)
}

pub fn markdown_to_html(md: &str) -> String {
    let mut html = String::new();
    let lines: Vec<&str> = md.lines().collect();
    let mut in_code_block = false;
    let mut in_list = false;

    for line in lines {
        if line.starts_with("```") {
            if in_code_block {
                html.push_str("</pre>");
                in_code_block = false;
            } else {
                html.push_str("<pre>");
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            html.push_str(&html_escape(line));
            html.push('\n');
            continue;
        }

        let processed = process_markdown_line(line);

        if line.starts_with("- ") || line.starts_with("* ") {
            if !in_list {
                html.push_str("<ul>");
                in_list = true;
            }
            html.push_str(&format!("<li>{}</li>", &processed[2..]));
        } else {
            if in_list {
                html.push_str("</ul>");
                in_list = false;
            }
            html.push_str(&processed);
        }
    }

    if in_list {
        html.push_str("</ul>");
    }
    if in_code_block {
        html.push_str("</pre>");
    }

    html
}

fn process_markdown_line(line: &str) -> String {
    let mut result = line.to_string();

    // Ordered longest-first so `### ` never matches the `# ` rule.
    const HEADINGS: [(&str, &str); 6] = [
        ("###### ", "h6"),
        ("##### ", "h5"),
        ("#### ", "h4"),
        ("### ", "h3"),
        ("## ", "h2"),
        ("# ", "h1"),
    ];
    for (prefix, tag) in HEADINGS {
        if let Some(text) = line.strip_prefix(prefix) {
            return format!("<{tag}>{text}</{tag}>");
        }
    }
    if let Some(text) = line.strip_prefix("> ") {
        return format!("<blockquote>{text}</blockquote>");
    }
    if line == "---" || line == "***" || line == "___" {
        return "<hr>".to_string();
    }

    result = process_inline_formatting(&result);

    if !result.is_empty() && !result.starts_with('<') {
        result = format!("<p>{}</p>", result);
    }

    result
}

fn process_inline_formatting(text: &str) -> String {
    let mut result = text.to_string();

    let bold_re = cached_re!(r"\*\*(.+?)\*\*");
    if let Some(re) = bold_re {
        result = re.replace_all(&result, "<strong>$1</strong>").to_string();
    }

    let italic_re = cached_re!(r"\*(.+?)\*");
    if let Some(re) = italic_re {
        result = re.replace_all(&result, "<em>$1</em>").to_string();
    }

    let code_re = cached_re!(r"`(.+?)`");
    if let Some(re) = code_re {
        result = re.replace_all(&result, "<code>$1</code>").to_string();
    }

    let link_re = cached_re!(r"\[(.+?)\]\((.+?)\)");
    if let Some(re) = link_re {
        result = re
            .replace_all(&result, r#"<a href="$2">$1</a>"#)
            .to_string();
    }

    result
}

pub fn rtf_to_html(rtf: &str) -> String {
    let mut html = String::new();
    let mut in_group = 0;
    let mut bold = false;
    let mut italic = false;
    let mut underline = false;
    let mut skip_chars = 0;
    let chars: Vec<char> = rtf.chars().collect();
    let mut i = 0;

    html.push_str("<div>");

    while i < chars.len() {
        if skip_chars > 0 {
            skip_chars -= 1;
            i += 1;
            continue;
        }

        let ch = chars[i];

        match ch {
            '{' => in_group += 1,
            '}' => in_group -= 1,
            '\\' => {
                let mut cmd = String::new();
                i += 1;
                while i < chars.len() && chars[i].is_ascii_alphabetic() {
                    cmd.push(chars[i]);
                    i += 1;
                }

                match cmd.as_str() {
                    "b" => {
                        if !bold {
                            html.push_str("<strong>");
                            bold = true;
                        }
                    }
                    "b0" => {
                        if bold {
                            html.push_str("</strong>");
                            bold = false;
                        }
                    }
                    "i" => {
                        if !italic {
                            html.push_str("<em>");
                            italic = true;
                        }
                    }
                    "i0" => {
                        if italic {
                            html.push_str("</em>");
                            italic = false;
                        }
                    }
                    "ul" => {
                        if !underline {
                            html.push_str("<u>");
                            underline = true;
                        }
                    }
                    "ulnone" => {
                        if underline {
                            html.push_str("</u>");
                            underline = false;
                        }
                    }
                    "par" | "line" => html.push_str("<br>"),
                    "tab" => html.push_str("&nbsp;&nbsp;&nbsp;&nbsp;"),
                    _ => {}
                }

                if i < chars.len() && chars[i] == ' ' {
                    i += 1;
                }
                continue;
            }
            '\n' | '\r' => {}
            _ => {
                if in_group <= 1 {
                    html.push(ch);
                }
            }
        }
        i += 1;
    }

    if underline {
        html.push_str("</u>");
    }
    if italic {
        html.push_str("</em>");
    }
    if bold {
        html.push_str("</strong>");
    }

    html.push_str("</div>");
    html
}

pub fn html_to_rtf(html: &str) -> String {
    let mut rtf = String::from("{\\rtf1\\ansi\\deff0\n");
    rtf.push_str("{\\fonttbl{\\f0 Arial;}}\n");
    rtf.push_str("\\f0\\fs24\n");

    let mut result = html.to_string();
    result = result.replace("<strong>", "\\b ");
    result = result.replace("</strong>", "\\b0 ");
    result = result.replace("<b>", "\\b ");
    result = result.replace("</b>", "\\b0 ");
    result = result.replace("<em>", "\\i ");
    result = result.replace("</em>", "\\i0 ");
    result = result.replace("<i>", "\\i ");
    result = result.replace("</i>", "\\i0 ");
    result = result.replace("<u>", "\\ul ");
    result = result.replace("</u>", "\\ulnone ");
    result = result.replace("<br>", "\\par\n");
    result = result.replace("<br/>", "\\par\n");
    result = result.replace("<br />", "\\par\n");
    result = result.replace("<p>", "");
    result = result.replace("</p>", "\\par\\par\n");
    result = result.replace("<h1>", "\\fs48\\b ");
    result = result.replace("</h1>", "\\b0\\fs24\\par\n");
    result = result.replace("<h2>", "\\fs36\\b ");
    result = result.replace("</h2>", "\\b0\\fs24\\par\n");
    result = result.replace("<h3>", "\\fs28\\b ");
    result = result.replace("</h3>", "\\b0\\fs24\\par\n");

    let stripped = strip_html(&result);
    rtf.push_str(&stripped);
    rtf.push('}');
    rtf
}

pub fn odt_content_to_html(odt_xml: &str) -> String {
    let mut html = String::from("<div>");

    let mut in_text = false;
    let mut in_span = false;
    let mut current_text = String::new();
    let chars: Vec<char> = odt_xml.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '<' {
            let mut tag = String::new();
            i += 1;
            while i < chars.len() && chars[i] != '>' {
                tag.push(chars[i]);
                i += 1;
            }

            if tag.starts_with("text:p") {
                if !current_text.is_empty() {
                    html.push_str(&current_text);
                    current_text.clear();
                }
                html.push_str("<p>");
                in_text = true;
            } else if tag == "/text:p" {
                html.push_str(&current_text);
                current_text.clear();
                html.push_str("</p>");
                in_text = false;
            } else if tag.starts_with("text:span") {
                if tag.contains("Bold") {
                    html.push_str("<strong>");
                } else if tag.contains("Italic") {
                    html.push_str("<em>");
                }
                in_span = true;
            } else if tag == "/text:span" {
                html.push_str(&current_text);
                current_text.clear();
                if in_span {
                    html.push_str("</strong>");
                }
                in_span = false;
            } else if tag.starts_with("text:h") {
                let level = tag
                    .chars()
                    .find(|c| c.is_ascii_digit())
                    .unwrap_or('1');
                html.push_str(&format!("<h{level}>"));
                in_text = true;
            } else if tag.starts_with("/text:h") {
                html.push_str(&current_text);
                current_text.clear();
                html.push_str("</h1>");
                in_text = false;
            } else if tag == "text:line-break" || tag == "text:line-break/" {
                current_text.push_str("<br>");
            } else if tag == "text:tab" || tag == "text:tab/" {
                current_text.push_str("&nbsp;&nbsp;&nbsp;&nbsp;");
            }
        } else if in_text {
            current_text.push(chars[i]);
        }
        i += 1;
    }

    if !current_text.is_empty() {
        html.push_str(&current_text);
    }

    html.push_str("</div>");
    html
}

pub fn html_to_odt_content(html: &str) -> String {
    let mut odt = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
office:version="1.2">
<office:body>
<office:text>
"#,
    );

    let mut result = html.to_string();
    result = result.replace("<p>", "<text:p>");
    result = result.replace("</p>", "</text:p>\n");
    result = result.replace("<br>", "<text:line-break/>");
    result = result.replace("<br/>", "<text:line-break/>");
    result = result.replace("<br />", "<text:line-break/>");
    result = result.replace("<strong>", "<text:span text:style-name=\"Bold\">");
    result = result.replace("</strong>", "</text:span>");
    result = result.replace("<b>", "<text:span text:style-name=\"Bold\">");
    result = result.replace("</b>", "</text:span>");
    result = result.replace("<em>", "<text:span text:style-name=\"Italic\">");
    result = result.replace("</em>", "</text:span>");
    result = result.replace("<i>", "<text:span text:style-name=\"Italic\">");
    result = result.replace("</i>", "</text:span>");
    result = result.replace("<h1>", "<text:h text:outline-level=\"1\">");
    result = result.replace("</h1>", "</text:h>\n");
    result = result.replace("<h2>", "<text:h text:outline-level=\"2\">");
    result = result.replace("</h2>", "</text:h>\n");
    result = result.replace("<h3>", "<text:h text:outline-level=\"3\">");
    result = result.replace("</h3>", "</text:h>\n");

    let stripped = strip_html(&result);
    let paragraphs: Vec<&str> = stripped.lines().collect();
    for para in paragraphs {
        if !para.trim().is_empty() {
            odt.push_str(&format!("<text:p>{}</text:p>\n", para.trim()));
        }
    }

    odt.push_str("</office:text>\n</office:body>\n</office:document-content>");
    odt
}

pub fn detect_document_format(content: &[u8]) -> &'static str {
    if content.len() >= 4 {
        if &content[0..4] == b"PK\x03\x04" {
            if content.len() > 30 {
                let content_str =
                    String::from_utf8_lossy(&content[0..100.min(content.len())]);
                if content_str.contains("word/") {
                    return "docx";
                } else if content_str.contains("content.xml") {
                    return "odt";
                }
            }
            return "zip";
        }
        if &content[0..4] == b"{\\rt" {
            return "rtf";
        }
        if content.len() >= 2 && content[0] == 0xD0 && content[1] == 0xCF {
            return "doc";
        }
    }

    let text = String::from_utf8_lossy(content);
    if text.trim_start().starts_with("<!DOCTYPE html") || text.trim_start().starts_with("<html") {
        return "html";
    }
    if text.trim_start().starts_with('#') || text.contains("\n# ") {
        return "markdown";
    }

    "txt"
}

pub fn convert_to_html(content: &[u8]) -> Result<String, String> {
    let format = detect_document_format(content);
    let text = String::from_utf8_lossy(content).to_string();

    match format {
        "rtf" => Ok(rtf_to_html(&text)),
        "html" => Ok(text),
        "markdown" => Ok(markdown_to_html(&text)),
        "txt" => Ok(format!(
            "<p>{}</p>",
            html_escape(&text).replace('\n', "</p><p>")
        )),
        _ => Err(format!("Unsupported format: {format}")),
    }
}
