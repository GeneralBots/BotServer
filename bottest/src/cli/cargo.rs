//! Part of the bottest CLI — see `cli/mod.rs` for the entry point.
//!
//! Moved out of the binary crate by #1505 so the fixture module tree is
//! compiled once; the CLI is part of the `bottest` library now.

use anyhow::Result;

pub fn discover_test_files(test_dir: &str) -> Vec<String> {
    let path = std::path::PathBuf::from(test_dir);
    if !path.exists() {
        return Vec::new();
    }

    let mut test_files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&path) {
        for entry in entries.flatten() {
            let file_path = entry.path();
            if file_path.extension().is_some_and(|e| e == "rs") {
                if let Some(name) = file_path.file_stem() {
                    let name_str = name.to_string_lossy().to_string();
                    if name_str != "mod" {
                        test_files.push(name_str);
                    }
                }
            }
        }
    }
    test_files.sort();
    test_files
}

pub fn run_cargo_test(
    test_type: &str,
    filter: Option<&str>,
    parallel: bool,
    env_vars: Vec<(&str, &str)>,
    features: Option<&str>,
) -> Result<(usize, usize, usize)> {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("test");
    cmd.arg("-p").arg("bottest");

    if let Some(feat) = features {
        cmd.arg("--features").arg(feat);
    }

    cmd.arg("--test").arg(test_type);

    if let Some(pattern) = filter {
        cmd.arg(pattern);
    }

    cmd.arg("--");

    if !parallel {
        cmd.arg("--test-threads=1");
    }

    cmd.arg("--nocapture");

    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    let output = cmd.output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");

    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;

    for line in combined.lines() {
        if line.contains("test result:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            for (i, part) in parts.iter().enumerate() {
                if *part == "passed;" && i > 0 {
                    passed = parts[i - 1].parse().unwrap_or(0);
                }
                if *part == "failed;" && i > 0 {
                    failed = parts[i - 1].parse().unwrap_or(0);
                }
                if *part == "ignored;" && i > 0 {
                    skipped = parts[i - 1].parse().unwrap_or(0);
                }
            }
        }
    }

    Ok((passed, failed, skipped))
}
