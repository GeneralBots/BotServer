

/// Removes a trailing statement terminator `;` from a parameter string,
/// but only when the semicolon sits OUTSIDE quoted literals. The compiled
/// `.ast` files produced by BasicCompiler append `;` to every line; without
/// this the terminator leaks into the last function-call argument and the
/// rewritten script fails to parse (e.g. `vibe_run("build an app";)`).
/// Unquote a BASIC token that may arrive wrapped in double quotes.
pub(super) fn unquote(s: &str) -> &str {
    s.trim().trim_matches('"')
}
pub(super) fn strip_trailing_stmt_semicolon(s: &str) -> String {    let trimmed = s.trim_end();
    if !trimmed.ends_with(';') {
        return trimmed.to_string();
    }
    let mut in_quote: Option<char> = None;
    for (i, c) in trimmed.char_indices() {
        if let Some(q) = in_quote {
            if c == q {
                in_quote = None;
            }
        } else if c == '"' || c == '\'' {
            in_quote = Some(c);
        }
        if c == ';' && in_quote.is_none() && i == trimmed.len() - 1 {
            return trimmed[..i].to_string();
        }
    }
    trimmed.to_string()
}
pub(super) fn parse_parameters(params_str: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = '"';
    let mut paren_depth: i32 = 0;
    let chars: Vec<char> = params_str.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        match c {
            '"' | '\'' if !in_quotes => {
                in_quotes = true;
                quote_char = c;
                current.push(c);
            }
            '"' | '\'' if in_quotes && c == quote_char => {
                in_quotes = false;
                current.push(c);
            }
            '(' if !in_quotes => {
                paren_depth += 1;
                current.push(c);
            }
            ')' if !in_quotes => {
                paren_depth -= 1;
                current.push(c);
            }
            ',' if !in_quotes && paren_depth == 0 => {
                if !current.is_empty() {
                    params.push(current.trim().to_string());
                    current = String::new();
                }
            }
            _ => {
                current.push(c);
            }
        }
        i += 1;
    }

    if !current.is_empty() {
        params.push(current.trim().to_string());
    }

    params
}
pub fn preprocess_llm_keyword(script: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = script.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let remaining: String = chars[i..].iter().collect();
        let remaining_upper = remaining.to_uppercase();

        if remaining_upper.starts_with("LLM ") {
            result.push_str("LLM ");
            i += 4;

            if i < chars.len() && chars[i] == '"' {
                result.push('"');
                i += 1;

                while i < chars.len() && chars[i] != '"' {
                    result.push(chars[i]);
                    i += 1;
                }
                if i < chars.len() && chars[i] == '"' {
                    result.push('"');
                    i += 1;
                }

                let before_with = result.trim_end_matches('"');
                if !before_with.to_uppercase().contains("WITH OPTIMIZE") {
                    result = format!("{} WITH OPTIMIZE FOR \"speed\"", before_with);
                }
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}
