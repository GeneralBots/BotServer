//! Reform #1502/#1504 — git-pull bot monitor (core sync primitives).
//!
//! Bot sources live in Forgejo repositories (one repo per project inside the
//! branch org, #1503) instead of Drive buckets. This module keeps a git
//! checkout of every bot-kind git-mode Vibe project fresh and materializes
//! the pulled sources into the standard work layout the runtime already reads
//! (`work/{branch}.gborg/{branch}.gbai/{bot}.gbdialog`) for BOTH environments:
//! `{bot}` (PROD) and `{bot}-test` (TEST twin, #1504).

use std::path::{Path, PathBuf};

use botcore::shared::utils::{get_work_path, DbPool};
use botlib::security::SafeCommand;
use diesel::prelude::*;

pub(crate) const MAX_MATERIALIZED_FILES: usize = 400;

/// A bot-kind git-mode project to monitor.
#[derive(Debug, Clone)]
pub(crate) struct MonitoredBot {
    pub project_id: uuid::Uuid,
    pub name: String,
    pub repo_slug: String,
    pub branch_slug: Option<String>,
}

pub(crate) fn run_git(cwd: &Path, args: &[&str]) -> Result<String, String> {
    let mut cmd = SafeCommand::new("git").map_err(|e| e.to_string())?;
    cmd = cmd.arg("-C").map_err(|e| e.to_string())?;
    cmd = cmd.trusted_arg(cwd.to_string_lossy().as_ref())?;
    for a in args {
        cmd = cmd.trusted_arg(a)?;
    }
    let out = cmd.execute().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() {
        return Err(if stderr.trim().is_empty() { stdout } else { stderr });
    }
    Ok(stdout)
}

pub(crate) fn alm_repo(name: &str) -> String {
    botvibe::vm_lifecycle::VmLifecycle::alm_repo(name)
}

/// Authenticated clone URL against the INTERNAL ALM base (same source
/// `git_mode.rs` uses so clones work inside the container).
pub(crate) fn authenticated_clone_url(org: &str, repo: &str) -> Option<String> {
    let (alm_base, alm_token, _) = botcoresecrets::alm_config();
    if alm_base.is_empty() || alm_token.is_empty() {
        return None;
    }
    let base = alm_base.trim_end_matches('/');
    Some(format!("{base}/{org}/{repo}.git").replace("://", &format!("://{}@", alm_token)))
}

pub(crate) fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let to = dst.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir_recursive(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}

/// Ensure a checkout exists at `vibe-workspaces/{repo}` — the SAME directory
/// the Vibe agent edits, so a git pull keeps the agent workspace and the
/// deployed sources one and the same.
pub(crate) fn ensure_checkout(project: &MonitoredBot, org: &str) -> Result<PathBuf, String> {
    let root = botvibe::harness::workspace_root();
    let cwd = root.join(&project.repo_slug);
    std::fs::create_dir_all(&cwd).map_err(|e| format!("mkdir {}: {e}", cwd.display()))?;
    if cwd.join(".git").exists() {
        return Ok(cwd);
    }
    let url = match authenticated_clone_url(org, &project.repo_slug) {
        Some(u) => u,
        None => {
            return Err("ALM config missing (secret/gbo/alm) — git monitor idle".to_string())
        }
    };
    let tmp = root.join(format!(".clone-{}", project.repo_slug));
    let _ = std::fs::remove_dir_all(&tmp);
    match run_git(&root, &["clone", "--quiet", &url, tmp.to_string_lossy().as_ref()]) {
        Ok(_) => {
            for entry in std::fs::read_dir(&tmp).map_err(|e| format!("read clone: {e}"))? {
                let entry = entry.map_err(|e| format!("clone entry: {e}"))?;
                let to = cwd.join(entry.file_name());
                if entry.path().is_dir() {
                    std::fs::rename(entry.path(), &to)
                        .or_else(|_| copy_dir_recursive(&entry.path(), &to))
                        .map_err(|e: std::io::Error| format!("move {e}"))?;
                } else {
                    std::fs::rename(entry.path(), &to).map_err(|e| format!("move {e}"))?;
                }
            }
            let _ = std::fs::remove_dir_all(&tmp);
            log::info!(
                "[git_monitor] cloned {org}/{} → {}",
                project.repo_slug,
                cwd.display()
            );
            Ok(cwd)
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&tmp);
            Err(format!("clone {org}/{}: {e}", project.repo_slug))
        }
    }
}

/// Self-heal for pre-reform projects (#1503 backfill): when the clone fails
/// because the Forgejo repo does not exist yet ("Repository not found"),
/// provision it from the currently-deployed PROD `.gbdialog` sources and
/// retry the clone once. Provisioning = seed workspace →
/// `git_mode::ensure_git_repo` (creates the Forgejo repo, commits, pushes
/// `main`). Retried on the next tick until it succeeds; never fatal.
pub(crate) fn ensure_checkout_with_heal(
    pool: &DbPool,
    project: &MonitoredBot,
    org: &str,
) -> Result<PathBuf, String> {
    match ensure_checkout(project, org) {
        Ok(cwd) => Ok(cwd),
        Err(clone_err) => {
            let not_found = clone_err.contains("not found")
                || clone_err.contains("does not appear to be a git repository");
            if !not_found {
                return Err(clone_err);
            }
            log::info!(
                "[git_monitor] {0}/{1} has no repo yet — provisioning from deployed sources",
                org, project.repo_slug
            );
            provision_repo(pool, project, org)?;
            ensure_checkout(project, org)
        }
    }
}

/// Builds the project workspace from the deployed PROD `.gbdialog` (falling
/// back to a minimal README for fresh bots) and wires the Forgejo repo,
/// origin remote and initial `main` push via `git_mode::ensure_git_repo`.
fn provision_repo(pool: &DbPool, project: &MonitoredBot, org: &str) -> Result<(), String> {
    let safe_name = botvibe::harness::sanitize_project_id(&project.name)?;
    let cwd = botvibe::harness::ensure_workspace(&safe_name)?;
    let registry = botvibe::projects::ProjectRegistry::new(pool.clone());
    let mut p = registry
        .get(project.project_id)
        .map_err(|e| format!("project load: {e}"))?
        .ok_or_else(|| format!("project {} vanished mid-heal", project.project_id))?;
    // Pre-reform rows may still carry source_control='native', which makes
    // ensure_git_repo a no-op — the reform owns bot sources, so flip the
    // project to git mode (persisted) BEFORE any early return: a partial
    // heal from a previous boot (workspace seeded, repo never created) must
    // not wedge the loop.
    if p.source_control != "git" {
        let mut conn = pool.get().map_err(|e| format!("pool: {e}"))?;
        diesel::sql_query(
            "UPDATE vibe_projects SET source_control = 'git', updated_at = now() WHERE id = $1",
        )
        .bind::<diesel::sql_types::Uuid, _>(project.project_id)
        .execute(&mut conn)
        .map_err(|e| format!("source_control flip: {e}"))?;
        drop(conn);
        log::info!(
            "[git_monitor] provision {0}/{1}: source_control → git (was '{2}')",
            org, project.repo_slug, p.source_control
        );
        p.source_control = "git".to_string();
    }
    // Seed from the deployed PROD sources when they exist (pre-reform bots
    // keep their content through the git move); fresh bots get a minimal
    // tree so the initial commit is never empty.
    let work_root = PathBuf::from(get_work_path());
    let branch_slug = project.branch_slug.clone().unwrap_or_default();
    let org_dialog = work_root.join(format!(
        "{0}.gborg/{0}.gbai/{1}.gbdialog",
        branch_slug, project.name
    ));
    let single_dialog = work_root.join(format!("{0}.gbai/{1}.gbdialog", branch_slug, project.name));
    let deployed = if org_dialog.is_dir() {
        Some(org_dialog)
    } else if single_dialog.is_dir() {
        Some(single_dialog)
    } else {
        None
    };
    match deployed {
        Some(src) => {
            copy_dir_recursive(&src, &cwd.join(".gbdialog"))
                .map_err(|e| format!("seed .gbdialog: {e}"))?;
            log::info!(
                "[git_monitor] provision {0}/{1}: seeded .gbdialog from deployed sources",
                org, project.repo_slug
            );
        }
        None if !cwd.join(".git").exists() => std::fs::write(
            cwd.join("README.md"),
            format!("# {0}\n\nVibe-managed bot sources (reform #1503).\n", project.name),
        )
        .map_err(|e| format!("seed README: {e}"))?,
        None => {}
    }
    // Always finish the wiring: with an existing partial-heal workspace
    // (.git, no origin or unpushed content) ensure_git_repo adds the origin,
    // commits and pushes main; a fully wired workspace exits on the origin
    // check. Idempotent from any state.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("runtime: {e}"))?;
    rt.block_on(botvibe::git_mode::ensure_git_repo(&p))
}

/// Resolve the `.gbdialog` source directory of a checkout. Repos store it at
/// the root (`.gbdialog/`) or bot-named (`{bot}.gbdialog/`) — accept both.
pub(crate) fn dialog_source_dir(checkout: &Path, bot_name: &str) -> Option<PathBuf> {
    [
        checkout.join(".gbdialog"),
        checkout.join(format!("{bot_name}.gbdialog")),
    ]
    .into_iter()
    .find(|p| p.is_dir())
}

/// Copy every source file of `source_dir` into `work_dir`, returning the
/// (drive-layout) `.bas` paths that were materialized.
pub(crate) fn materialize_dialog_dir(
    source_dir: &Path,
    work_dir: &Path,
    branch_slug: &str,
    bot_name: &str,
) -> Result<Vec<String>, String> {
    std::fs::create_dir_all(work_dir).map_err(|e| format!("mkdir {}: {e}", work_dir.display()))?;
    let mut materialized = Vec::new();
    let mut total = 0usize;
    for entry in std::fs::read_dir(source_dir)
        .map_err(|e| format!("read {}: {e}", source_dir.display()))?
    {
        let entry = entry.map_err(|e| format!("entry: {e}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        // `.ast` is a build artifact, never a source: copying one from the
        // repository would install a script compiled by another machine (or
        // another revision of this one) over the freshly materialized `.bas`.
        if file_name.ends_with(".ast") {
            continue;
        }
        if !file_name.ends_with(".bas") && !file_name.ends_with(".json") {
            continue;
        }
        total += 1;
        if total > MAX_MATERIALIZED_FILES {
            return Err(format!(
                "materialize {bot_name}: more than {MAX_MATERIALIZED_FILES} source files — trim the repo"
            ));
        }
        let to = work_dir.join(&file_name);
        std::fs::copy(&path, &to).map_err(|e| format!("copy {file_name}: {e}"))?;
        if file_name.ends_with(".bas") {
            // The compiled artifact of the previous revision is now stale.
            // Removing it is what makes the compiler pick the new source up:
            // `DriveCompiler` treats a missing `.ast` as a forced recompile,
            // and the git ETag it records is not a value the Drive monitor can
            // reproduce (one is a commit sha, the other an S3 ETag), so the
            // ETag alone must not be the trigger. Materialization only runs
            // when the checkout actually moved, so this is not churn.
            let tool = file_name.trim_end_matches(".bas").to_string();
            let stale_ast = work_dir.join(format!("{tool}.ast"));
            if stale_ast.exists() {
                if let Err(e) = std::fs::remove_file(&stale_ast) {
                    log::warn!(
                        "[git_monitor] {bot_name}: could not drop stale {tool}.ast ({e}) — the tool keeps running the previous revision"
                    );
                } else {
                    log::info!("[git_monitor] {bot_name}: {tool}.bas updated, stale .ast invalidated for recompile");
                }
            }
            materialized.push(format!("{branch_slug}.gbai/{bot_name}.gbdialog/{tool}.bas"));
        }
    }
    Ok(materialized)
}

/// Materialize one bot environment (PROD or TEST) from a checkout and return
/// the changed drive-layout paths.
pub(crate) fn materialize_checkout(
    checkout: &Path,
    work_root: &Path,
    branch_slug: &str,
    bot_name: &str,
) -> Result<Vec<String>, String> {
    let work_dir = work_root
        .join(format!("{branch_slug}.gborg/{branch_slug}.gbai"))
        .join(format!("{bot_name}.gbdialog"));
    let source_dir = match dialog_source_dir(checkout, bot_name) {
        Some(d) => d,
        None => return Ok(Vec::new()),
    };
    let materialized = materialize_dialog_dir(&source_dir, &work_dir, branch_slug, bot_name)?;

    // `.gbot` folder (bot config files: channel prompts, styles) — the runtime
    // reads `PROMPT-{CHANNEL}.md` from here and silently falls back to a
    // generic prompt when the copy is missing, so a failure is reported.
    let gbot_sources = [
        checkout.join(".gbot"),
        checkout.join(format!("{bot_name}.gbot")),
    ];
    if let Some(gbot_dir) = gbot_sources.into_iter().find(|p| p.is_dir()) {
        let gbot_target = work_root
            .join(format!("{branch_slug}.gborg/{branch_slug}.gbai"))
            .join(format!("{bot_name}.gbot"));
        if let Err(e) = copy_dir_recursive(&gbot_dir, &gbot_target) {
            log::warn!(
                "[git_monitor] {bot_name}: .gbot materialization failed: {e} — channel prompts stay on the fallback"
            );
        }
    }
    Ok(materialized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authenticated_url_embeds_token_once() {
        // Pure string test — no Vault call: the helper is exercised through
        // integration; here we verify the format contract of the replace.
        let base = "http://alm:4747";
        let url = format!("{base}/org/repo.git").replace("://", "://tok@");
        assert_eq!(url, "http://tok@alm:4747/org/repo.git");
    }

    #[test]
    fn dialog_source_dir_prefers_root_layout() {
        let tmp = std::env::temp_dir().join(format!("gbmon-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let root_dialog = tmp.join(".gbdialog");
        std::fs::create_dir_all(&root_dialog).unwrap();
        assert_eq!(
            dialog_source_dir(&tmp, "mybot"),
            Some(root_dialog.clone())
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn materialize_copies_bas_and_reports_paths() {
        let tmp = std::env::temp_dir().join(format!("gbmon-mat-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let src = tmp.join("checkout/.gbdialog");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("start.bas"), "TALK \"hi\"").unwrap();
        std::fs::write(src.join("tables.json"), "{}").unwrap();
        let work_root = tmp.join("work");
        let out = materialize_checkout(&tmp.join("checkout"), &work_root, "br", "mybot").unwrap();
        assert_eq!(out, vec!["br.gbai/mybot.gbdialog/start.bas".to_string()]);
        assert!(work_root
            .join("br.gborg/br.gbai/mybot.gbdialog/start.bas")
            .exists());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// A pushed `.bas` must invalidate the `.ast` compiled from the previous
    /// revision, and an `.ast` committed by mistake must never be installed —
    /// otherwise the bot keeps executing the old script after the push.
    #[test]
    fn materialize_invalidates_stale_ast() {
        let tmp = std::env::temp_dir().join(format!("gbmon-ast-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let src = tmp.join("checkout/.gbdialog");
        std::fs::create_dir_all(&src).unwrap();
        let work_root = tmp.join("work");
        let work_dir = work_root.join("br.gborg/br.gbai/mybot.gbdialog");
        std::fs::create_dir_all(&work_dir).unwrap();
        // Previous revision: compiled artifact of the old source.
        std::fs::write(work_dir.join("start.ast"), "OLD AST").unwrap();

        std::fs::write(src.join("start.bas"), "TALK \\\"new\\\"").unwrap();
        // A committed artifact must be ignored, not copied.
        std::fs::write(src.join("start.ast"), "COMMITTED AST").unwrap();

        materialize_checkout(&tmp.join("checkout"), &work_root, "br", "mybot").unwrap();

        assert!(!work_dir.join("start.ast").exists());
        assert_eq!(
            std::fs::read_to_string(work_dir.join("start.bas")).unwrap(),
            "TALK \\\"new\\\""
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
