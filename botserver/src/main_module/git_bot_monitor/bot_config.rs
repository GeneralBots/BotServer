//! Reform #1501/#1505 — a bot's `.gbot` configuration (channel prompts, styles,
//! `config.csv`) inside its git repository.
//!
//! The runtime reads `PROMPT-{CHANNEL}.md` from the work layout
//! (`work/{branch}.gborg/{branch}.gbai/{bot}.gbot/`), and
//! `load_system_prompt_for_channel` falls back to a generic assistant prompt
//! when that directory is missing. Losing `.gbot` is therefore not a cosmetic
//! loss: a media-filing bot stops calling `classify_media`, a sales bot loses
//! its tone, and no error is logged anywhere — the model simply answers with
//! the default behaviour.
//!
//! Three paths keep it from being lost:
//! - [`write_config_files`] — a shipped template commits its channel prompts
//!   together with the tool, so the behaviour ships as one unit.
//! - [`import_config_from_drive`] — recovery/import of `.gbot` objects that
//!   still live in Drive, including the ones the archive pass already moved to
//!   `{branch}.gbai/archive/{bot}-{stamp}/.gbot/`.
//! - [`has_config_in_repo`] — the archive pass refuses to delete a Drive
//!   `.gbot` whose content is not in the repository yet.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use botcore::shared::state::AppState;
use botcore::shared::utils::DbPool;

use super::core::ensure_checkout_with_heal;
use super::loop_ops::list_monitored_bots;
use super::source_ops::{commit_and_push, find_source_target, prepare_checkout};

/// Directory names a checkout may use for the bot configuration, in the order
/// `materialize_checkout` looks them up.
fn config_candidates(checkout: &Path, bot_name: &str) -> [PathBuf; 2] {
    [
        checkout.join(".gbot"),
        checkout.join(format!("{bot_name}.gbot")),
    ]
}

/// The bot's configuration directory inside a checkout, when it exists.
fn existing_config_dir(checkout: &Path, bot_name: &str) -> Option<PathBuf> {
    config_candidates(checkout, bot_name)
        .into_iter()
        .find(|p| p.is_dir())
}

/// True when the bot's repository already carries configuration files.
pub(crate) fn has_config_in_repo(checkout: &Path, bot_name: &str) -> bool {
    match existing_config_dir(checkout, bot_name) {
        Some(dir) => std::fs::read_dir(&dir)
            .map(|mut entries| entries.any(|e| e.map(|f| f.path().is_file()).unwrap_or(false)))
            .unwrap_or(false),
        None => false,
    }
}

/// Resolve the bot's checkout without editing it — used by the archive pass to
/// decide whether a Drive prefix is safe to remove.
pub(crate) fn checkout_of(pool: &DbPool, bot_name: &str) -> Option<PathBuf> {
    let target = find_source_target(pool, bot_name).ok()?;
    ensure_checkout_with_heal(pool, &target.project, &target.org).ok()
}

/// Write `.gbot` configuration files into the bot's repository and push them.
///
/// `files` are `(file name, content)` pairs — the names are used verbatim, so
/// a caller can only write into the configuration root. Idempotent: identical
/// content produces no commit.
pub(crate) fn write_config_files(
    pool: &DbPool,
    bot_name: &str,
    files: &[(String, String)],
    message: &str,
) -> Result<Vec<String>, String> {
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let target = find_source_target(pool, bot_name)?;
    let checkout = prepare_checkout(pool, &target)?;
    let dir = existing_config_dir(&checkout, bot_name)
        .unwrap_or_else(|| checkout.join(".gbot"));
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir {}: {e}", dir.display()))?;
    let mut written = Vec::new();
    for (name, body) in files {
        if name.contains('/') || name.contains("..") || name.is_empty() {
            return Err(format!("invalid bot config file name: '{name}'"));
        }
        std::fs::write(dir.join(name), body)
            .map_err(|e| format!("write .gbot/{name}: {e}"))?;
        written.push(name.clone());
    }
    commit_and_push(pool, &checkout, &target, message, &written)?;
    Ok(written)
}

/// Drive keys holding one bot's `.gbot` files, newest layout first: the live
/// prefix, then any prefix the archive pass already moved aside.
async fn drive_config_keys(
    state: &AppState,
    bucket: &str,
    branch_prefix: &str,
    bot_name: &str,
) -> Result<Vec<(String, String)>, String> {
    let s3 = state
        .drive
        .as_ref()
        .ok_or_else(|| "drive (S3) not available in AppState".to_string())?;
    let live = format!("{branch_prefix}{bot_name}.gbot/");
    let mut keys = s3.list_objects(bucket, Some(&live)).await.unwrap_or_default();
    if keys.is_empty() {
        // The archive pass moved the prefix under `archive/{bot}-{stamp}/`;
        // recovery must read it back from there, otherwise the bot keeps
        // running on the generic fallback prompt forever.
        let archive = format!("{branch_prefix}archive/");
        if let Ok(objects) = s3.list_objects(bucket, Some(&archive)).await {
            let marker = format!("/{bot_name}.gbot/");
            for key in objects {
                if key.contains(&marker) {
                    keys.push(key);
                }
            }
        }
    }
    let mut pairs: Vec<(String, String)> = Vec::new();
    for key in keys {
        let name = match key.rsplit(&marker_name(bot_name)).next() {
            Some(rest) if !rest.is_empty() && !rest.contains('/') => rest.to_string(),
            _ => continue,
        };
        let content = match s3.get_object(bucket, &key).await {
            Ok(c) => c,
            Err(e) => {
                log::warn!("[git_config] {}: get {key}: {e}", bot_name);
                continue;
            }
        };
        // A non-UTF8 object is not a prompt; skip it instead of committing
        // replacement characters the runtime would later serve as the system
        // prompt.
        match String::from_utf8(content) {
            Ok(text) => pairs.push((name, text)),
            Err(_) => log::warn!("[git_config] {bot_name}: {key} is not UTF-8 text — skipped"),
        }
    }
    Ok(pairs)
}

fn marker_name(bot_name: &str) -> String {
    format!("{bot_name}.gbot/")
}

/// Import a bot's `.gbot` files from Drive into its repository when the
/// repository has none. Idempotent: a checkout that already carries
/// configuration is left untouched, so the monitor never fights a commit.
pub(crate) async fn import_config_from_drive(
    state: Arc<AppState>,
    pool: DbPool,
    bot_name: &str,
    bucket: &str,
    branch_prefix: &str,
) -> usize {
    if state.drive.is_none() {
        return 0;
    }
    let checkout = {
        let pool = pool.clone();
        let bot = bot_name.to_string();
        match tokio::task::spawn_blocking(move || checkout_of(&pool, &bot)).await {
            Ok(Some(dir)) => dir,
            Ok(None) => return 0,
            Err(e) => {
                log::warn!("[git_config] {bot_name}: checkout task: {e}");
                return 0;
            }
        }
    };
    if has_config_in_repo(&checkout, bot_name) {
        return 0;
    }
    let pairs = match drive_config_keys(&state, bucket, branch_prefix, bot_name).await {
        Ok(p) => p,
        Err(e) => {
            log::warn!("[git_config] {bot_name}: list Drive config: {e}");
            return 0;
        }
    };
    if pairs.is_empty() {
        return 0;
    }
    let files: Vec<(String, String)> = pairs;
    let count = files.len();
    let pool_for_write = pool.clone();
    let bot = bot_name.to_string();
    match tokio::task::spawn_blocking(move || {
        write_config_files(
            &pool_for_write,
            &bot,
            &files,
            "autotask: import .gbot configuration from Drive",
        )
    })
    .await
    {
        Ok(Ok(_)) => {
            log::info!(
                "[git_config] {bot_name}: imported {count} Drive configuration file(s) into the repository (#1501)"
            );
            count
        }
        Ok(Err(e)) => {
            log::warn!("[git_config] {bot_name}: config import deferred: {e}");
            0
        }
        Err(e) => {
            log::warn!("[git_config] {bot_name}: config import task: {e}");
            0
        }
    }
}

/// Bots with a git-owned project, used by the import and archive passes.
pub(crate) fn git_bot_names(pool: &DbPool) -> Vec<(String, String)> {
    list_monitored_bots(pool)
        .into_iter()
        .filter_map(|p| {
            let slug = p.branch_slug.clone().unwrap_or_default();
            if slug.trim().is_empty() {
                return None;
            }
            Some((p.name, slug.trim().to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_candidates_cover_both_layouts() {
        let tmp = std::env::temp_dir().join(format!("gbconf-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let a = config_candidates(&tmp, "mybot");
        assert!(a[0].ends_with(".gbot"));
        assert!(a[1].ends_with("mybot.gbot"));
        assert!(!has_config_in_repo(&tmp, "mybot"));
        std::fs::create_dir_all(&a[0]).unwrap_or_default();
        assert!(!has_config_in_repo(&tmp, "mybot"));
        std::fs::write(a[0].join("PROMPT.md"), "hi").unwrap_or_default();
        assert!(has_config_in_repo(&tmp, "mybot"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn marker_name_targets_the_config_prefix() {
        assert_eq!(marker_name("acme"), "acme.gbot/");
    }
}
