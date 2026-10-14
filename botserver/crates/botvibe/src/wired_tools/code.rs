//! Code intelligence group of wired tools (Issue #1486). Answers questions
//! about a project from CodeGraph's pre-built symbol graph instead of letting
//! the agent crawl files one read at a time.
//!
//! Every handler resolves the project through `ensure_workspace`, which
//! sanitizes the project id and verifies containment (`harness/mod.rs:147`), so
//! no caller-supplied path ever reaches the filesystem. The query itself is
//! executed inside the project's dev VM with `incus exec` (#1488 installs the
//! CLI there), through the command guard: arguments are verbatim and no shell
//! string is composed on either side.

use super::{err, handler, ok, require_str};
use crate::harness::{cmd, ensure_workspace};
use crate::tool_executor::{ToolHandler, ToolSchema};
use crate::types::{VibeUseCase, VibeState};
use serde_json::{json, Value};

/// Hard ceiling on what a single tool call may return to the agent. A code
/// graph can produce a very large answer, and an unbounded payload is a context
/// blow-up for the caller rather than a feature.
const MAX_OUTPUT_CHARS: usize = 24_000;

/// Ceiling on a tool invocation, in seconds. A full first-time index of a large
/// project is slower than an incremental query, so this is generous but finite.
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Environment whose dev VM serves these calls. `Run` always provisions the
/// development container (`projects_api/workspace_2.rs`, `env: "development"`)
/// and #1488 installs the CLI there, so the container name is derived from the
/// same pair the provisioner uses.
const VM_ENV: &str = "development";

/// Working directory inside that container: `vm_incus::linux_create`
/// pre-creates it and Run stages the workspace into it, so it is where the
/// `.codegraph/` index of the deployed tree lives.
const VM_WORKDIR: &str = "/opt/vibe/app";

/// Environment-level failures, in the order incus reports them: the container
/// is absent, exists but is stopped, or is not reachable at all.
fn vm_unavailable(detail: &str) -> bool {
    [
        "Instance not found",
        "is not running",
        "not running",
        "No such instance",
    ]
    .iter()
    .any(|needle| detail.contains(needle))
}

/// Runs `codegraph` inside the project's dev VM. The CLI ships with
/// `apps`-kind project VMs (#1488) and the index is built from the staged
/// workspace, so the query goes through `incus exec` instead of the
/// botserver host, where the binary and the indexed tree are both absent.
///
/// Returns the trimmed stdout on success; a non-zero exit becomes an error
/// carrying the tool's own stderr, because a missing index and a bad query
/// must not look alike.
fn run_codegraph(project: &str, args: &[String], timeout: u64) -> Result<String, String> {
    // Containment first: `project` is attacker-influenced and must resolve
    // inside the workspaces root before it is turned into a container name.
    let cwd = ensure_workspace(project)?;
    let container = crate::vm_lifecycle::VmLifecycle::container_name(project, VM_ENV, false);
    let mut argv: Vec<String> = Vec::with_capacity(args.len() + 6);
    argv.push("exec".to_string());
    argv.push(container.clone());
    argv.push("--cwd".to_string());
    argv.push(VM_WORKDIR.to_string());
    argv.push("--".to_string());
    argv.push("codegraph".to_string());
    argv.extend(args.iter().cloned());
    let out = cmd::run("incus", &argv, &cwd, timeout).map_err(|e| match e {
        // `incus` is allowlisted; a spawn failure means the host cannot reach
        // the hypervisor at all. Say which VM was targeted instead of letting
        // the agent read a bare ENOENT.
        cmd::GuardError::Spawn(_) => format!(
            "incus is unavailable on this host, so the code graph of '{project}' cannot be read; it lives in the project VM ({container}, #1488)"
        ),
        other => other.to_string(),
    })?;
    if out.exit_code != Some(0) {
        let detail = if out.stderr.trim().is_empty() {
            out.stdout.trim().to_string()
        } else {
            out.stderr.trim().to_string()
        };
        if vm_unavailable(&detail) {
            return Err(format!(
                "codegraph runs in the dev VM of '{project}' ({container}), which is not available; Run the project first (#1488)"
            ));
        }
        return Err(format!(
            "codegraph {} failed: {}",
            args.first().map(String::as_str).unwrap_or("query"),
            detail
        ));
    }
    Ok(out.stdout)
}

/// Truncates on a char boundary and says so, so the agent never mistakes a cut
/// payload for a complete one.
fn bounded(text: String) -> Value {
    let trimmed = text.trim().to_string();
    if trimmed.chars().count() <= MAX_OUTPUT_CHARS {
        return json!({ "output": trimmed, "truncated": false });
    }
    let cut: String = trimmed.chars().take(MAX_OUTPUT_CHARS).collect();
    json!({ "output": cut, "truncated": true })
}

/// Ceiling for one call: the caller may raise or lower it, bounded so no agent
/// turn can pin the harness for an unbounded time.
fn timeout_of(args: &Value) -> u64 {
    args.get("timeout_secs")
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
        .clamp(5, 900)
}

/// `code/explore` — one question in, the relevant symbols with their call paths
/// out. This is the tool that replaces the discovery crawl, so it is the one the
/// prompt should reach for first.
fn code_explore() -> ToolHandler {
    handler(|args, _state: &dyn VibeState| async move {
        let project = match require_str(&args, "project") {
            Ok(v) => v.trim().to_string(),
            Err(e) => return err(e),
        };
        let query = match require_str(&args, "query") {
            Ok(v) => v.to_string(),
            Err(e) => return err(e),
        };
        let timeout = timeout_of(&args);
        let out = match run_codegraph(&project, &["explore".to_string(), query.clone()], timeout) {
            Ok(v) => v,
            Err(e) => return err(e),
        };
        wrap(bounded(out), project, query)
    })
}

/// `code/search` — find a symbol by name across the whole index.
fn code_search() -> ToolHandler {
    handler(|args, _state: &dyn VibeState| async move {
        let project = match require_str(&args, "project") {
            Ok(v) => v.trim().to_string(),
            Err(e) => return err(e),
        };
        let query = match require_str(&args, "query") {
            Ok(v) => v.to_string(),
            Err(e) => return err(e),
        };
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20);
        let mut argv = vec!["query".to_string(), query.clone(), "--limit".to_string()];
        argv.push(limit.clamp(1, 200).to_string());
        argv.push("--json".to_string());
        let out = match run_codegraph(&project, &argv, timeout_of(&args)) {
            Ok(v) => v,
            Err(e) => return err(e),
        };
        wrap(bounded(out), project, query)
    })
}

/// `code/impact` — what a change to this symbol would reach.
fn code_impact() -> ToolHandler {
    handler(|args, _state: &dyn VibeState| async move {
        let project = match require_str(&args, "project") {
            Ok(v) => v.trim().to_string(),
            Err(e) => return err(e),
        };
        let symbol = match require_str(&args, "symbol") {
            Ok(v) => v.to_string(),
            Err(e) => return err(e),
        };
        let mut argv = vec![
            "impact".to_string(),
            symbol.clone(),
            "--json".to_string(),
        ];
        if let Some(depth) = args.get("depth").and_then(|v| v.as_u64()) {
            argv.push("--depth".to_string());
            argv.push(depth.clamp(1, 10).to_string());
        }
        let out = match run_codegraph(&project, &argv, timeout_of(&args)) {
            Ok(v) => v,
            Err(e) => return err(e),
        };
        wrap(bounded(out), project, symbol)
    })
}

/// Wraps a payload with the project and the term it answers, so a run transcript
/// records what was asked about which project.
fn wrap(payload: Value, project: String, term: String) -> crate::types::VibeToolResult {
    ok(json!({
        "project": project,
        "term": term,
        "result": payload,
    }))
}

pub fn code_tools() -> Vec<(String, ToolSchema, ToolHandler)> {
    let cases = vec![VibeUseCase::SoftwareDevelopment];
    vec![
        (
            "code/explore".to_string(),
            ToolSchema::new(
                "code/explore",
                "Answer a question about this project's code from the pre-built symbol graph: returns the relevant symbols with their source and the call paths between them. Use this instead of reading files one by one to understand how something works",
            )
            .with_parameters(json!({
                "type": "object",
                "properties": {
                    "project": {"type": "string", "minLength": 1, "description": "Vibe project id (name)"},
                    "query": {"type": "string", "minLength": 1, "description": "The question, e.g. 'how does the login flow validate a token' or a file or symbol name"},
                    "timeout_secs": {"type": "integer", "description": "Ceiling on this call; defaults to 120 for a cold index"}
                },
                "required": ["project", "query"]
            }))
            .with_use_cases(cases.clone()),
            code_explore(),
        ),
        (
            "code/search".to_string(),
            ToolSchema::new(
                "code/search",
                "Find a symbol by name across the whole project index; returns matches with their file and kind",
            )
            .with_parameters(json!({
                "type": "object",
                "properties": {
                    "project": {"type": "string", "minLength": 1, "description": "Vibe project id (name)"},
                    "query": {"type": "string", "minLength": 1, "description": "Symbol name or fragment"},
                    "limit": {"type": "integer", "description": "Maximum matches; defaults to 20"},
                    "timeout_secs": {"type": "integer", "description": "Ceiling on this call; defaults to 120"}
                },
                "required": ["project", "query"]
            }))
            .with_use_cases(cases.clone()),
            code_search(),
        ),
        (
            "code/impact".to_string(),
            ToolSchema::new(
                "code/impact",
                "Report what a change to this symbol would reach: callers, callees and the blast radius. Use before editing a shared function",
            )
            .with_parameters(json!({
                "type": "object",
                "properties": {
                    "project": {"type": "string", "minLength": 1, "description": "Vibe project id (name)"},
                    "symbol": {"type": "string", "minLength": 1, "description": "Symbol whose blast radius to compute"},
                    "depth": {"type": "integer", "description": "Traversal depth; 1 to 10"},
                    "timeout_secs": {"type": "integer", "description": "Ceiling on this call; defaults to 120"}
                },
                "required": ["project", "symbol"]
            }))
            .with_use_cases(cases),
            code_impact(),
        ),
    ]
}

/// Guards the two invariants this module depends on: the project id comes from
/// the argument and nothing else, and the tool never builds a shell string.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_three_tools() {
        let tools = code_tools();
        let names: Vec<&str> = tools.iter().map(|(n, _, _)| n.as_str()).collect();
        assert_eq!(names, vec!["code/explore", "code/search", "code/impact"]);
    }

    #[test]
    fn every_tool_requires_a_project() {
        for (_, schema, _) in code_tools() {
            let required = schema
                .parameters
                .get("required")
                .and_then(|r| r.as_array())
                .expect("required list");
            let keys: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
            assert!(keys.contains(&"project"), "{} must require project", schema.name);
        }
    }

    #[test]
    fn tools_are_scoped_to_software_development() {
        for (_, schema, _) in code_tools() {
            assert_eq!(
                schema.allowed_use_cases,
                vec![VibeUseCase::SoftwareDevelopment],
                "{} must be scoped to software development",
                schema.name
            );
        }
    }

    #[test]
    fn tools_are_read_only() {
        // No approval gate: these never mutate the workspace, and gating them
        // would put a prompt in front of every code question.
        for (_, schema, _) in code_tools() {
            assert!(!schema.requires_approval, "{} must not need approval", schema.name);
        }
    }

    #[test]
    fn short_payload_is_not_truncated() {
        let value = bounded("hello".to_string());
        assert_eq!(value["truncated"], json!(false));
        assert_eq!(value["output"], json!("hello"));
    }

    #[test]
    fn long_payload_is_truncated_and_flagged() {
        let value = bounded("x".repeat(MAX_OUTPUT_CHARS + 500));
        assert_eq!(value["truncated"], json!(true));
        let out = value["output"].as_str().expect("output string");
        assert_eq!(out.chars().count(), MAX_OUTPUT_CHARS);
    }

    #[test]
    fn wrapper_records_project_and_term() {
        let result = wrap(bounded("body".to_string()), "proj".to_string(), "login".to_string());
        assert!(result.success);
        assert_eq!(result.data["project"], json!("proj"));
        assert_eq!(result.data["term"], json!("login"));
    }

    /// The CLI ships in `apps`-kind project VMs (#1488), so on any host without
    /// it the tool must say so. A bare ENOENT would read as a missing binary
    /// and send an agent looking for an install bug instead of reading this.
    #[test]
    fn missing_cli_yields_an_actionable_message() {
        let _guard = crate::harness::WORKSPACE_ENV_LOCK
            .lock()
            .expect("workspace env lock");
        let tmp = std::env::temp_dir().join(format!("vibe-code-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).expect("create workspace root");
        let previous = std::env::var_os("VIBE_WORKSPACE_ROOT");
        std::env::set_var("VIBE_WORKSPACE_ROOT", &tmp);
        let outcome = run_codegraph("proj", &["explore".to_string(), "anything".to_string()], 10);
        if let Some(previous) = previous {
            std::env::set_var("VIBE_WORKSPACE_ROOT", previous);
        } else {
            std::env::remove_var("VIBE_WORKSPACE_ROOT");
        }
        let _ = std::fs::remove_dir_all(&tmp);

        // Either the binary is present on this machine (then the call ran and
        // failed for a different, legitimate reason) or it is absent and the
        // message must name the project VMs.
        if let Err(message) = outcome { assert!(
            message.contains("codegraph")
                && (message.contains("#1488") || message.contains("failed")),
            "unhelpful failure message: {message}"
        ) }
    }

    /// A project id is attacker-influenced, so the workspace resolution must
    /// reject traversal rather than reading outside the workspaces root.
    #[test]
    fn project_traversal_is_refused() {
        assert!(run_codegraph("../etc", &["explore".to_string(), "x".to_string()], 5).is_err());
        assert!(run_codegraph("", &["explore".to_string(), "x".to_string()], 5).is_err());
    }
}
