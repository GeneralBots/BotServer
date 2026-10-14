use super::*;

pub fn convert_multiword_keywords(script: &str) -> String {
    let multiword_patterns = vec![
        (r#"USE\s+WEBSITE"#, 1, 2, vec!["url", "refresh"]),
        (r#"USE\s+MODEL"#, 1, 1, vec!["model"]),
        (r#"USE\s+KB"#, 1, 1, vec!["kb_name"]),
        (r#"USE\s+TOOL"#, 1, 1, vec!["tool_path"]),

        (r#"SET\s+BOT\s+MEMORY"#, 2, 2, vec!["key", "value"]),
        (r#"SET\s+CONTEXT"#, 2, 2, vec!["key", "value"]),
        (r#"SET\s+USER"#, 1, 1, vec!["user_id"]),

        (r#"GET\s+BOT\s+MEMORY"#, 1, 1, vec!["key"]),

        (r#"CLEAR\s+KB"#, 0, 0, vec![]),
        (r#"CLEAR\s+SUGGESTIONS"#, 0, 0, vec![]),
        (r#"CLEAR\s+SWITCHERS"#, 0, 0, vec![]),
        (r#"CLEAR\s+TOOLS"#, 0, 0, vec![]),
        (r#"CLEAR\s+WEBSITES"#, 0, 0, vec![]),

        (r#"ADD\s+SUGGESTION\s+TOOL"#, 2, 2, vec!["tool", "text"]),
        (r#"ADD\s+SUGGESTION\s+TEXT"#, 2, 2, vec!["value", "text"]),
        (r#"ADD\s+SUGGESTION(?!\s+TOOL|\s+TEXT|_)"#, 2, 2, vec!["context", "text"]),
        (r#"ADD\s+SWITCHER"#, 2, 2, vec!["switcher", "text"]),
        (r#"ADD\s+MEMBER"#, 2, 2, vec!["name", "role"]),
        (r#"ADD\s+MEMBER"#, 2, 2, vec!["name", "role"]),

        (r#"CREATE\s+TASK"#, 1, 1, vec!["task"]),
        (r#"CREATE\s+DRAFT"#, 4, 4, vec!["to", "subject", "body", "attachments"]),
        (r#"CREATE\s+SITE"#, 1, 1, vec!["site"]),

        (r#"ON\s+FORM\s+SUBMIT"#, 1, 1, vec!["form"]),
        (r#"ON\s+EMAIL"#, 1, 1, vec!["filter"]),
        (r#"ON\s+EVENT"#, 1, 1, vec!["event"]),

        // Error handling: these used to rely on custom-syntax registrations in
        // the runtime engine, but Rhai keys custom syntax by the first token,
        // so the later-registered `ON $ident$ OF "table"` trigger keyword
        // silently overrode every `ON ERROR …` form at execution time (the
        // compile-only engine does not load the trigger crate, which is why
        // compiled scripts passed). Rewriting to plain function calls here
        // removes the ambiguity entirely.
        (r#"ON\s+ERROR\s+RESUME\s+NEXT"#, 0, 0, vec![]),
        (r#"ON\s+ERROR\s+GOTO\s+0"#, 0, 0, vec![]),
        (r#"CLEAR\s+ERROR"#, 0, 0, vec![]),

        // Multimodal perception keywords share the DESCRIBE/SPEECH/GENERATE
        // first tokens across custom syntaxes; Rhai keeps only the last
        // registration per first token, so only the last one worked at runtime
        // (DESCRIBE VIDEO lost to DESCRIBE IMAGE, etc.). Function forms are
        // unambiguous and used via these rewrites.
        (r#"DESCRIBE\s+IMAGE"#, 1, 1, vec!["source"]),
        (r#"DESCRIBE\s+VIDEO"#, 1, 1, vec!["source"]),
        (r#"SPEECH\s+TO\s+TEXT"#, 1, 1, vec!["source"]),

        (r#"SEND\s+MAIL"#, 4, 4, vec!["to", "subject", "body", "attachments"]),
        (r#"SEND\s+TEAMS\s+MESSAGE"#, 2, 2, vec!["chat_id", "message"]),
        (r#"SEND\s+TO"#, 2, 2, vec!["target", "message"]),

        (r#"HAS\s+ROLE"#, 1, 1, vec!["role"]),
        (r#"BOOK"#, 1, 1, vec!["event"]),

        (r#"VIBE\s+RUN"#, 1, 1, vec!["intent"]),
        (r#"VIBE\s+STATUS"#, 1, 1, vec!["run_id"]),
        (r#"VIBE\s+APPROVE"#, 1, 1, vec!["run_id"]),
        (r#"VIBE\s+CANCEL"#, 1, 1, vec!["run_id"]),
        (r#"VIBE\s+EVENTS"#, 1, 1, vec!["run_id"]),
        (r#"VIBE\s+TOOLS"#, 0, 0, vec![]),
    ];

    let mut result = String::new();
    // Open FOR EACH blocks awaiting their NEXT (innermost last).
    let mut foreach_stack: Vec<String> = Vec::new();

    for original in script.lines() {
        let rewritten = rewrite_bare_datetime_tokens(original);
        let line: &str = rewritten.as_str();
        let trimmed = line.trim();
        let mut converted = false;

        // FOR EACH var IN expr … NEXT: the compiled .ast stores the
        // underscored FOR_EACH form with a bare `NEXT;`, and the runtime
        // engine only registers the two-token "FOR EACH" custom syntax — so
        // execution-time self-heal produced `FOR_EACH x in e;` … `NEXT;`,
        // which Rhai rejects ("Expecting ';'" on the loop variable).
        // Rewriting to the Rhai-native `for var in (expr) { … }` makes the
        // dialect unambiguous at both compile and execution time.
        if let Some(caps) = crate::re_cache::cached_re!(
            r#"(?i)^\s*FOR[_\s]+EACH\s+([A-Za-z_]\w*)\s+IN\s+(.+?)\s*;?\s*$"#
        )
        .and_then(|re| re.captures(line))
        {
            let var = caps.get(1).map_or("", |m| m.as_str());
            let expr = strip_trailing_stmt_semicolon(caps.get(2).map_or("", |m| m.as_str().trim()));
            foreach_stack.push(var.to_lowercase());
            result.push_str(&format!("for {var} in ({expr}) {{\n"));
            continue;
        }
        if !foreach_stack.is_empty() {
            if let Some(caps) =
                crate::re_cache::cached_re!(r#"(?i)^\s*NEXT(?:\s+([A-Za-z_]\w*))?\s*;?\s*$"#)
                    .and_then(|re| re.captures(line))
            {
                let named = caps.get(1).map(|m| m.as_str().to_lowercase());
                let matches_top = match &named {
                    Some(n) => foreach_stack.last().is_some_and(|top| top == n),
                    None => true,
                };
                if matches_top {
                    foreach_stack.pop();
                    result.push_str("}\n");
                    continue;
                }
            }
        }

        // `ON EVENT "<event>"` is a declaration consumed at design time (the
        // compiler registers the tool for the event); at run time it collapses
        // to a no-op call. It needs its own rewrite because the generic table
        // derives the callee name from the pattern text, which breaks on a
        // pattern that embeds a capture group.
        if let Some(caps) = crate::re_cache::cached_re!(
            r#"(?i)^\s*ON\s+EVENT\s+("[^"]*"|[A-Za-z_]\w*)\s*;?\s*$"#
        )
        .and_then(|re| re.captures(line))
        {
            let event = unquote(caps.get(1).map_or("", |m| m.as_str()));
            result.push_str(&format!("on_event(\"{event}\");\n"));
            continue;
        }

        // CREATE FILE <path> WITH <data> needs a dedicated rewrite: the generic
        // comma-based parameter parser below cannot split on the WITH keyword.
        // Like ON ERROR, the CREATE FILE custom syntax is overridden at runtime
        // by the later-registered CREATE SITE syntax (same first token).
        if let Some(caps) = crate::re_cache::cached_re!(
            r#"(?i)^\s*(.*?)\bCREATE\s+FILE\s+(.+?)\s+WITH\s+(.+?)\s*;?$"#
        )
        .and_then(|re| re.captures(line))
        {
            // Regex groups: 1 = text before the keyword, 2 = path (may be an
            // expression like `destination + ".meta.txt"`), 3 = data.
            let prefix = caps.get(1).map_or("", |m| m.as_str());
            let path = caps.get(2).map_or("", |m| m.as_str().trim());
            let data = caps.get(3).map_or("", |m| m.as_str().trim());
            let data = strip_trailing_stmt_semicolon(data);
            result.push_str(&format!(
                "{prefix}create_file({path}, {data});\n"
            ));
            continue;
        }

        let trimmed_upper = trimmed.to_uppercase();
        if trimmed_upper.contains("ADD_SUGGESTION_TOOL") ||
            trimmed_upper.contains("ADD_SUGGESTION_TEXT") ||
            trimmed_upper.starts_with("ADD_SUGGESTION_") ||
            (trimmed_upper.starts_with("ADD_SWITCHER") && trimmed_upper.contains(" AS ")) ||
            trimmed_upper.starts_with("ADD_MEMBER") ||
            (trimmed_upper.starts_with("CLEAR_SWITCHERS") && trimmed.contains('(')) ||
            (trimmed_upper.starts_with("USE_") && trimmed.contains('(')) {
            result.push_str(line);
            if !trimmed.ends_with(';') && !trimmed.ends_with('{') && !trimmed.ends_with('}') {
                result.push(';');
            }
            result.push('\n');
            continue;
        }

        // Bare `GET <drive path | url>` needs a dedicated rewrite: the generic
        // machinery derives the function name from the pattern and appends an
        // extra \s+ separator. Runtime override context: the `GET $expr$`
        // custom syntax loses to the later-registered GET SHAREPOINT … syntax
        // (same Rhai first-token key). NOTE: the Rust regex crate has no
        // lookahead support, so the GET-family protection (GET FROM/HTTP/
        // QUEUE/SHAREPOINT/metrics …) is checked in code below, not in the
        // pattern; protected lines fall through to the generic loop.
        if let Some(caps) = crate::re_cache::cached_re!(r#"(?i)^(\s*)(.*?)\bGET\s+(.+?)\s*;?$"#)
            .and_then(|re| re.captures(line))
        {
            let arg = caps.get(3).map_or("", |m| m.as_str().trim());
            let first_word = arg
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_uppercase();
            let protected = matches!(
                first_word.as_str(),
                "FROM" | "HTTP" | "BOT" | "QUEUE" | "ATTENDANT" | "ATTENDANTS" | "TIPS"
                    | "SMART" | "SUMMARY" | "CUSTOMER" | "INSTAGRAM" | "FACEBOOK"
                    | "LINKEDIN" | "TWITTER" | "SHAREPOINT" | "STOCK" | "BANCO"
                    | "UNMATCHED"
            );
            if !protected {
                let indent = caps.get(1).map_or("", |m| m.as_str());
                let prefix = caps.get(2).map_or("", |m| m.as_str());
                let arg = strip_trailing_stmt_semicolon(arg);
                result.push_str(&format!("{indent}{prefix}get_file({arg});\n"));
                continue;
            }
        }

        // Check for multiword keywords in ANY position (not just start of line)
        for (pattern, min_params, max_params, _param_names) in &multiword_patterns {
            // Use a regex that captures prefix (indent + any text before keyword) and params after keyword
            // Zero-param keywords (e.g. CLEAR KB) have no trailing params, so make the param group optional
            let regex_str = if *min_params == 0 {
                format!(
                    r#"(?i)^(\s*)(.*?)\b{}(?:\s+(.*))?\s*;?$"#,
                    pattern
                )
            } else {
                format!(
                    r#"(?i)^(\s*)(.*?)\b{}\s+(.*)$"#,
                    pattern
                )
            };

            if let Some(re) = crate::re_cache::dynamic(&regex_str) {
                if let Some(caps) = re.captures(line) {
                    if let Some(prefix_match) = caps.get(2) {
                        let indent = caps.get(1).map_or("", |m| m.as_str());
                        let prefix = prefix_match.as_str();
                        let mut params_str = caps.get(3).map_or("", |m| m.as_str().trim()).to_string();
                        // The .ast files written by BasicCompiler append a
                        // statement terminator ';' to every line. Strip it when
                        // it sits OUTSIDE quoted strings so it never leaks into
                        // the last function-call parameter (e.g. vibe_run("x";)).
                        params_str = strip_trailing_stmt_semicolon(&params_str);
                        let mut params = if params_str.is_empty() {
                            Vec::new()
                        } else {
                            parse_parameters(&params_str)
                        };
                        let mut param_count = params.len();

                        // Fallback: if comma-based parsing gave too few params, try splitting by '='
                        // e.g. SET BOT MEMORY "key" = value uses '=' not ','
                        if param_count < *min_params && params_str.contains('=') && !params_str.contains(','){
                            let eq_parts: Vec<&str> = params_str.splitn(2, '=').collect();
                            if eq_parts.len() == 2 {
                                let key = eq_parts[0].trim();
                                let value = eq_parts[1].trim();
                                let fallback_params = vec![key.to_string(), value.to_string()];
                                if fallback_params.len() >= *min_params && fallback_params.len() <= *max_params {
                                    params = fallback_params;
                                    param_count = params.len();
                                }
                            }
                        }

                        // Fallback: try splitting by ' AS ' (for ADD_SUGGESTION, ADD_SUGGESTION_TOOL, etc.)
                        if param_count < *min_params {
                            let as_upper = params_str.to_uppercase();
                            if let Some(pos) = as_upper.find(" AS ") {
                                let a = &params_str[..pos];
                                let b = &params_str[pos + 4..];
                                let fallback_params = vec![a.trim().to_string(), b.trim().to_string()];
                                if fallback_params.len() >= *min_params && fallback_params.len() <= *max_params {
                                    params = fallback_params;
                                    param_count = params.len();
                                }
                            }
                        }

                        if param_count >= *min_params && param_count <= *max_params {
                            let keyword = pattern.replace(r"\s+", "_").to_lowercase();

                            let output = if keyword == "add_switcher" {
                                let (switcher_id, label) = if params.len() == 2 {
                                    (params[0].clone(), params[1].clone())
                                } else if params.len() == 3 && params[1].eq_ignore_ascii_case("AS") {
                                    (params[0].clone(), params[2].clone())
                                } else {
                                    (params[0].clone(), params.last().cloned().unwrap_or_default())
                                };
                                let up_keyword = keyword.to_uppercase();
                                format!("{}{}{} {} as {};", indent, prefix, up_keyword, switcher_id, label)
                            } else if keyword.starts_with("add_suggestion") {
                                // ADD_SUGGESTION, ADD_SUGGESTION_TOOL, ADD_SUGGESTION_TEXT
                                // all use "AS" separator between param1 and param2 and are custom syntaxes, not functions
                                let (a, b) = if params.len() == 2 {
                                    (params[0].clone(), params[1].clone())
                                } else if params.len() >= 3 {
                                    let as_idx = params.iter().position(|p| p.eq_ignore_ascii_case("as")).unwrap_or(1);
                                    (params[0].clone(), params[as_idx + 1..].join(" "))
                                } else {
                                    (params[0].clone(), String::new())
                                };
                                let up_keyword = keyword.to_uppercase();
                                format!("{}{}{} {} as {};", indent, prefix, up_keyword, a, b)
                            } else {
                                let param_str = if params.is_empty() {
                                    String::new()
                                } else {
                                    params.join(", ")
                                };
                                format!("{}{}{}({});", indent, prefix, keyword, param_str)
                            };

                            result.push_str(&output);
                            result.push('\n');
                            converted = true;
                            break;
                        }
                    }
                }
            }
        }

        if !converted {
            let trimmed_line = line.trim();
            let is_comment = trimmed_line.starts_with('\'') || trimmed_line.starts_with('#') || trimmed_line.starts_with("//");
            let is_decl = trimmed_line.starts_with("BEGIN ") || trimmed_line.starts_with("END ") || trimmed_line.starts_with("TABLE ");
            let is_control = trimmed_line.starts_with("IF ") || trimmed_line.starts_with("ELSE") || trimmed_line.starts_with("END ") || trimmed_line.starts_with("NEXT") || trimmed_line.starts_with("LOOP") || trimmed_line.starts_with("FOR ") || trimmed_line.starts_with("WHILE") || trimmed_line.starts_with("SWITCH") || trimmed_line.starts_with("CASE") || trimmed_line.starts_with("DEFAULT") || trimmed_line.starts_with("DO") || trimmed_line.starts_with("UNTIL") || trimmed_line.starts_with("WEND");
            if is_comment || trimmed_line.is_empty() {
                // skip comment/blank lines entirely — Rhai doesn't understand ' as comment
            } else if is_decl || is_control {
                result.push_str(line);
            } else if trimmed_line.ends_with(';') || trimmed_line.ends_with('{') || (trimmed_line.ends_with('}') && !trimmed_line.contains("#{")) || trimmed_line.ends_with(':') {
                result.push_str(line);
            } else {
                result.push_str(line);
                result.push(';');
            }
            result.push('\n');
        }
    }

    result
}
