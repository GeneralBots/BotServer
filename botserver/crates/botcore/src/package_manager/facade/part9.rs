use super::*;

/// After extracting a zip/tar.gz, some archives (e.g. EDB PostgreSQL for Windows)
/// place files inside a subdirectory (e.g. `pgsql/`). This function collapses
/// a single top-level subdirectory by moving its contents up and removing it.
pub(super) fn collapse_single_subdirectory(bin_path: &std::path::Path) -> Result<()> {
    let entries: Vec<_> = match std::fs::read_dir(bin_path) {
        Ok(iter) => iter.flatten().collect(),
        Err(_) => return Ok(()),
    };
    let dirs: Vec<_> = entries.iter().filter(|e| e.path().is_dir()).collect();
    if dirs.len() == 1 && entries.len() == 1 {
        let sub = dirs[0].path();
        for entry in std::fs::read_dir(&sub).into_iter().flatten().flatten() {
            let src = entry.path();
            let name = src.file_name().unwrap_or_default();
            let dst = bin_path.join(name);
            let _ = std::fs::rename(&src, &dst);
        }
        let _ = std::fs::remove_dir(&sub);
        trace!("Collapsed single subdirectory {:?} in {:?}", sub, bin_path);
    }
    Ok(())
}
