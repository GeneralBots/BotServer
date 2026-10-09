use super::*;

impl BotManager {
    pub(crate) fn load_template_from_directory(
        &self,
        path: &std::path::Path,
        name: &str,
    ) -> Option<BotTemplate> {
        let metadata_path = path.join("template.toml");
        let description = if metadata_path.exists() {
            std::fs::read_to_string(&metadata_path).ok()
                .and_then(|content| {
                    toml::from_str::<toml::Value>(&content).ok()
                        .and_then(|v| v.get("description")
                            .and_then(|d| d.as_str().map(String::from)))
                })
                .unwrap_or_else(|| format!("Template loaded from {}", name))
        } else {
            format!("Template loaded from {}", name)
        };

        let dialog_dir = path.join(format!("{}.gbdialog", name));
        let dialogs = if dialog_dir.exists() {
            std::fs::read_dir(&dialog_dir).ok()
                .map(|entries| {
                    entries.flatten()
                        .filter(|e| e.path().extension().is_some_and(|ext| ext == "bas"))
                        .filter_map(|e| {
                            let file_name = e.file_name().to_string_lossy().to_string();
                            let content = std::fs::read_to_string(e.path()).ok()?;
                            Some(DialogFile { name: file_name, content })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let preview_image = ["preview.png", "preview.jpg", "preview.svg"]
            .iter()
            .map(|f| path.join(f))
            .find(|p| p.exists())
            .and_then(|p| p.to_str().map(String::from));

        Some(BotTemplate {
            name: name.to_string(),
            display_name: name.to_string(),
            description,
            category: "Custom".to_string(),
            dialogs,
            files: Vec::new(),
            preview_image,
        })
    }

    pub(crate) fn get_org_slug_from_db(&self, conn: &DbPool, org_id: Uuid) -> String {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to get database connection for org lookup: {}", e);
                return "default".to_string();
            }
        };

        let result = organizations::table
            .filter(organizations::org_id.eq(org_id))
            .select(organizations::slug)
            .first::<String>(&mut db_conn)
            .optional();

        match result {
            Ok(Some(slug)) => {
                debug!("Found org slug '{}' for org_id {}", slug, org_id);
                slug
            }
            Ok(None) => {
                debug!("No org found for org_id {}, using 'default'", org_id);
                "default".to_string()
            }
            Err(e) => {
                warn!("Database error looking up org {}: {}", org_id, e);
                "default".to_string()
            }
        }
    }

    pub(crate) async fn create_minio_bucket(
        &self,
        bucket_name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Creating MinIO bucket: {}", bucket_name);

        let bucket_arg = format!("local/{}", bucket_name);
        if let Ok(cmd) = SafeCommand::new("mc")
            .and_then(|c| c.arg("mb"))
            .and_then(|c| c.arg(&bucket_arg))
            .and_then(|c| c.arg("--ignore-existing"))
        {
            match cmd.execute_async().await {
                Ok(result) => {
                    if !result.status.success() {
                        let stderr = String::from_utf8_lossy(&result.stderr);
                        if !stderr.contains("already exists") {
                            warn!("Bucket creation warning: {}", stderr);
                        }
                    }
                }
                Err(e) => error!("Failed to create bucket: {}", e),
            }
        }
        Ok(())
    }

    pub(crate) async fn apply_template(
        &self,
        bucket: &str,
        template_name: &str,
        bot_name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Applying template '{}' to bucket '{}'", template_name, bucket);

        let templates = self.templates.read().await;
        let template = templates
            .get(template_name)
            .ok_or_else(|| format!("Template not found: {}", template_name))?;

        for file in &template.files {
            let content = file.content.replace("{{botname}}", bot_name);
            self.upload_file(bucket, &file.path, content.as_bytes()).await?;
        }

        info!("Applied template '{}' ({} files)", template_name, template.files.len());
        Ok(())
    }

    pub(crate) async fn create_default_structure(
        &self,
        bucket: &str,
        bot_name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Creating default structure in bucket: {}", bucket);

        let dirs = [
            format!("{}.gbdialog/", bot_name),
            format!("{}.gbkb/", bot_name),
            format!("{}.gbot/", bot_name),
        ];

        for dir in &dirs {
            self.upload_file(bucket, dir, b"").await?;
        }

        let start_script = format!(
            r#"REM {} - Start Script
TALK "Hello! I'm {}. How can I help you?"
HEAR user_input
response = LLM "Respond helpfully to: " + user_input
TALK response
"#,
            bot_name, bot_name
        );

        self.upload_file(bucket, &format!("{}.gbdialog/start.bas", bot_name), start_script.as_bytes()).await?;

        info!("Default structure created");
        Ok(())
    }

    async fn upload_file(
        &self,
        bucket: &str,
        path: &str,
        content: &[u8],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Uploading to {}/{}", bucket, path);

        let temp_path = format!("/tmp/upload_{}", Uuid::new_v4());
        std::fs::write(&temp_path, content)?;

        let dest_path = format!("local/{}/{}", bucket, path);
        if let Ok(cmd) = SafeCommand::new("mc")
            .and_then(|c| c.arg("cp"))
            .and_then(|c| c.arg(&temp_path))
            .and_then(|c| c.arg(&dest_path))
        {
            let _ = cmd.execute_async().await;
        }

        let _ = std::fs::remove_file(&temp_path);
        Ok(())
    }

    pub async fn get_templates(&self) -> Vec<BotTemplate> {
        let templates = self.templates.read().await;
        templates.values().cloned().collect()
    }

    pub async fn get_bot(&self, bot_id: Uuid) -> Option<BotConfig> {
        let cache = self.bots_cache.read().await;
        cache.get(&bot_id).cloned()
    }

    pub async fn get_bot_by_name(&self, org_slug: &str, bot_name: &str) -> Option<BotConfig> {
        let cache = self.bots_cache.read().await;
        cache.values().find(|b| b.org_slug == org_slug && b.name == bot_name).cloned()
    }

    pub async fn list_bots(&self, org_id: Uuid) -> Vec<BotConfig> {
        let cache = self.bots_cache.read().await;
        cache.values().filter(|b| b.org_id == org_id).cloned().collect()
    }

    pub async fn delete_bot(&self, bot_id: Uuid) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let bot = self.get_bot(bot_id).await.ok_or("Bot not found")?;
        info!("Deleting bot: {} ({})", bot.name, bot_id);

        let bucket_path = format!("local/{}", bot.bucket);
        if let Ok(cmd) = SafeCommand::new("mc")
            .and_then(|c| c.arg("rm"))
            .and_then(|c| c.arg("--recursive"))
            .and_then(|c| c.arg("--force"))
            .and_then(|c| c.arg(&bucket_path))
        {
            let _ = cmd.execute_async().await;
        }

        {
            let mut cache = self.bots_cache.write().await;
            cache.remove(&bot_id);
        }

        info!("Bot deleted: {}", bot_id);
        Ok(())
    }
}
pub fn get_default_bot() -> (String, String) {
    ("default".to_string(), "Default Bot".to_string())
}
pub async fn get_bot_config(
    axum::extract::State(state): axum::extract::State<Arc<botcore::shared::state::AppState>>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<axum::Json<serde_json::Value>, crate::security::SafeErrorResponse> {
    use botcore::shared::models::schema::bots;

    let mut conn = state.conn.get().map_err(|e| {
        log::error!("DB connection error in get_bot_config: {}", e);
        crate::security::SafeErrorResponse::internal_error()
    })?;

    let cfg = botcore::config::ConfigManager::new(state.conn.clone());

    let sensitive_prefixes = ["llm-key", "llm-url", "llm-server", "secret", "token", "password", "api-key"];
    let mut map: HashMap<String, String> = if let Some(name) = params.get("bot_name") {
        let bot_uuid = bots::table
            .filter(bots::name.eq(name))
            .select(bots::id)
            .first::<Uuid>(&mut conn)
            .ok();
        if let Some(bid) = bot_uuid {
            cfg.get_all_config(&bid)
        } else {
            HashMap::new()
        }
    } else {
        HashMap::new()
    }
    .into_iter()
    .filter(|(k, _)| !sensitive_prefixes.iter().any(|prefix| k.to_lowercase().contains(prefix)))
    .collect();

    if let Some(name) = params.get("bot_name") {
        let is_public_val: bool = bots::table
            .filter(bots::name.eq(name))
            .select(bots::is_public)
            .first(&mut conn)
            .unwrap_or(false);
        map.insert("is_public".to_string(), if is_public_val { "true".to_string() } else { "false".to_string() });
    }

    Ok(axum::Json(serde_json::to_value(&map).unwrap_or_default()))
}
