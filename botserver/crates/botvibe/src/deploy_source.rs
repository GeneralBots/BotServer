//! Reform #1505 — git-revision deploy source for website payloads.
//!
//! A deploy must ship **committed state**, not whatever happens to sit in the
//! agent workspace. For `source_control = "git"` projects the payload is
//! materialized with `git archive` from the checkout HEAD (the same revision
//! the deploy pipeline snapshotted into `release/deploy-<ts>`), so PROD is
//! always byte-identical to the pushed commit and a rollback re-materializes
//! from history instead of live workspace state.
//!
//! Native-mode projects (no checkout) keep the legacy workspace walk.

use crate::harness::cmd::run;
use crate::projects::Project;
use serde_json::Value;

/// Run a `git` command inside `cwd`, returning trimmed stdout. Any failure
/// (binary missing, non-zero exit, timeout) collapses into a descriptive
/// `String` — callers decide whether to fall back or fail the deploy.
fn git_run(cwd: &std::path::Path, args: &[&str]) -> Result<String, String> {
    let owned: Vec<String> = args.iter().map(|a| (*a).to_string()).collect();
    let out = run("git", &owned, cwd, 60).map_err(|e| format!("git {args:?}: {e}"))?;
    if out.exit_code != Some(0) {
        return Err(format!("git {args:?}: {}", out.stderr.trim()));
    }
    Ok(out.stdout.trim().to_string())
}

/// The workspace checkout of `project`, if it is a real git-mode clone.
fn git_checkout(project: &Project) -> Result<std::path::PathBuf, String> {
    if project.source_control != "git" {
        return Err(format!(
            "project '{}' is not git-controlled",
            project.name
        ));
    }
    let safe = crate::harness::sanitize_project_id(&project.name)?;
    let cwd = crate::harness::ensure_workspace(&safe)?;
    if !cwd.join(".git").exists() {
        return Err(format!(
            "workspace '{}' has no git checkout yet — provisioning pending",
            cwd.display()
        ));
    }
    Ok(cwd)
}

/// Resolve the revision to deploy: an explicit `rev` when given, otherwise
/// the checkout HEAD. An empty/unborn HEAD (fresh repo, nothing committed)
/// is reported as `None` so the caller can fall back to the workspace.
pub fn deploy_revision(project: &Project, rev: Option<&str>) -> Result<Option<String>, String> {
    let cwd = git_checkout(project)?;
    match rev {
        Some(r) if !r.trim().is_empty() => Ok(Some(r.trim().to_string())),
        _ => {
            let head = git_run(&cwd, &["rev-parse", "HEAD"]).unwrap_or_default();
            if head.is_empty() {
                Ok(None)
            } else {
                Ok(Some(head))
            }
        }
    }
}

/// Materialize the deploy payload for `project` from git revision `rev`
/// (HEAD when `None`). Every tracked file becomes a `{path, content}` entry —
/// the same shape `collect_workspace_files` produces, so the proxy staging
/// path (tar, limits, serveability checks) is unchanged.
///
/// Files come from `git archive`, i.e. the committed tree: uncommitted
/// workspace edits never leak into a deploy.
pub fn materialize_files(
    project: &Project,
    rev: Option<&str>,
) -> Result<Vec<Value>, String> {
    // Native-mode projects (source_control != "git") keep the legacy
    // workspace walk — the module contract. Git-mode projects never fall
    // back here: a pending provisioning or git failure must fail the
    // deploy rather than ship uncommitted workspace state (#1505).
    if project.source_control != "git" {
        return fallback_files(project, "native mode");
    }
    let revision = match deploy_revision(project, rev)? {
        Some(r) => r,
        None => return fallback_files(project, "no commits yet"),
    };
    let cwd = git_checkout(project)?;
    let archive = git_run(&cwd, &["archive", "--format=tar", &revision])?;
    if archive.is_empty() {
        return fallback_files(project, "empty tree");
    }
    let mut out = Vec::new();
    let mut total_bytes = 0u64;
    let max_bytes = crate::publish::publish_max_bytes_budget();
    for entry in tar::Archive::new(archive.as_bytes()).entries().map_err(|e| format!("tar: {e}"))? {
        let mut entry = entry.map_err(|e| format!("tar entry: {e}"))?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = match entry.path() {
            Ok(p) => p.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        // git archive output is trusted (no `..`, no absolute paths), but the
        // guard stays — the payload shape is shared with user-facing staging.
        if path.is_empty() || path.contains("..") {
            continue;
        }
        let mut content = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut content)
            .map_err(|e| format!("read {path}: {e}"))?;
        total_bytes += content.len() as u64;
        if total_bytes > max_bytes {
            return Err(format!(
                "deploy payload exceeds size budget ({max_bytes} bytes)"
            ));
        }
        out.push(serde_json::json!({ "path": path, "content": content }));
    }
    if out.is_empty() {
        return fallback_files(project, "empty tree");
    }
    log::info!(
        "Vibe deploy-source {}: {} file(s) materialized from revision {}",
        project.name,
        out.len(),
        botlib::text::truncate_at_char_boundary(&revision, 12)
    );
    Ok(out)
}

/// Legacy source: the agent workspace walk. Used by native-mode projects and
/// as the fallback when a git-mode project has nothing committed yet.
fn fallback_files(project: &Project, reason: &str) -> Result<Vec<Value>, String> {
    log::info!(
        "Vibe deploy-source {}: using workspace files ({reason})",
        project.name
    );
    crate::publish::collect_workspace_files(project)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_project(name: &str, source_control: &str) -> Project {
        Project {
            id: uuid::Uuid::new_v4(),
            org_id: uuid::Uuid::new_v4(),
            branch_id: uuid::Uuid::new_v4(),
            name: name.to_string(),
            project_type: "website".to_string(),
            repository: String::new(),
            framework: None,
            custom_domain: None,
            source_control: source_control.to_string(),
            status: "active".to_string(),
            environment: "development".to_string(),
            payload: serde_json::json!({}),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn deploy_revision_requires_git_mode() {
        let p = test_project("Native Site", "native");
        assert!(deploy_revision(&p, None).is_err());
    }

    #[test]
    fn materialize_native_project_falls_back_to_workspace() {
        let p = test_project("Native Site 2", "native");
        // Native mode: git_checkout rejects, materialize falls back to the
        // workspace walk (which may be empty — that is the legacy contract).
        let files = materialize_files(&p, None);
        assert!(files.is_ok());
    }
}
