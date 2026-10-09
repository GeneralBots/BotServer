use super::*;

pub fn predeclare_variables(script: &str) -> String {
    let reserved: std::collections::HashSet<&str> = [
        "if", "else", "while", "for", "loop", "return", "break", "continue",
        "let", "fn", "true", "false", "in", "do", "match", "switch", "case",
        "mod", "and", "or", "not", "rem", "call", "talk", "hear", "save",
        "insert", "update", "delete", "find", "get", "set", "print",
    ].iter().cloned().collect();

    let mut vars: BTreeSet<String> = BTreeSet::new();

    for line in script.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") || t.starts_with('\'') || t.starts_with('#') {
            continue;
        }
        if let Some(eq_pos) = t.find('=') {
            let after_char = t.as_bytes().get(eq_pos + 1).copied();
            let prev_char = if eq_pos > 0 { t.as_bytes().get(eq_pos - 1).copied() } else { None };
            if after_char == Some(b'=') { continue; }
            if matches!(prev_char, Some(b'!') | Some(b'<') | Some(b'>') | Some(b'+') | Some(b'-') | Some(b'*') | Some(b'/')) { continue; }
            let before = &t[..eq_pos];
            let lhs = before.trim();
            if lhs.is_empty() || lhs.contains(' ') || lhs.contains('"') || lhs.contains('(') || lhs.contains('[') {
                continue;
            }
            if !lhs.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_') {
                continue;
            }
            if !lhs.chars().all(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let lower = lhs.to_lowercase();
            if reserved.contains(lower.as_str()) {
                continue;
            }
            vars.insert(lhs.to_string());
        }
    }

    if vars.is_empty() {
        return script.to_string();
    }

    let mut declarations = String::new();
    for v in &vars {
        declarations.push_str(&format!("let {};\n", v));
    }
    declarations.push('\n');
    declarations.push_str(script);
    declarations
}
/// Replace the contents of double-quoted string literals with spaces so
/// heuristics that look for `=`/`==`/`+=` operate on code, not on text inside
/// strings (e.g. `create_file(p, "category=" + c)` must not look like an
/// assignment just because a literal contains `=`).
pub(super) fn strip_string_literals(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_string = false;
    let mut escaped = false;
    for ch in line.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
                out.push('"');
            } else {
                out.push(' ');
            }
        } else {
            if ch == '"' {
                in_string = true;
            }
            out.push(ch);
        }
    }
    out
}
