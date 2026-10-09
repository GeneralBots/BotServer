use super::*;

/// Persist a shipped template's files to `{bot}.gbai/{bot}.gbdialog/` and
/// return the standard create-and-execute response.
pub(super) async fn persist_shipped_template(
    api: &Arc<AutoTaskApi>,
    bot_id: Uuid,
    intent: &str,
    template: crate::templates::ShippedTemplate,
) -> Json<CreateAndExecuteResponse> {
    info!(
        "create-and-execute matched shipped template '{}' for intent: {}",
        template.name,
        &intent[..intent.len().min(80)]
    );
    let info = match resolve_bot_info(api.state().db_pool(), bot_id) {
        Ok(Some(info)) => info,
        Ok(None) => {
            let e: Box<dyn std::error::Error + Send + Sync> = "bot not found for classification".into();
            return error_create(intent, &*e);
        }
        Err(e) => {
            let e: Box<dyn std::error::Error + Send + Sync> = e.into();
            return error_create(intent, &*e);
        }
    };
    // Reform #1505 — persist the template into the bot's repository (ALM),
    // the canonical `.gbdialog`; Drive is the legacy fallback.
    if let Some(result) = crate::source_persist::persist_to_git(
        api.state().as_ref(),
        &info,
        bot_id,
        template.tool_path,
        template.tool_source,
        template.manifest_path.zip(template.manifest_source),
        &format!("autotask: add '{}' template", template.name),
    ) {
        let persisted = match result {
            Ok(p) => p,
            Err(e) => {
                let e: Box<dyn std::error::Error + Send + Sync> = e.into();
                return error_create(intent, &*e);
            }
        };
        info!(
            "Committed shipped template '{}' to ALM: {}",
            template.name, persisted.tool_key
        );
        record_auto_task(
            api.state().db_pool(),
            bot_id,
            &format!("{} ({})", template.name, template.tool_path),
            intent,
            "completed",
            Some(&persisted.tool_key),
        );
        let mut created = vec![crate::api::CreatedResourceResponse {
            resource_type: "tool".to_string(),
            name: template
                .tool_path
                .trim_start_matches("tools/")
                .trim_end_matches(".bas")
                .to_string(),
            path: Some(persisted.tool_key.clone()),
        }];
        // The channel prompts are what make the model call the tool; without
        // them the tool is committed but never invoked, so they ship with it.
        if let Some(config) = crate::source_persist::persist_config_to_git(
            api.state().as_ref(),
            &info,
            bot_id,
            template.config_files,
            &format!("autotask: add '{}' channel prompts", template.name),
        ) {
            match config {
                Ok(keys) if !keys.is_empty() => {
                    info!(
                        "Committed '{}' channel prompts to ALM: {}",
                        template.name,
                        keys.join(", ")
                    );
                    created.push(crate::api::CreatedResourceResponse {
                        resource_type: "bot-config".to_string(),
                        name: keys
                            .iter()
                            .map(|k| k.rsplit('/').next().unwrap_or_default().to_string())
                            .collect::<Vec<String>>()
                            .join(", "),
                        path: Some(
                            keys[0]
                                .rsplit_once('/')
                                .map(|(folder, _)| format!("{folder}/"))
                                .unwrap_or_default(),
                        ),
                    });
                }
                Ok(_) => {}
                // The tool is already committed; report the config failure
                // instead of aborting, so the user can retry the prompts.
                Err(e) => warn!("autotask: channel prompts not committed: {e}"),
            }
        }
        if let Some(key) = &persisted.manifest_key {
            created.push(crate::api::CreatedResourceResponse {
                resource_type: "mcp-manifest".to_string(),
                name: key.rsplit('/').next().unwrap_or_default().to_string(),
                path: Some(key.clone()),
            });
        }
        let tables = if persisted.tables.is_empty() {
            String::new()
        } else {
            format!(" (tables attached to tables.bas: {})", persisted.tables.join(", "))
        };
        return Json(CreateAndExecuteResponse {
            success: true,
            task_id: Uuid::new_v4().to_string(),
            status: "created".to_string(),
            message: format!(
                "Automation created from shipped template '{}' in the bot repository: {}{tables}",
                template.name, persisted.tool_key
            ),
            app_url: None,
            created_resources: created,
            pending_items: Vec::new(),
            error: None,
        });
    }
    let ops = match api.state().file_ops() {
        Some(ops) => ops,
        None => {
            let e: Box<dyn std::error::Error + Send + Sync> =
                "Drive ops not available — cannot persist shipped template".into();
            return error_create(intent, &*e);
        }
    };
    let bucket = info.bucket_name();
    let dialog = info.dialog_folder();
    let mut created = Vec::new();
    let tool_key = format!("{dialog}/{}", template.tool_path);
    if let Err(e) = ops.put_object(
        &bucket,
        &tool_key,
        template.tool_source.as_bytes().to_vec(),
        "text/plain",
    ) {
        let e: Box<dyn std::error::Error + Send + Sync> = format!("drive put failed: {e}").into();
        return error_create(intent, &*e);
    }
    info!("Saved BASIC file to Drive: {bucket}/{tool_key}");
    created.push(crate::api::CreatedResourceResponse {
        resource_type: "tool".to_string(),
        name: template
            .tool_path
            .trim_start_matches("tools/")
            .trim_end_matches(".bas")
            .to_string(),
        path: Some(tool_key.clone()),
    });
    if let (Some(rel), Some(src)) = (template.manifest_path, template.manifest_source) {
        // Persist the manifest next to the tool AND as the sibling root-level
        // copy the compiler reads: compile_file() regenerates a manifest with
        // an empty schema on every .bas compile, and the drive compiler syncs
        // the Drive copy over the generated one — without this root copy the
        // schema (and tool_exec's missing-argument defaulting) is lost.
        let manifest_key = format!("{dialog}/{rel}");
        let root_manifest_key = format!(
            "{dialog}/{}",
            rel.trim_start_matches("tools/")
        );
        let mut persisted = false;
        if let Err(e) = ops.put_object(
            &bucket,
            &manifest_key,
            src.as_bytes().to_vec(),
            "application/json",
        ) {
            warn!("shipped template manifest persist failed (tool kept): {e}");
        } else {
            persisted = true;
            info!("Saved MCP manifest to Drive: {bucket}/{manifest_key}");
            created.push(crate::api::CreatedResourceResponse {
                resource_type: "mcp-manifest".to_string(),
                name: rel.trim_start_matches("tools/").to_string(),
                path: Some(manifest_key.clone()),
            });
        }
        if persisted && root_manifest_key != manifest_key {
            if let Err(e) = ops.put_object(
                &bucket,
                &root_manifest_key,
                src.as_bytes().to_vec(),
                "application/json",
            ) {
                warn!("root MCP manifest persist failed (schema may be lost on recompile): {e}");
            } else {
                info!("Saved root MCP manifest to Drive: {bucket}/{root_manifest_key}");
            }
        }
    }
    Json(CreateAndExecuteResponse {
        success: true,
        task_id: Uuid::new_v4().to_string(),
        status: "created".to_string(),
        message: format!(
            "Automation created from shipped template '{}': {bucket}/{tool_key}",
            template.name
        ),
        app_url: None,
        created_resources: created,
        pending_items: Vec::new(),
        error: None,
    })
}
pub(super) fn error_create(intent: &str, e: &dyn std::error::Error) -> Json<CreateAndExecuteResponse> {
    let msg = err_msg("create_and_execute", e);
    warn!("create failed for intent: {intent}");
    Json(CreateAndExecuteResponse {
        success: false,
        task_id: String::new(),
        status: "failed".to_string(),
        message: msg.clone(),
        app_url: None,
        created_resources: Vec::new(),
        pending_items: Vec::new(),
        error: Some(msg),
    })
}
/// Insert an `auto_tasks` row recording a created automation. The row is what
/// makes an AutoTask item traceable: the list UI reads these rows, and the
/// ✏️ action opens the `.bas` it produced.
pub(crate) fn record_auto_task(
    pool: &DbPool,
    bot_id: Uuid,
    title: &str,
    intent: &str,
    status: &str,
    script_path: Option<&str>,
) {
    // branch_id is NOT NULL (6.5.23); resolve it from the bot's row.
    let outcome = pool.get().ok().map(|mut conn| {
        let branch_id: Option<Uuid> = sql_query("SELECT branch_id AS id FROM bots WHERE id = $1")
            .bind::<DieselUuid, _>(bot_id)
            .get_result::<BotBranchRow>(&mut conn)
            .ok()
            .and_then(|r| r.id);
        sql_query(
            "INSERT INTO auto_tasks (bot_id, branch_id, title, intent, status, basic_program, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW()) RETURNING id",
        )
        .bind::<DieselUuid, _>(bot_id)
        .bind::<NullableSql<DieselUuid>, _>(branch_id)
        .bind::<Text, _>(title)
        .bind::<Text, _>(intent)
        .bind::<Text, _>(status)
        .bind::<NullableSql<Text>, _>(script_path)
        .get_result::<TaskIdRow>(&mut conn)
    });
    match outcome {
        Some(Ok(row)) => info!("Recorded auto_tasks row {} for '{title}'", row.id),
        Some(Err(e)) => warn!("Failed to record auto_tasks row for '{title}': {e}"),
        None => warn!("DB pool unavailable — auto_tasks row for '{title}' not recorded"),
    }
}
