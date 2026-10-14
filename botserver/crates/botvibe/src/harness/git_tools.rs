//! #747 — Git tooling for the Vibe agent: status, log, diff and commit
//! against the project workspace checkout.

use crate::harness::cmd::run;
use crate::harness::ensure_workspace;
use crate::tool_executor::ToolHandler;
use crate::types::{VibeState, VibeToolResult};
use serde_json::json;
use std::sync::Arc;

const GIT_USER_NAME: &str = "Vibe Agent";
const GIT_USER_EMAIL: &str = "vibe@gbo.local";

fn ok(data: serde_json::Value) -> VibeToolResult {
    VibeToolResult { success: true, data, error: None, latency_ms: 0 }
}

fn err(msg: String) -> VibeToolResult {
    VibeToolResult { success: false, data: serde_json::Value::Null, error: Some(msg), latency_ms: 0 }
}

fn str_arg(args: &serde_json::Value, key: &str) -> String {
    args.get(key).and_then(|v| v.as_str()).unwrap_or_default().to_string()
}

fn git_run(project: &str, git_args: &[String], timeout: u64) -> VibeToolResult {
    let cwd = match ensure_workspace(project) {
        Ok(p) => p,
        Err(e) => return err(e),
    };
    match run("git", git_args, &cwd, timeout) {
        Ok(out) if out.exit_code == Some(0) => ok(json!({
            "exit_code": out.exit_code,
            "output": out.stdout,
            "stderr": out.stderr,
        })),
        Ok(out) => err(format!("git {} failed (exit {}): {}", git_args.join(" "),
            out.exit_code.unwrap_or(-1), out.stderr.trim())),
        Err(e) => err(e.to_string()),
    }
}

/// `git/status` — porcelain status of the project workspace.
pub fn git_status() -> ToolHandler {
    Arc::new(move |args: serde_json::Value, _state: &dyn VibeState| {
        let args = args.clone();
        Box::pin(async move {
            let project = str_arg(&args, "project");
            git_run(&project, &["status".to_string(), "--porcelain=v1".to_string()], 30)
        })
    })
}

/// `git/log` — last N commits (default 20).
pub fn git_log_tool() -> ToolHandler {
    Arc::new(move |args: serde_json::Value, _state: &dyn VibeState| {
        let args = args.clone();
        Box::pin(async move {
            let project = str_arg(&args, "project");
            let n = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20).clamp(1, 100);
            git_run(&project, &["log".to_string(), "--oneline".to_string(), format!("-n{n}")], 30)
        })
    })
}

/// `git/diff` — working tree diff, optionally cached (staged).
pub fn git_diff_tool() -> ToolHandler {
    Arc::new(move |args: serde_json::Value, _state: &dyn VibeState| {
        let args = args.clone();
        Box::pin(async move {
            let project = str_arg(&args, "project");
            let cached = args.get("cached").and_then(|v| v.as_bool()).unwrap_or(false);
            let flags = if cached {
                vec!["diff".to_string(), "--cached".to_string()]
            } else {
                vec!["diff".to_string()]
            };
            git_run(&project, &flags, 300)
        })
    })
}

/// `git/commit` — stage all changes and commit with the given message.
pub fn git_commit_tool() -> ToolHandler {
    Arc::new(move |args: serde_json::Value, _state: &dyn VibeState| {
        let args = args.clone();
        Box::pin(async move {
            let project = str_arg(&args, "project");
            let message = str_arg(&args, "message");
            if message.trim().is_empty() {
                return err("commit message is required".into());
            }
            let cwd = match ensure_workspace(&project) {
                Ok(p) => p,
                Err(e) => return err(e),
            };
            // Auto-initialize the repository when absent so the deploy
            // pipeline's commit_push stage (which runs git/commit directly
            // without a prior git/init) works on fresh projects instead of
            // failing with "not a git repository".
            if !cwd.join(".git").exists() {
                // Branch naming standard: `main`, never `master` (git >= 2.28
                // supports -b; dev/prod containers ship git 2.4x).
                if let Ok(out) = run("git", &["init".to_string(), "-b".to_string(), "main".to_string()], &cwd, 30) {
                    if out.exit_code != Some(0) {
                        return err(format!("git init failed: {}", out.stderr.trim()));
                    }
                }
            }
            match run("git", &["add".to_string(), "-A".to_string()], &cwd, 30) {
                Ok(out) if out.exit_code == Some(0) => {}
                Ok(out) => return err(format!("git add failed: {}", out.stderr.trim())),
                Err(e) => return err(e.to_string()),
            }
            // `-c` config must precede the subcommand; the harness runs
            // processes with env_clear(), so git has no user identity and the
            // commit would otherwise fail with "Please tell me who you are".
            let commit_args = vec![
                "-c".to_string(), format!("user.name={GIT_USER_NAME}"),
                "-c".to_string(), format!("user.email={GIT_USER_EMAIL}"),
                "commit".to_string(),
                "-m".to_string(),
                message.clone(),
            ];
            // Deploy pipelines re-run against the same workspace: a tree
            // that is already committed has nothing to stage, so both the
            // `git add` ("nothing to commit") and `git commit` ("nothing to
            // commit, working tree clean") no-op paths must succeed instead
            // of failing the whole deploy run. Git may print these to
            // stdout or stderr depending on version, so both are checked.
            let clean_tree = |out: &crate::harness::cmd::RunOutput| {
                let combined = format!("{} {}", out.stdout, out.stderr);
                combined.contains("nothing to commit")
                    || combined.contains("no changes added to commit")
                    || combined.contains("nothing added to commit")
            };
            // #1271 — git-mode workspaces (Forgejo-backed, origin remote set)
            // push every commit so the branch + commit land in ALM and the VM
            // can sync from the repo. Native workspaces (no origin) stay
            // local-only. Push failure is reported, not silent.
            // The deploy pipeline's publish stage adds a CI/CD workflow commit
            // to origin/main AFTER this push, so a subsequent deploy would
            // otherwise be rejected ("fetch first"). Rebase onto the remote
            // tip before pushing so the workspace stays in sync with Forgejo.
            // #1271 — github-mode projects clone an EXTERNAL repository (the
            // caller's GitHub repo). That remote is read-only for us (no
            // GitHub credentials), so `git push origin` fails with "could not
            // read Username" and aborts the whole deploy pipeline before
            // publish. The deploy publishes from the WORKSPACE, not from the
            // GitHub remote, so the push must be skipped: the commit stays
            // local, and the pipeline proceeds to publish. Only git-mode
            // workspaces (origin = internal ALM/Forgejo with an embedded
            // token) actually push.
            fn origin_is_external(cwd: &std::path::Path) -> bool {
                let url = match run(
                    "git",
                    &["remote".to_string(), "get-url".to_string(), "origin".to_string()],
                    cwd,
                    15,
                ) {
                    Ok(out) if out.exit_code == Some(0) => out.stdout.trim().to_string(),
                    _ => return false,
                };
                let (alm_base, _, _) = botcoresecrets::alm_config();
                // No ALM configured -> there is no internal remote to push to
                // (git-mode projects require ALM config to exist), so any
                // origin is external by definition. Skip the push rather than
                // attempt an unauthenticated push to a foreign remote.
                if alm_base.is_empty() {
                    return true;
                }
                let alm_host = alm_base
                    .trim_end_matches('/')
                    .split("://")
                    .nth(1)
                    .unwrap_or(&alm_base)
                    .split(['/', ':'])
                    .next()
                    .unwrap_or("")
                    .to_string();
                if alm_host.is_empty() {
                    return true;
                }
                // The internal remote embeds the ALM host; an external
                // (github) remote points elsewhere.
                !url.contains(&alm_host)
            }

            fn push_origin(cwd: &std::path::Path) -> VibeToolResult {
                if origin_is_external(cwd) {
                    return ok(json!({
                        "pushed": false,
                        "external": true,
                        "output": "external origin — push skipped (commit is local-only)",
                    }));
                }
                // Sync with the remote tip first (best-effort; a missing
                // origin/main is fine — it just means no remote history yet).
                match run("git", &["fetch".to_string(), "origin".to_string()], cwd, 60) {
                    Ok(out) if out.exit_code == Some(0) => {
                        let has_origin_main = run(
                            "git",
                            &[
                                "rev-parse".to_string(),
                                "--verify".to_string(),
                                "origin/main".to_string(),
                            ],
                            cwd,
                            15,
                        )
                        .map(|r| r.exit_code == Some(0))
                        .unwrap_or(false);
                        if has_origin_main {
                            // A rebase replays commits, so it needs a committer
                            // identity just like `git commit` — and the harness
                            // runs processes with env_clear(), leaving git with
                            // none. Pass it via `-c` (must precede the
                            // subcommand) or a second deploy after a fresh edit
                            // fails with "Committer identity unknown".
                            match run(
                                "git",
                                &[
                                    "-c".to_string(),
                                    format!("user.name={GIT_USER_NAME}"),
                                    "-c".to_string(),
                                    format!("user.email={GIT_USER_EMAIL}"),
                                    "rebase".to_string(),
                                    "origin/main".to_string(),
                                ],
                                cwd,
                                90,
                            ) {
                                Ok(reb) if reb.exit_code == Some(0) => {}
                                Ok(reb) => {
                                    let _ = run(
                                        "git",
                                        &["rebase".to_string(), "--abort".to_string()],
                                        cwd,
                                        15,
                                    );
                                    return err(format!(
                                        "git rebase onto origin/main failed: {}",
                                        reb.stderr.trim()
                                    ));
                                }
                                Err(e) => return err(e.to_string()),
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(e) => return err(e.to_string()),
                }
                match run("git", &["push".to_string(), "origin".to_string()], cwd, 120) {
                    Ok(out) if out.exit_code == Some(0) => ok(json!({ "pushed": true })),
                    Ok(out) => {
                        let combined = format!("{} {}", out.stdout, out.stderr);
                        if combined.contains("Everything up-to-date") {
                            ok(json!({ "pushed": true, "output": "everything up-to-date" }))
                        } else {
                            err(format!("git push origin failed: {}", out.stderr.trim()))
                        }
                    }
                    Err(e) => err(e.to_string()),
                }
            }
            match run("git", &commit_args, &cwd, 30) {
                Ok(out) if out.exit_code == Some(0) => {
                    let has_origin = run(
                        "git",
                        &["remote".to_string(), "get-url".to_string(), "origin".to_string()],
                        &cwd,
                        15,
                    )
                    .map(|r| r.exit_code == Some(0))
                    .unwrap_or(false);
                    if has_origin {
                        // push_origin returns a VibeToolResult directly
                        // (success=false + error carries push failures).
                        let mut pushed = push_origin(&cwd);
                        if pushed.success {
                            pushed.data["message"] = json!(message);
                            pushed.data["hash"] =
                                json!(out.stdout.lines().next().unwrap_or("").trim());
                            pushed.data["output"] = json!(out.stdout);
                        }
                        pushed
                    } else {
                        ok(json!({
                            "message": message,
                            "hash": out.stdout.lines().next().unwrap_or("").trim(),
                            "output": out.stdout,
                        }))
                    }
                }
                Ok(out) if clean_tree(&out) => ok(json!({
                    "message": message,
                    "hash": "",
                    "output": "nothing to commit — workspace already clean",
                    "noop": true,
                })),
                Ok(out) => err(format!("git commit failed: {}", out.stderr.trim())),
                Err(e) => err(e.to_string()),
            }
        })
    })
}

/// `git/snapshot-previous` — before a deploy publishes the current state,
/// snapshot the currently deployed commit into a `release/prev-<ts>` branch so
/// the user can switch back from the toolbar branch combo and re-deploy the
/// previous version. Runs `git branch release/prev-<ts> <current-HEAD>`
/// without checking out, so the working tree (and the running app) stays put.
pub fn git_snapshot_previous_tool() -> ToolHandler {
    Arc::new(move |args: serde_json::Value, _state: &dyn VibeState| {
        let args = args.clone();
        Box::pin(async move {
            let project = str_arg(&args, "project");
            let cwd = match ensure_workspace(&project) {
                Ok(p) => p,
                Err(e) => return err(e),
            };
            if !cwd.join(".git").exists() {
                // Branch naming standard: `main`, never `master`.
                match run("git", &["init".to_string(), "-b".to_string(), "main".to_string()], &cwd, 30) {
                    Ok(out) if out.exit_code == Some(0) => {}
                    Ok(out) => return err(format!("git init failed: {}", out.stderr.trim())),
                    Err(e) => return err(format!("git init failed: {e}")),
                }
            }
            // Current HEAD; "previous" = the commit that is about to be
            // replaced by this deploy (the currently deployed version).
            let head = match run("git", &["rev-parse".to_string(), "HEAD".to_string()], &cwd, 15) {
                Ok(out) if out.exit_code == Some(0) => out.stdout.trim().to_string(),
                _ => String::new(),
            };
            if head.is_empty() {
                return ok(json!({
                    "snapshot": false,
                    "reason": "no commits yet — nothing to snapshot",
                    "branch": null,
                }));
            }
            let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
            let branch = format!("release/prev-{ts}");
            let existing = run(
                "git",
                &["rev-parse".to_string(), "--verify".to_string(), format!("refs/heads/{branch}")],
                &cwd,
                15,
            )
            .map(|out| out.exit_code == Some(0))
            .unwrap_or(false);
            let args_vec = if existing {
                vec!["branch".to_string(), "-f".to_string(), branch.clone(), head.clone()]
            } else {
                vec!["branch".to_string(), branch.clone(), head.clone()]
            };
            match run("git", &args_vec, &cwd, 30) {
                Ok(out) if out.exit_code == Some(0) => {
                    // #1271 — git-mode workspaces push the snapshot branch to
                    // origin so the per-deploy rollback branch persists in
                    // ALM/Forgejo (the Branch combo and VM sync read remote
                    // refs too). Push is best-effort; failure is reported in
                    // the output, not fatal — the local branch is enough for
                    // the toolbar's branch combo.
                    let mut pushed = false;
                    let has_origin = run(
                        "git",
                        &["remote".to_string(), "get-url".to_string(), "origin".to_string()],
                        &cwd,
                        15,
                    )
                    .map(|r| r.exit_code == Some(0))
                    .unwrap_or(false);
                    if has_origin {
                        if let Ok(p) = run(
                            "git",
                            &["push".to_string(), "origin".to_string(), branch.clone().to_string()],
                            &cwd,
                            60,
                        ) {
                            pushed = p.exit_code == Some(0);
                        }
                    }
                    ok(json!({
                        "snapshot": true,
                        "branch": branch,
                        "from": head,
                        "output": out.stdout.trim(),
                        "pushed": pushed,
                    }))
                }
                Ok(out) => err(format!("git branch snapshot failed: {}", out.stderr.trim())),
                Err(e) => err(e.to_string()),
            }
        })
    })
}

/// `git/init` — initialize a git repository if absent, and clone support.
pub fn git_init_tool() -> ToolHandler {
    Arc::new(move |args: serde_json::Value, _state: &dyn VibeState| {
        let args = args.clone();
        Box::pin(async move {
            let project = str_arg(&args, "project");
            let cwd = match ensure_workspace(&project) {
                Ok(p) => p,
                Err(e) => return err(e),
            };
            if cwd.join(".git").exists() {
                return ok(json!({"already_initialized": true}));
            }
            let clone_url = str_arg(&args, "clone_url");
            if clone_url.is_empty() {
                // Branch naming standard: `main`, never `master`.
                match run("git", &["init".to_string(), "-b".to_string(), "main".to_string()], &cwd, 30) {
                    Ok(out) if out.exit_code == Some(0) => {
                        ok(json!({"initialized": true, "output": out.stdout}))
                    }
                    Ok(out) => err(format!("git init failed: {}", out.stderr.trim())),
                    Err(e) => err(e.to_string()),
                }
            } else {
                match run("git", &["clone".to_string(), clone_url, ".".to_string()], &cwd, 300) {
                    Ok(out) if out.exit_code == Some(0) => {
                        ok(json!({"cloned": true, "output": out.stdout}))
                    }
                    Ok(out) => err(format!("git clone failed: {}", out.stderr.trim())),
                    Err(e) => err(e.to_string()),
                }
            }
        })
    })
}