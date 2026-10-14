//! #1271 — Forgejo-backed source control for Vibe projects (`source_control =
//! "git"`).
//!
//! In git mode the project's source lives in a real ALM (Forgejo) repository
//! instead of only the local workspace. Project creation:
//!   1. creates the repo in Forgejo (`{org}/{repo}` derived from branch id +
//!      project name, exactly like the deployment router),
//!   2. initializes the workspace as a git checkout with the token-embedded
//!      `origin` remote,
//!   3. commits the seeded workspace and pushes `main`.
//! Deploy then snapshots the current release into a `release/deploy-<ts>`
//! branch (per-deploy rollback point) before committing the new state, and
//! the workspace stays in sync with Forgejo — the VM runs the pushed copy.
//!
//! The Forgejo URL + token come from Vault (`secret/gbo/alm`) via
//! `botcoresecrets::alm_config()`, the same single source the deployment
//! router uses — no per-bot LLM config is involved.

use crate::harness::cmd::run;
use crate::harness::ensure_workspace;
use crate::projects::Project;
use crate::vm_lifecycle::VmLifecycle;
use log::{info, warn};
use std::path::Path;

const GIT_USER_NAME: &str = "Vibe Agent";
const GIT_USER_EMAIL: &str = "vibe@gbo.local";

/// Configure the workspace as a Forgejo-backed checkout for a git-mode
/// project: create the repo, init the workspace with `origin` and push the
/// seeded state. Idempotent: a workspace that already has an `origin` remote
/// is left untouched (re-runs of creation / migration).
pub async fn ensure_git_repo(project: &Project) -> Result<(), String> {
    if project.source_control != "git" {
        return Ok(());
    }
    // Path-injection guard: restrict the user-controlled project name to the
    // workspace-safe charset before it reaches any filesystem join.
    let safe_name = crate::harness::sanitize_project_id(&project.name)?;
    let cwd = ensure_workspace(&safe_name)?;
    // Containment barrier: canonicalize and verify the workspace stays inside
    // the workspaces root (defense against path injection; symlink-aware).
    let root = crate::harness::workspace_root();
    crate::harness::ensure_contained(&cwd, &root)?;
    let remote = match run(
        "git",
        &["remote".to_string(), "get-url".to_string(), "origin".to_string()],
        &cwd,
        15,
    ) {
        Ok(out) if out.exit_code == Some(0) => Some(out.stdout.trim().to_string()),
        _ => None,
    };
    if let Some(url) = remote {
        info!("Vibe git-mode {}: origin already set ({url})", project.name);
        return Ok(());
    }

    let (alm_base, alm_token, _org) = botcoresecrets::alm_config();
    if alm_base.is_empty() || alm_token.is_empty() {
        return Err(format!(
            "git-mode project '{}' needs ALM config (vault secret/gbo/alm) — got base='{alm_base}' token={}",
            project.name,
            if alm_token.is_empty() { "empty" } else { "set" }
        ));
    }

    let org = VmLifecycle::alm_org(project.branch_id);
    let repo = VmLifecycle::alm_repo(&project.name);

    // 1. Create (or fetch existing) repo in Forgejo.
    let client = botdeployment::ForgejoClient::new(alm_base.clone(), alm_token.clone());
    let forgejo_repo = match client
        .create_repository(&org, &repo, &format!("Vibe project {}", project.name), false)
        .await
    {
        Ok(r) => r,
        Err(e) => return Err(format!("git-mode create Forgejo repo {org}/{repo}: {e}")),
    };

    // The API returns the public clone URL (not resolvable from inside the
    // bot container); rebuild it from the configured internal base, keeping
    // the path (`/{org}/{repo}.git`).
    let fallback_path = format!("{org}/{repo}.git");
    let path = forgejo_repo
        .clone_url.split_once("://").map(|x| x.1)
        .and_then(|rest| rest.split_once('/').map(|x| x.1))
        .unwrap_or(&fallback_path)
        .trim_start_matches('/');
    let internal_url = format!("{}/{path}", alm_base.trim_end_matches('/'));
    let auth_url = add_token_to_url(&internal_url, &alm_token);

    // 2. Init the workspace checkout on `main`.
    if !cwd.join(".git").exists() {
        git(&cwd, &["init", "-b", "main"], "init")?;
    }
    git(&cwd, &["remote", "add", "origin", &auth_url], "remote add origin")?;

    // 3. Commit the seeded state and push `main`. Nothing to commit on
    // re-runs is fine (no-op success).
    git(&cwd, &["add", "-A"], "add")?;
    let commit = git(
        &cwd,
        &[
            "-c", &format!("user.name={GIT_USER_NAME}"),
            "-c", &format!("user.email={GIT_USER_EMAIL}"),
            "commit", "-m", &format!("Initial commit: {}", project.name),
        ],
        "commit",
    );
    match commit {
        Ok(_) => {}
        Err(e) if e.contains("nothing to commit") => {}
        Err(e) => return Err(e),
    }
    match run("git", &["push".to_string(), "-u".to_string(), "origin".to_string(), "main".to_string()], &cwd, 120) {
        Ok(out) if out.exit_code == Some(0) => {}
        Ok(out) => {
            let msg = format!(
                "git-mode push {org}/{repo} failed: {}",
                out.stderr.trim()
            );
            warn!("Vibe git-mode {}: {msg}", project.name);
            // A repo that was just created and force-pushed over may need
            // --force when the remote's auto-init README conflicts.
            match run("git", &["push".to_string(), "--force".to_string(), "-u".to_string(), "origin".to_string(), "main".to_string()], &cwd, 120) {
                Ok(force) if force.exit_code == Some(0) => {}
                Ok(force) => return Err(format!("git-mode push failed: {}", force.stderr.trim())),
                Err(e) => return Err(e.to_string()),
            }
        }
        Err(e) => return Err(e.to_string()),
    }
    info!("Vibe git-mode {}: pushed main to {org}/{repo}", project.name);
    Ok(())
}

/// Clone an external repository into the workspace for a `github`-mode
/// project (`source_control = "github"`). The clone URL comes from
/// `project.payload.clone_url`. Idempotent: an existing `.git` checkout is
/// left untouched (re-runs of creation).
pub async fn ensure_github_clone(project: &Project) -> Result<(), String> {
    if project.source_control != "github" {
        return Ok(());
    }
    // Path-injection guard: the project name arrives from the REST body and
    // becomes a filesystem path below (temp clone dir + rename target).
    // Restrict it to the same charset `ensure_workspace` will accept so the
    // join can never traverse outside the workspaces root.
    let safe_name = crate::harness::sanitize_project_id(&project.name)?;
    let cwd = ensure_workspace(&safe_name)?;
    // Containment barrier for the github-clone flow (temp dir + rename below).
    let root = crate::harness::workspace_root();
    crate::harness::ensure_contained(&cwd, &root)?;
    if cwd.join(".git").exists() {
        info!("Vibe github-mode {}: workspace already cloned", project.name);
        return Ok(());
    }
    let clone_url = project
        .payload
        .get("clone_url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if clone_url.is_empty() {
        return Err(format!(
            "github-mode project '{}' needs payload.clone_url",
            project.name
        ));
    }
    // The workspace dir exists (ensure_workspace created it). `git clone`
    // requires an empty target directory, so clone into a temp sibling and
    // move the contents into place, then wire the origin remote (the clone
    // already carries it).
    let parent = cwd
        .parent()
        .ok_or_else(|| "workspace has no parent dir".to_string())?;
    let tmp = parent.join(format!(".clone-{safe_name}"));
    crate::harness::ensure_contained(&tmp, &root)?;
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp).map_err(|e| format!("clean temp clone: {e}"))?;
    }
    let owned = vec![
        "clone".to_string(),
        "--quiet".to_string(),
        clone_url.clone(),
        tmp.to_string_lossy().to_string(),
    ];
    match run("git", &owned, parent, 180) {
        Ok(out) if out.exit_code == Some(0) => {}
        Ok(out) => {
            return Err(format!(
                "git clone {} failed: {}",
                clone_url,
                out.stderr.trim()
            ))
        }
        Err(e) => return Err(format!("git clone: {e}")),
    }
    // Move the cloned checkout into the workspace (hidden dotfiles included).
    for entry in std::fs::read_dir(&tmp).map_err(|e| format!("read temp clone: {e}"))? {
        let entry = entry.map_err(|e| format!("read temp clone entry: {e}"))?;
        let from = entry.path();
        let to = cwd.join(entry.file_name());
        crate::harness::ensure_contained(&to, &cwd)?;
        std::fs::rename(&from, &to).map_err(|e| format!("move {} into workspace: {e}", entry.file_name().to_string_lossy()))?;
    }
    std::fs::remove_dir_all(&tmp).map_err(|e| format!("remove temp clone: {e}"))?;
    // Reset origin to the canonical (token-free) URL the caller supplied.
    match run(
        "git",
        &["remote".to_string(), "set-url".to_string(), "origin".to_string(), clone_url.clone()],
        &cwd,
        15,
    ) {
        Ok(out) if out.exit_code == Some(0) => {}
        Ok(out) => warn!(
            "Vibe github-mode {}: reset origin failed: {}",
            project.name,
            out.stderr.trim()
        ),
        Err(e) => warn!("Vibe github-mode {}: reset origin: {e}", project.name),
    }
    info!(
        "Vibe github-mode {}: cloned {} into workspace",
        project.name, clone_url
    );
    Ok(())
}

/// Create a per-deploy snapshot branch `release/deploy-<ts>` pointing at the
/// current HEAD (the deployed version being replaced), without checking it
/// out — the working tree and running app stay put. Returns the branch name
/// or `None` when there is nothing committed yet.
pub fn snapshot_deploy_branch(project: &Project) -> Result<Option<String>, String> {
    if project.source_control != "git" {
        return Ok(None);
    }
    // Path-injection guard (same as ensure_git_repo/ensure_github_clone).
    let cwd = ensure_workspace(&crate::harness::sanitize_project_id(&project.name)?)?;
    if !cwd.join(".git").exists() {
        return Ok(None);
    }
    let head = match run(
        "git",
        &["rev-parse".to_string(), "HEAD".to_string()],
        &cwd,
        15,
    ) {
        Ok(out) if out.exit_code == Some(0) => out.stdout.trim().to_string(),
        _ => String::new(),
    };
    if head.is_empty() {
        return Ok(None);
    }
    let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let branch = format!("release/deploy-{ts}");
    let created = run(
        "git",
        &["branch".to_string(), branch.clone(), head.clone()],
        &cwd,
        15,
    )
    .map(|out| out.exit_code == Some(0))
    .unwrap_or(false);
    if created {
        info!("Vibe git-mode {}: snapshot branch {branch}", project.name);
        Ok(Some(branch))
    } else {
        // Branch with that exact timestamp already exists; it is fine.
        Ok(None)
    }
}

/// Rewrite a clone URL to embed the token (`token@host`) like the deployment
/// router does, so `git push` authenticates over http(s) without prompting.
fn add_token_to_url(url: &str, token: &str) -> String {
    match url.split_once("://") {
        Some((scheme, rest)) => format!("{scheme}://{token}@{rest}"),
        None => url.to_string(),
    }
}

/// Run a git command in the workspace, returning stdout on success or a
/// formatted error on failure.
fn git(cwd: &Path, args: &[&str], what: &str) -> Result<String, String> {
    let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    match run("git", &owned, cwd, 60) {
        Ok(out) if out.exit_code == Some(0) => Ok(out.stdout.trim().to_string()),
        Ok(out) => {
            let combined = format!("{} {}", out.stdout, out.stderr);
            Err(format!("git {what} failed: {}", combined.trim()))
        }
        Err(e) => Err(format!("git {what}: {e}")),
    }
}
