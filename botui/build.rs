fn main() {
let manifest_dir = env!("CARGO_MANIFEST_DIR");
let ui_path = std::path::Path::new(manifest_dir).join("ui");
println!("cargo:rustc-env=BOTUI_UI_PATH={}", ui_path.display());

let commit = std::env::var("BOTUI_COMMIT")
.ok()
.or_else(git_commit_hash);
if let Some(hash) = commit {
println!("cargo:rustc-env=BOTUI_COMMIT={}", hash);
}
}

fn git_commit_hash() -> Option<String> {
let output = std::process::Command::new("git")
.args(["rev-parse", "--short", "HEAD"])
.output()
.ok()?;
if !output.status.success() {
return None;
}
String::from_utf8(output.stdout).ok().map(|s| s.trim().to_string())
}