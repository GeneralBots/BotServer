use super::*;

/// Collect every `*.gbai` directory under `root`, recursively (bounded by
/// `depth`). The bot name is the `.gbai` directory's own name, which is what the
/// runtime resolvers and the drive monitor key on. Unreadable directories and
/// entries are warned about and skipped — a malformed template must never abort
/// startup.
pub(super) fn collect_gbai_dirs_recursive(dir: &Path, depth: usize, out: &mut Vec<(String, PathBuf)>) {
    if depth == 0 {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            warn!("Skipping unreadable template dir {}: {e}", dir.display());
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                warn!(
                    "Skipping unreadable template entry in {}: {e}",
                    dir.display()
                );
                continue;
            }
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = match entry.file_name().into_string() {
            Ok(name) => name,
            Err(_) => continue,
        };
        if let Some(bot_name) = name.strip_suffix(".gbai") {
            if !bot_name.is_empty() {
                out.push((bot_name.to_string(), path));
            }
            // A `.gbai` directory holds bot files, never nested templates.
            continue;
        }
        collect_gbai_dirs_recursive(&path, depth - 1, out);
    }
}

/// Recursively copy a directory tree, replacing the destination. Returns an
/// error with context when any file cannot be copied, leaving already-copied
/// files in place (the caller treats each template independently).
pub(super) fn copy_dir_recursive(src: &Path, dst: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst).with_context(|| format!("Failed to create {}", dst.display()))?;
    for entry in
        std::fs::read_dir(src).with_context(|| format!("Failed to read {}", src.display()))?
    {
        let entry = entry.with_context(|| format!("Failed to read entry in {}", src.display()))?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            std::fs::copy(&from, &to).with_context(|| {
                format!("Failed to copy {} to {}", from.display(), to.display())
            })?;
        }
    }
    Ok(())
}
