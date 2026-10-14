//! #747 — File tools for the Vibe agent. Writes are confined to the
//! project workspace (`VIBE_WORKSPACE_ROOT/{project}/`).

use crate::harness::{list_rel, read_rel_file, write_rel_file};
use crate::tool_executor::ToolHandler;
use crate::types::{VibeState, VibeToolResult};
use serde_json::json;
use std::sync::Arc;

fn ok(data: serde_json::Value) -> VibeToolResult {
    VibeToolResult {
        success: true,
        data,
        error: None,
        latency_ms: 0,
    }
}

fn err(msg: String) -> VibeToolResult {
    VibeToolResult {
        success: false,
        data: serde_json::Value::Null,
        error: Some(msg),
        latency_ms: 0,
    }
}

fn str_arg(args: &serde_json::Value, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn required_str_arg(args: &serde_json::Value, key: &str) -> Result<String, String> {
    let value = args
        .get(key)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("required argument '{key}' must be a non-empty string"))?;
    if key == "path" && matches!(value, "..." | "…") {
        return Err("required argument 'path' must name a real workspace file".to_string());
    }
    Ok(value.to_string())
}

fn replace_content(
    content: &str,
    old: &str,
    new: &str,
    replace_all: bool,
) -> Result<(String, usize), String> {
    if old.contains(':') != new.contains(':') {
        return Err(
            "old and new text cover different syntactic scopes; replace a value with a value or a declaration with a declaration"
                .to_string(),
        );
    }
    let compact_old = compact_syntax_whitespace(old);
    let target = if content.contains(old) || compact_old == old {
        old
    } else {
        compact_old.as_str()
    };
    let occurrences = content.matches(target).count();
    if occurrences == 0 {
        return Err("text to replace was not found".to_string());
    }
    if !replace_all && occurrences > 1 {
        return Err(format!(
            "text to replace is ambiguous ({occurrences} matches); include more surrounding context or set all=true"
        ));
    }
    if replace_all {
        Ok((content.replace(target, new), occurrences))
    } else {
        Ok((content.replacen(target, new, 1), 1))
    }
}

fn compact_syntax_whitespace(value: &str) -> String {
    fn is_punctuation(ch: char) -> bool {
        matches!(ch, '{' | '}' | '[' | ']' | '(' | ')' | ';' | ',' | ':')
    }

    let mut compact = String::with_capacity(value.len());
    let mut pending_space = false;
    for ch in value.trim().chars() {
        if ch.is_whitespace() {
            pending_space = !compact.is_empty();
            continue;
        }
        if is_punctuation(ch) {
            if compact.ends_with(' ') {
                compact.pop();
            }
            compact.push(ch);
            pending_space = false;
            continue;
        }
        if pending_space
            && compact
                .chars()
                .last()
                .is_some_and(|last| !is_punctuation(last))
        {
            compact.push(' ');
        }
        compact.push(ch);
        pending_space = false;
    }
    compact
}

fn normalized_html_title(raw: &str) -> String {
    let without_markers = raw
        .replace("<newline>", " ")
        .replace("<NEWLINE>", " ")
        .replace("<space>", " ")
        .replace("<SPACE>", " ");
    let normalized = without_markers
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    normalized
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// `file/read` — read a workspace file (1 MiB cap) as text.
pub fn file_read() -> ToolHandler {
    Arc::new(move |args: serde_json::Value, _state: &dyn VibeState| {
        let args = args.clone();
        Box::pin(async move {
            let project = str_arg(&args, "project");
            let path = str_arg(&args, "path");
            match crate::harness::read_rel_file(&project, &path, 1024 * 1024) {
                Ok(bytes) => ok(json!({
                    "project": project,
                    "path": path,
                    "size": bytes.len(),
                    "content": String::from_utf8_lossy(&bytes),
                })),
                Err(e) => err(e),
            }
        })
    })
}

/// `file/write` — write a workspace file (4 MiB cap, path confined).
pub fn file_write() -> ToolHandler {
    Arc::new(move |args: serde_json::Value, _state: &dyn VibeState| {
        let args = args.clone();
        Box::pin(async move {
            let project = str_arg(&args, "project");
            let path = str_arg(&args, "path");
            let content = str_arg(&args, "content");
            log::info!(
                "file_write called project={project} path={path} len={}",
                content.len()
            );
            match write_rel_file(&project, &path, content.as_bytes()) {
                Ok(()) => {
                    log::info!("file_write OK project={project} path={path}");
                    ok(json!({"project": project, "path": path, "bytes": content.len()}))
                }
                Err(e) => {
                    log::info!("file_write ERR project={project} path={path}: {e}");
                    err(e)
                }
            }
        })
    })
}

/// `file/replace` — replace exact text in an existing workspace file.
/// This is safer and substantially easier for small local models than
/// reproducing an entire source file in a `file/write` call.
pub fn file_replace() -> ToolHandler {
    Arc::new(move |args: serde_json::Value, _state: &dyn VibeState| {
        let args = args.clone();
        Box::pin(async move {
            let project = match required_str_arg(&args, "project") {
                Ok(value) => value,
                Err(e) => return err(e),
            };
            let path = match required_str_arg(&args, "path") {
                Ok(value) => value,
                Err(e) => return err(e),
            };
            let old = match required_str_arg(&args, "old") {
                Ok(value) => value,
                Err(e) => return err(e),
            };
            let new = match args.get("new").and_then(|value| value.as_str()) {
                Some(value) => value.to_string(),
                None => return err("required argument 'new' must be a string".to_string()),
            };
            let replace_all = args
                .get("all")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);

            let bytes = match read_rel_file(&project, &path, 1024 * 1024) {
                Ok(value) => value,
                Err(e) => return err(e),
            };
            let content = String::from_utf8_lossy(&bytes).into_owned();
            let (updated, replacements) = match replace_content(&content, &old, &new, replace_all) {
                Ok(value) => value,
                Err(e) => return err(format!("{e} in '{path}'")),
            };
            if let Err(e) = write_rel_file(&project, &path, updated.as_bytes()) {
                return err(e);
            }
            ok(json!({
                "project": project,
                "path": path,
                "replacements": replacements,
            }))
        })
    })
}

/// `file/set-title` — replace the first HTML document title found in the
/// project's source files without rewriting unrelated content.
pub fn file_set_title() -> ToolHandler {
    Arc::new(move |args: serde_json::Value, _state: &dyn VibeState| {
        let args = args.clone();
        Box::pin(async move {
            let project = str_arg(&args, "project");
            let title = normalized_html_title(&str_arg(&args, "title"));
            if title.is_empty() {
                return err("title is required".to_string());
            }
            let entries = match list_rel(&project, "", 0) {
                Ok(entries) => entries,
                Err(e) => return err(e),
            };
            let mut changed = Vec::new();
            for path in entries {
                if path.ends_with('/') {
                    continue;
                }
                let extension = std::path::Path::new(&path)
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if !matches!(
                    extension.as_str(),
                    "html" | "htm" | "js" | "jsx" | "ts" | "tsx" | "vue" | "svelte"
                ) {
                    continue;
                }
                let bytes = match crate::harness::read_rel_file(&project, &path, 1024 * 1024) {
                    Ok(bytes) => bytes,
                    Err(_) => continue,
                };
                let mut content = String::from_utf8_lossy(&bytes).into_owned();
                let lower = content.to_ascii_lowercase();
                let Some(open) = lower.find("<title>") else {
                    continue;
                };
                let value_start = open + "<title>".len();
                let Some(relative_end) = lower[value_start..].find("</title>") else {
                    continue;
                };
                let value_end = value_start + relative_end;
                content.replace_range(value_start..value_end, &title);
                if let Err(e) = write_rel_file(&project, &path, content.as_bytes()) {
                    return err(e);
                }
                changed.push(path);
            }
            if changed.is_empty() {
                err("no HTML <title> element found in the project workspace".to_string())
            } else {
                ok(json!({"project": project, "title": title, "changed_files": changed}))
            }
        })
    })
}

/// `file/list` — list entries under a workspace path.
pub fn file_list() -> ToolHandler {
    Arc::new(move |args: serde_json::Value, _state: &dyn VibeState| {
        let args = args.clone();
        Box::pin(async move {
            let project = str_arg(&args, "project");
            let path = str_arg(&args, "path");
            match list_rel(&project, &path, 0) {
                Ok(mut entries) => {
                    entries.sort();
                    ok(
                        json!({"project": project, "path": path, "count": entries.len(), "entries": entries}),
                    )
                }
                Err(e) => err(e),
            }
        })
    })
}

/// `file/delete` — delete a workspace file.
pub fn file_delete() -> ToolHandler {
    Arc::new(move |args: serde_json::Value, _state: &dyn VibeState| {
        let args = args.clone();
        Box::pin(async move {
            let project = str_arg(&args, "project");
            let path = str_arg(&args, "path");
            match crate::harness::resolve_workspace_path(&project, &path) {
                Ok(p) => match std::fs::remove_file(&p) {
                    Ok(()) => ok(json!({"project": project, "path": path, "deleted": true})),
                    Err(e) => err(format!("delete {}: {}", path, e)),
                },
                Err(e) => err(e),
            }
        })
    })
}

/// `file/exists` — probe a workspace path.
pub fn file_exists() -> ToolHandler {
    Arc::new(move |args: serde_json::Value, _state: &dyn VibeState| {
        let args = args.clone();
        Box::pin(async move {
            let project = str_arg(&args, "project");
            let path = str_arg(&args, "path");
            match crate::harness::resolve_workspace_path(&project, &path) {
                Ok(p) => ok(json!({"exists": p.exists(), "is_dir": p.is_dir()})),
                Err(e) => err(e),
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::{
        compact_syntax_whitespace, normalized_html_title, replace_content, required_str_arg,
    };

    #[test]
    fn title_normalization_removes_local_model_format_markers() {
        assert_eq!(
            normalized_html_title("<newline><space>XCalculator"),
            "XCalculator"
        );
    }

    #[test]
    fn title_normalization_escapes_html() {
        assert_eq!(
            normalized_html_title("R&D <tools>"),
            "R&amp;D &lt;tools&gt;"
        );
    }

    #[test]
    fn focused_replace_changes_one_or_all_occurrences() {
        let (one, count) =
            replace_content("color:red background:red", "color:red", "color:blue", false)
                .expect("replace");
        assert_eq!(one, "color:blue background:red");
        assert_eq!(count, 1);

        let (all, count) = replace_content("red red", "red", "blue", true).expect("replace");
        assert_eq!(all, "blue blue");
        assert_eq!(count, 2);
        assert!(replace_content("red red", "red", "blue", false).is_err());
        assert!(replace_content("red", "green", "blue", false).is_err());

        let (css, count) = replace_content(
            "button{background:#84d669}",
            "background: #84d669",
            "background: blue",
            false,
        )
        .expect("compact CSS replacement");
        assert_eq!(css, "button{background: blue}");
        assert_eq!(count, 1);
        assert_eq!(
            compact_syntax_whitespace(" button { background: #84d669; } "),
            "button{background:#84d669;}"
        );
        assert!(replace_content(
            "button{background:#84d669}",
            "#84d669",
            "background:blue",
            false,
        )
        .is_err());
    }

    #[test]
    fn required_path_rejects_missing_and_placeholder_values() {
        assert!(required_str_arg(&serde_json::json!({}), "path").is_err());
        assert!(required_str_arg(&serde_json::json!({"path": "..."}), "path").is_err());
        assert_eq!(
            required_str_arg(&serde_json::json!({"path": "/index.js"}), "path").expect("path"),
            "/index.js"
        );
    }
}
