use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use botlib::security::SafeCommand;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

use botcore::shared::state::AppState;

const DEFAULT_REPO: &str = "/tmp/git-repo";

/// Root that hosts every vibe project workspace (mirrors
/// `botvibe::harness::workspace_root()` so the Source Control dialog reads
/// the SAME repo the Vibe agent edits). `VIBE_WORKSPACE_ROOT` wins when set;
/// otherwise the platform stack data directory is used.
fn vibe_workspace_root() -> PathBuf {
    std::env::var("VIBE_WORKSPACE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(botlib::security::get_stack_path())
                .join("data")
                .join("vibe-workspaces")
        })
}

#[derive(Deserialize)]
pub struct GitQuery {
    pub repo: Option<String>,
}

#[derive(Deserialize)]
pub struct GitStatusQuery {
    pub repo: Option<String>,
}

#[derive(Deserialize)]
pub struct TreeQuery {
    pub repo: Option<String>,
    pub path: Option<String>,
}

#[derive(Serialize)]
pub struct GitStatusResponse {
    pub repo: String,
    pub branch: Option<String>,
    pub files: Vec<GitFileStatus>,
}

#[derive(Serialize)]
pub struct GitFileStatus {
    pub file: String,
    pub status: String,
}

#[derive(Deserialize)]
pub struct CommitRequest {
    pub message: String,
    pub repo: Option<String>,
    /// #1506 — optional pathspec list: commit ONLY these files (GitHub-style
    /// per-file staging). Empty/absent commits everything (legacy behavior).
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Serialize)]
pub struct GitTreeEntry {
    pub path: String,
    pub is_dir: bool,
    /// Status letter for changed files: "M" modified, "U" untracked,
    /// "A" added, "D" deleted, "R" renamed; empty when unchanged.
    pub status: String,
}

#[derive(Serialize)]
pub struct GitLogResponse {
    pub repo: String,
    pub commits: Vec<GitLogEntry>,
}

#[derive(Serialize)]
pub struct GitLogEntry {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub date: String,
}

#[derive(Serialize)]
pub struct GitBranch {
    pub name: String,
    pub current: bool,
}

fn resolve_repo(query_repo: Option<&str>) -> String {
    let repo = query_repo
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .unwrap_or(DEFAULT_REPO);
    if repo.starts_with("/tmp") || repo.starts_with("/var/tmp") {
        return repo.to_string();
    }
    // Absolute paths point at an explicit git checkout.
    if repo.starts_with('/') {
        return repo.to_string();
    }
    // A bare name is a vibe project: resolve it against the workspace root
    // (e.g. `e2e-gh-4404` → `/opt/gbo/data/vibe-workspaces/e2e-gh-4404`).
    // Only accept plain project ids — no separators, no traversal.
    if !repo.is_empty()
        && repo != "."
        && repo != ".."
        && !repo.contains('/')
        && !repo.contains('\\')
        && repo.len() <= 128
    {
        let candidate = vibe_workspace_root().join(repo);
        if candidate.join(".git").exists() {
            return candidate.to_string_lossy().into_owned();
        }
    }
    DEFAULT_REPO.to_string()
}

fn run_git(repo: &str, args: &[&str]) -> Result<String, String> {
    let mut cmd = SafeCommand::new("git").map_err(|e| e.to_string())?;
    cmd = cmd.arg("-C")?;
    cmd = cmd.trusted_arg(repo)?;
    for a in args {
        cmd = cmd.trusted_arg(a)?;
    }
    let out = cmd.execute().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() {
        return Err(if stderr.is_empty() { stdout } else { stderr });
    }
    Ok(stdout)
}

pub async fn git_status(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<GitStatusQuery>,
) -> Result<Json<GitStatusResponse>, axum::http::StatusCode> {
    let repo = resolve_repo(params.repo.as_deref());
    let branch_out = run_git(&repo, &["rev-parse", "--abbrev-ref", "HEAD"]);
    let branch = branch_out.ok();
    let status_out = match run_git(&repo, &["status", "--porcelain=v1"]) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("git status failed for {repo}: {e}");
            String::new()
        }
    };
    let files = status_out.lines().filter_map(|line| {
        if line.len() < 3 {
            return None;
        }
        let code = &line[..2];
        let file = line[3..].trim();
        let status = match code {
            " M" | "M " | "MM" => "modified",
            "??" => "untracked",
            "A" => "added",
            "D" | " D" => "deleted",
            "R" => "renamed",
            _ => "changed",
        };
        Some(GitFileStatus {
            file: file.to_string(),
            status: status.to_string(),
        })
    }).collect();
    Ok(Json(GitStatusResponse {
        repo,
        branch,
        files,
    }))
}

pub async fn git_diff(
    State(_state): State<Arc<AppState>>,
    Path(file): Path<String>,
    Query(params): Query<GitQuery>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let repo = resolve_repo(params.repo.as_deref());
    let out = run_git(&repo, &["diff", "HEAD", "--", &file]);
    let diff = match out {
        Ok(d) => d,
        Err(_) => run_git(&repo, &["diff", "--cached", "--", &file]).unwrap_or_default(),
    };
    Ok(Json(serde_json::json!({ "file": file, "diff": diff })))
}

pub async fn git_commit(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<CommitRequest>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let repo = resolve_repo(payload.repo.as_deref());
    // #1506 — stage the selected files when a pathspec list is given, else
    // persist everything (legacy single-panel behavior).
    let add = if payload.files.is_empty() {
        run_git(&repo, &["add", "-A"])
    } else {
        let mut args: Vec<&str> = vec!["add", "--"];
        for f in &payload.files {
            args.push(f);
        }
        run_git(&repo, &args)
    };
    let mut results = serde_json::Map::new();
    results.insert("add".into(), serde_json::Value::String(add.unwrap_or_else(|e| e)));
    let message = if payload.message.trim().is_empty() {
        "wip".to_string()
    } else {
        payload.message.trim().to_string()
    };
    // Project workspaces are not configured with a git identity (the agent
    // harness passes `-c user.*` for its own commits), so committing without
    // one fails with "Please tell me who you are". Supply the same identity
    // the Vibe agent uses so the Source Control dialog commit always works.
    let commit = run_git(
        &repo,
        &[
            "-c",
            "user.name=Vibe Agent",
            "-c",
            "user.email=vibe@gbo.local",
            "commit",
            "-m",
            &message,
        ],
    );
    match commit {
        Ok(out) => {
            results.insert("status".into(), serde_json::Value::String("success".into()));
            results.insert("output".into(), serde_json::Value::String(out.trim().to_string()));
        }
        Err(e) => {
            results.insert("status".into(), serde_json::Value::String("failure".into()));
            results.insert("error".into(), serde_json::Value::String(e));
        }
    }
    Ok(Json(serde_json::Value::Object(results)))
}

pub async fn git_push(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<GitQuery>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let repo = resolve_repo(params.repo.as_deref());
    let out = run_git(&repo, &["push", "origin", "HEAD"]);
    match out {
        Ok(_) => Ok(Json(serde_json::json!({ "status": "success", "message": "Pushed to origin/HEAD" }))),
        Err(e) => Ok(Json(serde_json::json!({ "status": "failure", "error": e }))),
    }
}

pub async fn git_branches(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<GitQuery>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let repo = resolve_repo(params.repo.as_deref());
    let out = run_git(&repo, &["branch", "--format=%(refname:short)"]);
    let current = run_git(&repo, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_default();
    let branches: Vec<GitBranch> = out
        .unwrap_or_default()
        .lines()
        .map(|b| b.trim().to_string())
        .filter(|b| !b.is_empty())
        .map(|b| GitBranch {
            name: b.clone(),
            current: b == current.trim(),
        })
        .collect();
    Ok(Json(serde_json::json!({ "branches": branches })))
}

pub async fn git_create_or_switch_branch(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(params): Query<GitQuery>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let repo = resolve_repo(params.repo.as_deref());
    let exists = run_git(&repo, &["rev-parse", "--verify", &format!("refs/heads/{name}")]);
    let cmd: &[&str] = if exists.is_ok() {
        &["checkout", &name]
    } else {
        &["checkout", "-B", &name]
    };
    let out = run_git(&repo, cmd);
    match out {
        Ok(msg) => Ok(Json(serde_json::json!({ "status": "success", "branch": name, "output": msg.trim() }))),
        Err(e) => Ok(Json(serde_json::json!({ "status": "failure", "error": e }))),
    }
}

pub async fn git_log(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<GitQuery>,
) -> Result<Json<GitLogResponse>, axum::http::StatusCode> {
    let repo = resolve_repo(params.repo.as_deref());
    let out = run_git(
        &repo,
        &["log", "-20", "--pretty=format:%H%x00%s%x00%an%x00%ad", "--date=short"],
    );
    let commits: Vec<GitLogEntry> = out
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(4, '\x00');
            let hash = parts.next()?.to_string();
            let message = parts.next().unwrap_or("").to_string();
            let author = parts.next().unwrap_or("").to_string();
            let date = parts.next().unwrap_or("").to_string();
            Some(GitLogEntry { hash, message, author, date })
        })
        .collect();
    Ok(Json(GitLogResponse { repo, commits }))
}

/// #1506 — list a repo subtree with per-entry git status, feeding the
/// GitHub-style tree view. One call returns the whole subtree so the editor
/// renders folders + files without N status round-trips.
/// Query: `?repo=&path=` (path empty = repo root).
pub async fn git_tree(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<TreeQuery>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let repo = resolve_repo(params.repo.as_deref());
    let dir = params.path.as_deref().unwrap_or("").trim().trim_matches('/');
    // Pathspec safety: reject traversal before it reaches `git ls-files`.
    if dir.split('/').any(|seg| seg == ".." || seg.is_empty() && !dir.is_empty()) {
        return Ok(Json(serde_json::json!({ "entries": [], "error": "invalid path" })));
    }
    let mut entries: Vec<GitTreeEntry> = Vec::new();
    // `git ls-files` rejects an empty pathspec argument — omit it entirely
    // for the repo root instead of passing "".
    let mut ls_args: Vec<&str> = vec!["ls-files", "--cached", "--others", "--exclude-standard", "-z"];
    if !dir.is_empty() {
        ls_args.push(dir);
    }
    let listing = run_git(&repo, &ls_args).unwrap_or_default();
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let status_out = run_git(&repo, &["status", "--porcelain=v1", "-z"]).unwrap_or_default();
    let mut status_by_path: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for record in status_out.split('\0').filter(|r| r.len() > 3) {
        let code = record[..2].trim().to_string();
        let path = record[3..].trim().to_string();
        let letter = match code.as_str() {
            "M" | "MM" | "AM" => "M",
            "??" => "U",
            "A" => "A",
            "D" => "D",
            "R" => "R",
            _ => "M",
        };
        status_by_path.insert(path, letter.to_string());
    }
    for path in listing.split('\0').filter(|p| !p.is_empty()) {
        let rel = path.trim().to_string();
        if rel.is_empty() {
            continue;
        }
        let rest = if dir
            .is_empty() { rel.clone() } else { {
                rel.strip_prefix(&format!("{dir}/")).unwrap_or(&rel).to_string()
            } };
        if rest.is_empty() {
            continue;
        }
        if let Some(idx) = rest.find('/') {
            // Direct child directory of the requested path.
            let dir_name = rest[..idx].to_string();
            let dir_path = if dir.is_empty() { dir_name.clone() } else { format!("{dir}/{dir_name}") };
            if seen.insert(format!("d:{dir_path}")) {
                entries.push(GitTreeEntry {
                    path: dir_path,
                    is_dir: true,
                    status: String::new(),
                });
            }
        } else {
            let full = if dir.is_empty() { rest.clone() } else { format!("{dir}/{rest}") };
            if seen.insert(format!("f:{full}")) {
                entries.push(GitTreeEntry {
                    status: status_by_path.get(&full).cloned().unwrap_or_default(),
                    path: full,
                    is_dir: false,
                });
            }
        }
    }
    // Deleted files no longer appear in ls-files — surface them from status.
    for (path, letter) in &status_by_path {
        if letter == "D" {
            let in_scope = dir.is_empty() || path.starts_with(&format!("{dir}/"));
            if in_scope && seen.insert(format!("f:{path}")) {
                entries.push(GitTreeEntry {
                    path: path.clone(),
                    is_dir: false,
                    status: letter.clone(),
                });
            }
        }
    }
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.path.cmp(&b.path)));
    Ok(Json(serde_json::json!({ "entries": entries })))
}

/// #1506 — pull the current branch from origin (editor toolbar button);
/// fast-forward only so a confused checkout never loses local work silently.
pub async fn git_pull(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<GitQuery>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let repo = resolve_repo(params.repo.as_deref());
    match run_git(&repo, &["pull", "--ff-only", "origin"])
    {
        Ok(out) => Ok(Json(serde_json::json!({ "status": "success", "output": out.trim() }))),
        Err(e) => Ok(Json(serde_json::json!({ "status": "failure", "error": e }))),
    }
}