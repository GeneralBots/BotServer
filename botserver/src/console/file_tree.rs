use crate::core::shared::state::AppState;
use color_eyre::Result;
use std::path::Path;
use std::sync::Arc;
#[derive(Debug, Clone)]
pub enum TreeNode {
    Bucket { name: String },
    Folder { bucket: String, path: String },
    File { bucket: String, path: String },
}
#[derive(Debug)]
pub struct FileTree {
    app_state: Arc<AppState>,
    items: Vec<(String, TreeNode)>,
    selected: usize,
    current_bucket: Option<String>,
    current_path: Vec<String>,
}

fn has_extension_ci(name: &str, ext: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

impl FileTree {
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self {
            app_state,
            items: Vec::new(),
            selected: 0,
            current_bucket: None,
            current_path: Vec::new(),
        }
    }
    pub async fn load_root(&mut self) -> Result<()> {
        self.items.clear();
        self.current_bucket = None;
        self.current_path.clear();
    if let Some(drive) = &self.app_state.drive {
        let result = drive.list_all_buckets().await;
        match result {
            Ok(buckets) => {
                for name in &buckets {
                    let icon = if has_extension_ci(name, "gbai") {
                        ""
                    } else {
                        "📦"
                    };
                    let display = format!("{} {}", icon, name);
                    self.items.push((
                        display,
                        TreeNode::Bucket {
                            name: name.to_string(),
                        },
                    ));
                }
            }
            Err(e) => {
                    self.items.push((
                        format!("x Error: {}", e),
                        TreeNode::Bucket {
                            name: String::new(),
                        },
                    ));
                }
            }
        } else {
            self.items.push((
                "x Drive not connected".to_string(),
                TreeNode::Bucket {
                    name: String::new(),
                },
            ));
        }
        if self.items.is_empty() {
            self.items.push((
                "(no buckets found)".to_string(),
                TreeNode::Bucket {
                    name: String::new(),
                },
            ));
        }
        self.selected = 0;
        Ok(())
    }
    pub async fn enter_bucket(&mut self, bucket: String) -> Result<()> {
        self.current_bucket = Some(bucket.clone());
        self.current_path.clear();
        self.load_bucket_contents(&bucket, "").await
    }
    pub async fn enter_folder(&mut self, bucket: String, path: String) -> Result<()> {
        self.current_bucket = Some(bucket.clone());
        let parts: Vec<&str> = path
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        self.current_path = parts.iter().map(|s| (*s).to_string()).collect();
        self.load_bucket_contents(&bucket, &path).await
    }
    pub fn go_up(&mut self) -> bool {
        if self.current_path.is_empty() {
            if self.current_bucket.is_some() {
                self.current_bucket = None;
                return true;
            }
            return false;
        }
        self.current_path.pop();
        true
    }
    pub async fn refresh_current(&mut self) -> Result<()> {
        if let Some(bucket) = &self.current_bucket.clone() {
            let path = self.current_path.join("/");
            self.load_bucket_contents(bucket, &path).await
        } else {
            self.load_root().await
        }
    }
    async fn load_bucket_contents(&mut self, bucket: &str, prefix: &str) -> Result<()> {
        self.items.clear();
        self.items.push((
            "  .. (go back)".to_string(),
            TreeNode::Folder {
                bucket: bucket.to_string(),
                path: "..".to_string(),
            },
        ));
        if let Some(drive) = &self.app_state.drive {
            let normalized_prefix = if prefix.is_empty() || prefix == "/" {
                String::new()
            } else if prefix.ends_with('/') {
                prefix.to_string()
            } else {
                format!("{}/", prefix)
            };
            let prefix_ref = if normalized_prefix.is_empty() { None } else { Some(normalized_prefix.as_str()) };
            let keys = drive.list_objects(bucket, prefix_ref).await.map_err(|e| color_eyre::eyre::eyre!("{e}"))?;
            let all_keys = keys;
            let mut folders = std::collections::HashSet::new();
            let mut files: Vec<(String, String)> = Vec::new();
            for key in all_keys {
                let relative =
                    if !normalized_prefix.is_empty() && key.starts_with(&normalized_prefix) {
                        &key[normalized_prefix.len()..]
                    } else {
                        &key
                    };
                if relative.is_empty() {
                    continue;
                }
                if let Some(slash_pos) = relative.find('/') {
                    let folder_name = &relative[..slash_pos];
                    if !folder_name.is_empty() {
                        folders.insert(folder_name.to_string());
                    }
                } else {
                    files.push((relative.to_string(), key));
                }
            }
            let mut folder_vec: Vec<String> = folders.into_iter().collect();
            folder_vec.sort();
            for folder_name in folder_vec {
                let full_path = if normalized_prefix.is_empty() {
                    folder_name.clone()
                } else {
                    format!("{}{}", normalized_prefix, folder_name)
                };
                let display = format!("📁 {}/", folder_name);
                self.items.push((
                    display,
                    TreeNode::Folder {
                        bucket: bucket.to_string(),
                        path: full_path,
                    },
                ));
            }
            files.sort_by(|(a, _), (b, _)| a.cmp(b));
            for (ref name, ref full_path) in files {
                let icon = if has_extension_ci(name, "bas")
                    || has_extension_ci(name, "ast")
                    || has_extension_ci(name, "csv")
                    || has_extension_ci(name, "gbkb")
                {
                    ""
                } else if has_extension_ci(name, "json") {
                    "🔖"
                } else {
                    "📄"
                };
                let display = format!("{} {}", icon, name);
                self.items.push((
                    display,
                    TreeNode::File {
                        bucket: bucket.to_string(),
                        path: full_path.to_string(),
                    },
                ));
            }
        }
        if self.items.len() == 1 {
            self.items.push((
                "(empty folder)".to_string(),
                TreeNode::Folder {
                    bucket: bucket.to_string(),
                    path: String::new(),
                },
            ));
        }
        self.selected = 0;
        Ok(())
    }
    pub fn render_items(&self) -> &[(String, TreeNode)] {
        &self.items
    }

    pub fn get_items(&self) -> &[(String, TreeNode)] {
        &self.items
    }
    pub fn selected_index(&self) -> usize {
        self.selected
    }
    pub fn get_selected_node(&self) -> Option<&TreeNode> {
        self.items.get(self.selected).map(|(_, node)| node)
    }
    pub fn get_selected_bot(&self) -> Option<String> {
        if let Some(bucket) = &self.current_bucket {
            if has_extension_ci(bucket, "gbai") {
                return Some(bucket.trim_end_matches(".gbai").to_string());
            }
        }
        if let Some((_, TreeNode::Bucket { name })) = self.items.get(self.selected) {
            if has_extension_ci(name, "gbai") {
                return Some(name.trim_end_matches(".gbai").to_string());
            }
        }
        None
    }
    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }
    pub fn move_down(&mut self) {
        if self.selected < self.items.len().saturating_sub(1) {
            self.selected += 1;
        }
    }
}
