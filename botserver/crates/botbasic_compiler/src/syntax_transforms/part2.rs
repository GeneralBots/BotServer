use super::*;

pub fn convert_if_then_syntax(script: &str) -> String {
    let mut result = String::new();
    let mut if_stack: Vec<bool> = Vec::new();
    let mut while_depth: usize = 0;
    let mut in_with_block = false;
    let mut in_line_continuation = false;

    log::trace!("Converting IF/THEN syntax, input has {} lines", script.lines().count());

    for line in script.lines() {
        let trimmed = line.trim();
        let upper = trimmed.to_uppercase();

        if trimmed.is_empty() || trimmed.starts_with('\'') || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }

        if trimmed.starts_with("while ") && trimmed.ends_with('{') {
            while_depth += 1;
            result.push_str(trimmed);
            result.push('\n');
            continue;
        }
        if trimmed == "}" && while_depth > 0 && if_stack.is_empty() {
            while_depth -= 1;
            result.push_str("}\n");
            continue;
        }

        if upper.starts_with("IF ") && upper.contains(" THEN") {
            let then_pos = match upper.find(" THEN") {
                Some(pos) => pos,
                None => continue,
            };
            let condition = &trimmed[3..then_pos].trim();
            // BASIC dialects test the last runtime error with a bare `ERROR`
            // condition (paired with ON ERROR RESUME NEXT). The engine exposes
            // it as the error_flag() function, not a variable, so rewrite it
            // before the generic = → == pass would mangle it.
            let condition = if condition.trim().eq_ignore_ascii_case("ERROR") {
                "error_flag()".to_string()
            } else {
                condition
                    .replace(" NOT IN ", " !in ").replace(" not in ", " !in ")
            };
            let condition = if condition == "error_flag()" {
                condition
            } else {
                condition.replace(" AND ", " && ").replace(" and ", " && ")
                    .replace(" OR ", " || ").replace(" or ", " || ")
            };
            let condition = if !condition.contains("==") && !condition.contains("!=")
                && !condition.contains("<=") && !condition.contains(">=")
                && !condition.contains("+=") && !condition.contains("-=")
                && !condition.contains("*=") && !condition.contains("/=") {
                condition.replace("=", "==")
            } else {
                condition.to_string()
            };
            log::trace!("Converting IF statement: condition='{}'", condition);

            // Handle inline IF/THEN with body on the same line
            // e.g., IF cond THEN stmt END IF  or  IF cond THEN stmt ELSE stmt2 END IF
            let after_then = trimmed[then_pos + 5..].trim();
            let has_inline_else = after_then.to_uppercase().contains(" ELSE ");
            let has_inline_end = after_then.to_uppercase().contains(" END IF");

            if has_inline_end || has_inline_else {
                // Extract inline THEN body (everything before ELSE or END IF)
                let inline_body = if has_inline_else {
                    let else_upper = after_then.to_uppercase();
                    let else_pos = match else_upper.find(" ELSE ") { Some(p) => p, None => continue };
                    after_then[..else_pos].trim().to_string()
                } else if has_inline_end {
                    let end_upper = after_then.to_uppercase();
                    let end_pos = match end_upper.find(" END IF") { Some(p) => p, None => continue };
                    after_then[..end_pos].trim().to_string()
                } else {
                    after_then.to_string()
                };

                result.push_str("if ");
                result.push_str(&condition);
                result.push_str(" {\n  ");
                result.push_str(&inline_body);
                result.push_str(";\n");

                if has_inline_else {
                    let else_upper = after_then.to_uppercase();
                    let else_pos = match else_upper.find(" ELSE ") { Some(p) => p, None => continue };
                    let after_else = after_then[else_pos + 6..].trim();
                    let else_body = if after_else.to_uppercase().ends_with(" END IF") {
                        // `to_uppercase` can change a character's byte length, so the
                        // guard above says nothing about where `len() - 7` lands.
                        botlib::text::truncate_at_char_boundary(
                            after_else,
                            after_else.len().saturating_sub(7),
                        )
                        .trim()
                        .to_string()
                    } else {
                        after_else.trim_end().to_string()
                    };
                    result.push_str("} else {\n  ");
                    result.push_str(&else_body);
                    result.push_str(";\n}\n");
                } else {
                    result.push_str("}\n");
                }
                // Don't push to if_stack since it's single-line and already closed
                continue;
            }

            result.push_str("if ");
            result.push_str(&condition);
            result.push_str(" {\n");
            if_stack.push(true);
            continue;
        }

        if upper.starts_with("ELSE IF ") && upper.contains(" THEN") {
            let then_pos = match upper.find(" THEN") {
                Some(pos) => pos,
                None => continue,
            };
            let condition = &trimmed[7..then_pos].trim();
            let condition = condition.replace(" NOT IN ", " !in ").replace(" not in ", " !in ");
            let condition = condition.replace(" AND ", " && ").replace(" and ", " && ")
                .replace(" OR ", " || ").replace(" or ", " || ");
            let condition = if !condition.contains("==") && !condition.contains("!=")
                && !condition.contains("<=") && !condition.contains(">=")
                && !condition.contains("+=") && !condition.contains("-=")
                && !condition.contains("*=") && !condition.contains("/=") {
                condition.replace("=", "==")
            } else {
                condition.to_string()
            };
            log::trace!("Converting ELSE IF statement: condition='{}'", condition);
            result.push_str("} else if ");
            result.push_str(&condition);
            result.push_str(" {\n");
            continue;
        }

        if upper == "ELSE" {
            log::trace!("Converting ELSE statement");
            result.push_str("} else {\n");
            continue;
        }

        if upper.starts_with("ELSEIF ") && upper.contains(" THEN") {
            let then_pos = match upper.find(" THEN") {
                Some(pos) => pos,
                None => continue,
            };
            let condition = &trimmed[6..then_pos].trim();
            let condition = condition.replace(" NOT IN ", " !in ").replace(" not in ", " !in ");
            let condition = condition.replace(" AND ", " && ").replace(" and ", " && ")
                .replace(" OR ", " || ").replace(" or ", " || ");
            let condition = if !condition.contains("==") && !condition.contains("!=")
                && !condition.contains("<=") && !condition.contains(">=")
                && !condition.contains("+=") && !condition.contains("-=")
                && !condition.contains("*=") && !condition.contains("/=") {
                condition.replace("=", "==")
            } else {
                condition.to_string()
            };
            log::trace!("Converting ELSEIF statement: condition='{}'", condition);
            result.push_str("} else if ");
            result.push_str(&condition);
            result.push_str(" {\n");
            continue;
        }

        if upper == "END IF" {
            log::trace!("Converting END IF statement");
            if if_stack.pop().is_some() {
                result.push_str("}\n");
            }
            continue;
        }

        if upper.starts_with("WITH ") {
            let object_name = &trimmed[5..].trim();
            log::trace!("Converting WITH statement: object='{}'", object_name);
            result.push_str("let ");
            result.push_str(object_name);
            result.push_str(" = #{\n");
            in_with_block = true;
            continue;
        }

        if upper == "END WITH" {
            log::trace!("Converting END WITH statement");
            result.push_str("};\n");
            in_with_block = false;
            continue;
        }

        if in_with_block {
            if trimmed.contains('=') && !trimmed.contains("==") && !trimmed.contains("!=") && !trimmed.contains("+=") && !trimmed.contains("-=") {
                let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let property_name = parts[0].trim();
                    let property_value = parts[1].trim();
                    let property_value = property_value.trim_end_matches(';');
                    result.push_str(&format!(" {}: {},\n", property_name, property_value));
                    continue;
                }
            }
            result.push_str("  ");
        }

        if upper.starts_with("SAVE") && upper.contains(',') {
            log::trace!("Processing SAVE line: '{}'", trimmed);
            let after_save = &trimmed[4..].trim();
            let parts: Vec<&str> = after_save.split(',').collect();
            log::trace!("SAVE parts: {:?}", parts);

            if parts.len() >= 2 {
                let table = parts[0].trim().trim_matches('"');
                let values = parts[1..].join(",");
                // Unified 2-arg SAVE: SAVE "table", data
                // Rhai handler checks if data has an `id` field to decide insert vs upsert
                let converted = format!("SAVE \"{}\", {};\n", table, values);
                log::trace!("Unified SAVE syntax: '{}'", converted);
                result.push_str(&converted);
                continue;
            }
        }

        if upper.starts_with("SEND EMAIL") {
            log::trace!("Processing SEND EMAIL line: '{}'", trimmed);
            let after_send = &trimmed[11..].trim();
            let parts: Vec<&str> = after_send.split(',').collect();
            log::trace!("SEND EMAIL parts: {:?}", parts);
            if parts.len() == 3 {
                let to = parts[0].trim();
                let subject = parts[1].trim();
                let body = parts[2].trim().trim_end_matches(';');
                let converted = format!("send_mail({}, {}, {}, []);\n", to, subject, body);
                log::trace!("Converted SEND EMAIL to: '{}'", converted);
                result.push_str(&converted);
                continue;
            }
        }

        if !if_stack.is_empty() {
            result.push_str("  ");
        }

        if !upper.starts_with("IF ") && !upper.starts_with("ELSE") && !upper.starts_with("END IF") {
            // Assignment detection must ignore `=` characters that live inside
            // string literals, otherwise keyword calls like
            // `create_file(p, "category=" + c)` are misread as assignments and
            // get a spurious `let ` prefix (invalid Rhai).
            let code_only = strip_string_literals(trimmed);
            let is_var_assignment = trimmed.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_')
                && code_only.contains('=')
                && !code_only.contains("==")
                && !code_only.contains("!=")
                && !code_only.contains("<=")
                && !code_only.contains(">=")
                && !code_only.contains("+=")
                && !code_only.contains("-=")
                && !code_only.contains("*=")
                && !code_only.contains("/=");

            let ends_with_comma = trimmed.ends_with(',');

            let line_to_process = if in_line_continuation && !is_var_assignment
                && !trimmed.contains('=') && !trimmed.starts_with('"') && !upper.starts_with("IF ") {
                let escaped = trimmed.replace('"', "\\\"");
                format!("\"{}\\n\"", escaped)
            } else {
                trimmed.to_string()
            };

            if is_var_assignment {
                let trimmed_lower = trimmed.to_lowercase();
                let in_block = !if_stack.is_empty() || while_depth > 0;
                if !in_block && !trimmed_lower.starts_with("let ") {
                    let first_word = trimmed_lower.split_whitespace().next().unwrap_or("");
                    let is_statement_keyword = matches!(first_word,
                        "if" | "else" | "while" | "for" | "update" | "save" | "insert"
                        | "delete" | "select" | "merge" | "talk" | "print" | "return"
                        | "switch" | "match" | "throw" | "import" | "export" | "const"
                    );
                    if !is_statement_keyword {
                        result.push_str("let ");
                    }
                }
            }
            result.push_str(&line_to_process);
            let is_keyword_stmt = upper.starts_with("INSERT ")
                || upper.starts_with("SAVE ")
                || upper.starts_with("TALK ")
                || upper.starts_with("PRINT ")
                || upper.starts_with("MERGE ")
                || upper.starts_with("UPDATE ");
            let ends_with_block_brace = trimmed.ends_with('}') && !is_keyword_stmt && !trimmed.contains("#{");
            let needs_semicolon = !trimmed.ends_with(';')
                && !trimmed.ends_with('{')
                && !ends_with_block_brace
                && !upper.starts_with("SELECT ")
                && !upper.starts_with("CASE ")
                && upper != "END SELECT"
                && !upper.starts_with("WHILE ")
                && !upper.starts_with("WEND")
                && !ends_with_comma
                && !in_line_continuation;
            if needs_semicolon {
                result.push(';');
            }
            result.push('\n');

            in_line_continuation = ends_with_comma;
        } else {
            result.push_str(trimmed);
            result.push('\n');
        }
    }

    log::trace!("IF/THEN conversion complete, output has {} lines", result.lines().count());

    result.replace(" <> ", " != ")
}
