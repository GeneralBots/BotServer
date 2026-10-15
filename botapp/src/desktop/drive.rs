use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::{Emitter, Window};

#[derive(Debug, Serialize, Deserialize)]
pub struct FileItem {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: Option<u64>,
}

/// List files in a directory.
///
/// # Errors
/// Returns an error if the path does not exist or cannot be read.
#[tauri::command]
pub fn list_files(path: &str) -> Result<Vec<FileItem>, String> {
    let base_path = Path::new(path);
    let mut files = Vec::new();

    if !base_path.exists() {
        return Err("Path does not exist".into());
    }

    for entry in fs::read_dir(base_path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let metadata = entry.metadata().ok();

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let size = metadata.as_ref().map(std::fs::Metadata::len);
        let is_dir = metadata.is_some_and(|m| m.is_dir());

        files.push(FileItem {
            name,
            path: path.to_str().unwrap_or("").to_string(),
            is_dir,
            size,
        });
    }

    files.sort_by(|a, b| {
        if a.is_dir && !b.is_dir {
            std::cmp::Ordering::Less
        } else if !a.is_dir && b.is_dir {
            std::cmp::Ordering::Greater
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });

    Ok(files)
}

/// Upload a file to the specified destination.
///
/// # Errors
/// Returns an error if the source file is invalid or the copy operation fails.
#[tauri::command]
pub fn upload_file(window: Window, src_path: &str, dest_path: &str) -> Result<(), String> {
    let src = PathBuf::from(src_path);
    let dest_dir = PathBuf::from(dest_path);
    let dest = dest_dir.join(src.file_name().ok_or("Invalid source file")?);

    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    }

    let mut source_file = File::open(&src).map_err(|e| e.to_string())?;
    let mut dest_file = File::create(&dest).map_err(|e| e.to_string())?;
    let file_size = source_file.metadata().map_err(|e| e.to_string())?.len();

    let mut buffer = [0; 8192];
    let mut total_read: u64 = 0;

    loop {
        let bytes_read = source_file.read(&mut buffer).map_err(|e| e.to_string())?;
        if bytes_read == 0 {
            break;
        }
        dest_file
            .write_all(&buffer[..bytes_read])
            .map_err(|e| e.to_string())?;

        total_read += bytes_read as u64;

        let progress = (total_read * 100).checked_div(file_size).unwrap_or(100);

        window
            .emit("upload_progress", progress)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Create a new folder at the specified path.
///
/// # Errors
/// Returns an error if the folder already exists or cannot be created.
#[tauri::command]
pub fn create_folder(path: &str, name: &str) -> Result<(), String> {
    let full_path = Path::new(path).join(name);

    if full_path.exists() {
        return Err("Folder already exists".into());
    }

    fs::create_dir(&full_path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Delete a file or folder at the specified path.
///
/// # Errors
/// Returns an error if the path does not exist or the item cannot be deleted.
#[tauri::command]
pub fn delete_path(path: &str) -> Result<(), String> {
    let target = Path::new(path);

    if !target.exists() {
        return Err("Path does not exist".into());
    }

    if target.is_dir() {
        fs::remove_dir_all(target).map_err(|e| e.to_string())?;
    } else {
        fs::remove_file(target).map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Get the user's home directory path.
///
/// # Errors
/// Returns an error if the home directory cannot be determined.
#[tauri::command]
pub fn get_home_dir() -> Result<String, String> {
    dirs::home_dir()
        .and_then(|p| p.to_str().map(String::from))
        .ok_or_else(|| "Could not determine home directory".into())
}
