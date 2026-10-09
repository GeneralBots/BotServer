use super::*;

pub fn handle_downloaded_file(temp_file: &std::path::Path, bin_path: &std::path::Path, binary_name: Option<&str>) -> Result<()> {
    let metadata = std::fs::metadata(temp_file)?;
    if metadata.len() == 0 {
        return Err(anyhow::anyhow!("Downloaded file is empty"));
    }
    let file_extension = temp_file
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    match file_extension {
        "gz" | "tgz" => {
            extract_tar_gz(temp_file, bin_path)?;
        }
        "zip" => {
            extract_zip(temp_file, bin_path)?;
        }
        _ => {
            if let Some(name) = binary_name {
                install_binary(temp_file, bin_path, name)?;
            } else {
                let final_path = bin_path.join(temp_file.file_name().unwrap_or_default());
                if temp_file.to_string_lossy().contains("botserver-installers") {
                    std::fs::copy(temp_file, &final_path)?;
                } else {
                    std::fs::rename(temp_file, &final_path)?;
                }
                make_executable(&final_path)?;
            }
        }
    }
    Ok(())
}

pub fn extract_tar_gz(temp_file: &std::path::Path, bin_path: &std::path::Path) -> Result<()> {
    let tar_gz = std::fs::File::open(temp_file)?;
    let tar = flate2::read::GzDecoder::new(tar_gz);
    let mut archive = tar::Archive::new(tar);
    archive.unpack(bin_path)?;

    collapse_single_subdirectory(bin_path)?;

    if !temp_file.to_string_lossy().contains("botserver-installers") {
        std::fs::remove_file(temp_file)?;
    }
    Ok(())
}

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

pub fn extract_zip(temp_file: &std::path::Path, bin_path: &std::path::Path) -> Result<()> {
    let file = std::fs::File::open(temp_file)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| anyhow::anyhow!("Failed to open zip archive: {}", e))?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)
            .map_err(|e| anyhow::anyhow!("Failed to read zip entry {}: {}", i, e))?;
        let Some(path) = entry.enclosed_name() else { continue };
        let target = bin_path.join(path);
        if entry.is_dir() {
            std::fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut output = std::fs::File::create(&target)?;
            std::io::copy(&mut entry, &mut output)?;
        }
    }
    #[cfg(unix)]
    {
        if let Ok(entries) = std::fs::read_dir(bin_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(_metadata) = std::fs::metadata(&path) {
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        if ext.is_empty() || ext == "sh" || ext == "bash" {
                            let _ = botlib::os::fs::get_permissions_manager().set_executable(&path);
                            trace!("Made executable: {}", path.display());
                        }
                    }
                }
            }
        }
    }

    collapse_single_subdirectory(bin_path)?;

    if !temp_file.to_string_lossy().contains("botserver-installers") {
        std::fs::remove_file(temp_file)?;
    }
    Ok(())
}
