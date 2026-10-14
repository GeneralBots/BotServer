/*****************************************************************************
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
|  ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
|  ██  ███ ████  █ ██  █ ████  █████  ██████ ██   █ ████   █████ █████  ███   |
|  ██   ██ █     ██  ██ ██ ████  █████ ██  ██ ██      ██  █ ████  █████ ██  █ |
|   █████  █████  █████  ███  ██ ██   █████  ██████ ██   █ ████   █████  ███  |
|                                                                             |
| General Bots Copyright (c) pragmatismo.com.br. All rights reserved.         |
| Licensed under the MIT License.                                             |
| Licensed under the MIT License.                                             |
******************************************************************************/

/// Convert arbitrary LLM output into Telegram-safe HTML.
///
/// Telegram's `parse_mode: HTML` rejects the WHOLE message on the first
/// unsupported tag ("can't parse entities: Unsupported start tag \"?xml\""),
/// which surfaced when classify_media replies echoed markup from analyzed
/// documents. Instead of gambling on the send and falling back to
/// unformatted text, the adapter normalizes content up front:
///
/// - REAL formatting tags (`b i u s code pre`) are kept — Telegram
///   supports them;
/// - any other tag (`<?xml …>`, `<table>`, partial markup) is stripped and
///   its text content kept; `br` becomes a newline, block containers add
///   separation;
/// - a `<` that does not start a plausible tag is escaped, as are bare
///   `&`; existing entities (`&lt;`, `&#39;`, …) pass through untouched.
pub fn sanitize_for_html(input: &str) -> String {
    const ALLOWED: [&str; 6] = ["b", "i", "u", "s", "code", "pre"];
    const BLOCKISH: [&str; 9] = ["p", "div", "table", "tr", "ul", "ol", "h1", "h2", "h3"];

    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;

    while i < bytes.len() {
        let ch = input[i..].chars().next().unwrap_or(' ');

        if ch == '<' {
            let close_rel = input[i + 1..].find('>');
            // A plausible tag starts with a letter, '/', '?' (XML decl) or
            // '!' (comment). "< b" is just a stray bracket — escape it.
            let plausible = input[i + 1..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '/' || c == '?' || c == '!');

            if plausible {
                if let Some(rel) = close_rel {
                    let close = i + 1 + rel;
                    let tag_body = input[i + 1..close].trim();
                    let (is_close, raw_name) = match tag_body.strip_prefix('/') {
                        Some(rest) => (
                            true,
                            rest.split_whitespace()
                                .next()
                                .unwrap_or("")
                                .to_lowercase(),
                        ),
                        None => (
                            false,
                            tag_body
                                .split_whitespace()
                                .next()
                                .unwrap_or("")
                                .to_lowercase(),
                        ),
                    };
                    let name = raw_name.trim_end_matches('/');

                    if ALLOWED.contains(&name) {
                        // Keep real formatting tags byte-for-byte.
                        out.push_str(&input[i..=close]);
                    } else if name == "br" {
                        out.push('\n');
                    } else {
                        // Unsupported markup: drop the tag, keep the text
                        // after it; block containers gain separation.
                        if !is_close && BLOCKISH.contains(&name) {
                            out.push('\n');
                        }
                    }
                    i = close + 1;
                    continue;
                }
                // No closing '>' at all: not markup Telegram could parse.
                out.push_str("&lt;");
                i += 1;
                continue;
            }

            if let Some(rel) = close_rel {
                // Plausible-looking but malformed (`< 1 >`): drop just the
                // bracket-looking fragment, keep the inside as text.
                let _ = rel;
            }
            out.push_str("&lt;");
            i += 1;
            continue;
        }

        if ch == '&' {
            // Entity: &name; or &#123; — only alphanumerics/# inside, short.
            if let Some(semi_rel) = input[i + 1..].find(';') {
                let name = &input[i + 1..i + 1 + semi_rel];
                let ok = !name.is_empty()
                    && name.len() <= 10
                    && name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '#')
                    && name.chars().any(|c| c.is_ascii_alphanumeric());
                if ok {
                    out.push_str(&input[i..i + 1 + semi_rel + 1]);
                    i += 1 + semi_rel + 1;
                    continue;
                }
            }
            out.push_str("&amp;");
            i += 1;
            continue;
        }

        if ch == '>' {
            // Bare `>` outside any tag (tags kept above are copied
            // byte-for-byte and never reach this branch).
            out.push_str("&gt;");
            i += 1;
            continue;
        }

        let len = ch.len_utf8();
        out.push_str(&input[i..i + len]);
        i += len;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::sanitize_for_html;

    #[test]
    fn xml_declaration_is_stripped_not_rejected() {
        let src = "<?xml version=\"1.0\"?>\n<document>Relat\u{f3}rio <b>importante</b></document>";
        let out = sanitize_for_html(src);
        assert!(!out.contains("<?xml"), "got: {out}");
        assert!(!out.contains("<document>"), "got: {out}");
        assert!(out.contains("Relat\u{f3}rio"), "got: {out}");
        assert!(out.contains("<b>importante</b>"), "got: {out}");
    }

    #[test]
    fn bare_angle_brackets_are_escaped() {
        let out = sanitize_for_html("compare a < b and x > y");
        assert_eq!(out, "compare a &lt; b and x &gt; y");
    }

    #[test]
    fn ampersands_are_escaped_but_entities_kept() {
        let out = sanitize_for_html("R&D and caf&eacute;");
        assert!(out.contains("R&amp;D"), "got: {out}");
        assert!(out.contains("caf&eacute;"), "got: {out}");
    }

    #[test]
    fn br_becomes_newline_and_block_tags_add_separation() {
        let out = sanitize_for_html("line1<br/>line2<p>line3</p>");
        assert!(out.contains("line1\nline2"), "got: {out}");
        assert!(out.contains("\nline3"), "got: {out}");
        assert!(!out.contains("<p>"), "got: {out}");
    }

    #[test]
    fn code_and_pre_survive() {
        let out = sanitize_for_html("<code>x &lt; 1</code>");
        assert_eq!(out, "<code>x &lt; 1</code>");
    }

    #[test]
    fn numeric_entities_pass_through() {
        let out = sanitize_for_html("item &#39;quoted&#39; end");
        assert_eq!(out, "item &#39;quoted&#39; end");
    }

    #[test]
    fn unsupported_tag_drops_markup_keeps_text() {
        let out = sanitize_for_html("a <table><tr><td>cell</td></tr></table> b");
        assert!(!out.contains("<"), "got: {out}");
        assert!(out.contains("cell"), "got: {out}");
        assert!(out.starts_with("a "), "got: {out}");
        assert!(out.trim_end().ends_with(" b"), "got: {out}");
    }
}
