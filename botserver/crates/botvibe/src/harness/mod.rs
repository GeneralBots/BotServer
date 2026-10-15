//! #747 — Real tool harness for the Vibe agent.
//!
//! Replaces the `botvibe` stubs with real, sandboxed tools operating on a
//! per-project workspace:
//! - `file_*`: read/write/list/delete project files (workspace-scoped)
//! - `run_command`: execute a project command through the command guard
//! - `git_*`: status/log/diff/commit against the project workspace checkout
//! - `logs_*`: read/tail the project runtime logs
//! - `project_test`: run the project test suite
//!
//! All filesystem access is rooted at `VIBE_WORKSPACE_ROOT` (defaulting to
//! the platform stack data directory) and confined per project; commands go
//! through `cmd::run` which enforces the same discipline as
//! `botcore`'s SafeCommand (allowlist + forbidden shell metacharacters).

pub mod cmd;
pub mod file_tools;
pub mod git_tools;
pub mod log_tools;
pub mod run_tools;
pub mod test_tools;

use std::path::{Path, PathBuf};

#[cfg(test)]
pub(crate) static WORKSPACE_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[cfg(test)]
pub(crate) fn restore_workspace_root(previous: Option<std::ffi::OsString>) {
    match previous {
        Some(value) => std::env::set_var("VIBE_WORKSPACE_ROOT", value),
        None => std::env::remove_var("VIBE_WORKSPACE_ROOT"),
    }
}

/// Root that hosts every project workspace subdirectory.
pub fn workspace_root() -> PathBuf {
    std::env::var("VIBE_WORKSPACE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_workspace_root())
}

/// Resolve the stack data directory (dev `./botserver-stack`, prod `/opt/gbo`)
/// under `data/vibe-workspaces`. Relying on `get_stack_path()` makes the
/// default work on every platform without an env var and without depending on
/// a pre-created `/opt/gbo` in a dev checkout. `VIBE_WORKSPACE_ROOT` (if set
/// explicitly) still takes precedence via `workspace_root()`.
fn stack_vibe_workspaces() -> PathBuf {
    PathBuf::from(botlib::security::get_stack_path())
        .join("data")
        .join("vibe-workspaces")
}

#[cfg(target_os = "windows")]
fn default_workspace_root() -> PathBuf {
    // Preserve projects created before the stack-relative default was
    // introduced: prefer the legacy path when it actually exists.
    let legacy = PathBuf::from(r"C:\opt\gbo\data\vibe-workspaces");
    if legacy.exists() {
        return legacy;
    }
    stack_vibe_workspaces()
}

#[cfg(not(target_os = "windows"))]
fn default_workspace_root() -> PathBuf {
    stack_vibe_workspaces()
}

/// Validate a project id: alphanumerics, dash, underscore, dot — no path
/// separators, no traversal.
pub fn sanitize_project_id(project: &str) -> Result<String, String> {
    if project.is_empty() || project.len() > 128 || project == "." || project == ".." {
        return Err("invalid project id: empty or too long".into());
    }
    if project
        .chars()
        .any(|c| !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.'))
    {
        return Err(format!("invalid project id: {project}"));
    }
    Ok(project.to_string())
}

/// Containment barrier: canonicalizes `path` and verifies it stays inside
/// `base`. This is the canonical defense against path injection — symlinks
/// and `..` components are resolved by the filesystem itself, so the check
/// cannot be bypassed by crafted names. Both paths must exist (or have an
/// existing ancestor) for canonicalization to succeed.
pub fn ensure_contained(path: &Path, base: &Path) -> Result<PathBuf, String> {
    let canon_path = path
        .canonicalize()
        .map_err(|e| format!("canonicalize {}: {e}", path.display()))?;
    let canon_base = base
        .canonicalize()
        .map_err(|e| format!("canonicalize {}: {e}", base.display()))?;
    if !canon_path.starts_with(&canon_base) {
        return Err(format!(
            "path {} escapes workspace root {}",
            canon_path.display(),
            canon_base.display()
        ));
    }
    Ok(canon_path)
}

/// Resolve a path inside the project workspace, refusing `..` escapes and
/// absolute paths.
pub fn resolve_workspace_path(project: &str, rel: &str) -> Result<PathBuf, String> {
    let project = sanitize_project_id(project)?;
    if rel.contains('\0') {
        return Err("path contains NUL byte".into());
    }

    #[cfg(target_os = "windows")]
    let rel = {
        let trimmed = rel.trim();
        if trimmed.starts_with("//") || trimmed.starts_with("\\\\") {
            return Err(format!("network paths are not allowed: {trimmed}"));
        }
        // Small local models commonly spell workspace-relative paths as
        // `/index.js`. On Windows that is rooted at the current drive, so
        // normalize one leading separator while still rejecting UNC paths.
        trimmed
            .strip_prefix('/')
            .or_else(|| trimmed.strip_prefix('\\'))
            .unwrap_or(trimmed)
    };

    for seg in rel.split(['/', '\\']) {
        if seg == ".." {
            return Err(format!("path escapes workspace: {rel}"));
        }
    }
    if Path::new(rel).is_absolute() {
        return Err(format!("path is absolute: {rel}"));
    }
    let root = workspace_root().join(&project);
    Ok(root.join(rel))
}

/// Ensure the workspace subdir for a project exists. The project id is
/// sanitized first so a user-controlled name can never traverse outside the
/// workspaces root (path-injection guard).
pub fn ensure_workspace(project: &str) -> Result<PathBuf, String> {
    let safe = sanitize_project_id(project)?;
    let base = workspace_root();
    let root = base.join(safe);
    std::fs::create_dir_all(&root).map_err(|e| format!("create workspace: {e}"))?;
    // Containment barrier: canonicalize and verify the created workspace is
    // inside the workspaces root (path-injection defense, symlink-aware).
    ensure_contained(&root, &base)?;
    Ok(root)
}

pub fn read_rel_file(project: &str, rel: &str, max_bytes: u64) -> Result<Vec<u8>, String> {
    let path = resolve_workspace_path(project, rel)?;
    let meta = std::fs::metadata(&path).map_err(|e| format!("stat {rel}: {e}"))?;
    if meta.is_dir() {
        return Err(format!("{rel} is a directory"));
    }
    if (meta.len() as u64) > max_bytes {
        return Err(format!("{rel} exceeds {max_bytes} bytes"));
    }
    std::fs::read(&path).map_err(|e| format!("read {rel}: {e}"))
}

pub fn write_rel_file(project: &str, rel: &str, bytes: &[u8]) -> Result<(), String> {
    let path = resolve_workspace_path(project, rel)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {parent:?}: {e}"))?;
    }
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("file exceeds 4 MiB write limit".into());
    }
    std::fs::write(&path, bytes).map_err(|e| format!("write {rel}: {e}"))
}

/// List entries under a workspace dir, recursively with depth limit.
pub fn list_rel(project: &str, rel: &str, depth: u8) -> Result<Vec<String>, String> {
    let dir = resolve_workspace_path(project, rel)?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    if !dir.is_dir() {
        return Err(format!("{rel} is not a directory"));
    }
    let mut out = Vec::new();
    walk(&dir, &dir, depth, &mut out)?;
    Ok(out)
}

fn walk(dir: &Path, root: &Path, depth: u8, out: &mut Vec<String>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read_dir {dir:?}: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("read_dir entry: {e}"))?;
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .map_err(|e| format!("strip prefix: {e}"))?;
        let rel = rel.to_string_lossy().to_string();
        let is_dir = path.is_dir();
        out.push(if is_dir { format!("{rel}/") } else { rel });
        if is_dir && depth < 6 {
            walk(&path, root, depth + 1, out)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_project_rejects_traversal() {
        assert!(sanitize_project_id("my-web").is_ok());
        assert!(sanitize_project_id("a/b").is_err());
        assert!(sanitize_project_id("..").is_err());
        assert!(sanitize_project_id("").is_err());
    }

    #[test]
    fn resolve_rejects_parent_escape() {
        assert!(resolve_workspace_path("proj", "../etc/passwd").is_err());
        assert!(resolve_workspace_path("proj", "a/../../x").is_err());
        assert!(resolve_workspace_path("proj", "a/./b").is_ok());
        assert!(resolve_workspace_path("proj", "src/main.rs").is_ok());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn resolve_normalizes_model_style_rooted_path_on_windows() {
        let resolved = resolve_workspace_path("proj", "/index.js").expect("relative path");
        assert!(resolved.ends_with(std::path::Path::new("proj").join("index.js")));
        assert!(resolve_workspace_path("proj", r"\\server\share\index.js").is_err());
        assert!(resolve_workspace_path("proj", r"C:\index.js").is_err());
    }

    #[tokio::test]
    async fn write_read_list_roundtrip() {
        let _guard = WORKSPACE_ENV_LOCK.lock().await;
        let previous = std::env::var_os("VIBE_WORKSPACE_ROOT");
        let tmp = std::env::temp_dir().join(format!("vibe-harness-test-{}", uuid::Uuid::new_v4()));
        std::env::set_var("VIBE_WORKSPACE_ROOT", &tmp);
        let project = "proj-a";

        ensure_workspace(project).expect("ensure workspace");
        write_rel_file(project, "src/main.rs", b"fn main() {}").expect("write");
        write_rel_file(project, "README.md", b"# demo").expect("write");

        let bytes = read_rel_file(project, "src/main.rs", 1024).expect("read");
        assert_eq!(bytes, b"fn main() {}");

        let entries = list_rel(project, "", 0).expect("list");
        assert!(entries.iter().any(|e| e == "src/"));
        assert!(entries.iter().any(|e| e == "README.md"));

        let _ = std::fs::remove_dir_all(&tmp);
        restore_workspace_root(previous);
    }

    #[tokio::test]
    async fn read_rejects_oversize() {
        let _guard = WORKSPACE_ENV_LOCK.lock().await;
        let previous = std::env::var_os("VIBE_WORKSPACE_ROOT");
        let tmp = std::env::temp_dir().join(format!("vibe-harness-test-{}", uuid::Uuid::new_v4()));
        std::env::set_var("VIBE_WORKSPACE_ROOT", &tmp);
        let project = "proj-b";
        ensure_workspace(project).expect("ensure workspace");
        write_rel_file(project, "big.bin", &vec![0u8; 4096]).expect("write");
        assert!(read_rel_file(project, "big.bin", 1024).is_err());
        let _ = std::fs::remove_dir_all(&tmp);
        restore_workspace_root(previous);
    }
}
