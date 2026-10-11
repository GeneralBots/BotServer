use super::*;

pub(crate) fn make_saas_router(app_state: &Arc<AppState>) -> Router<()> {
    use botcloud::{SaasService, SaasConfig, stripe::StripeClient, cloud_ui, api};
    let stripe_secret = match std::env::var("STRIPE_SECRET_KEY") {
        Ok(key) => key,
        Err(_) => {
            tracing::info!("STRIPE_SECRET_KEY not set — Stripe operations will fail at runtime");
            String::new()
        }
    };
    let stripe = StripeClient::new(stripe_secret, None);

    // Configure mc alias from AppState drive config (loaded from Vault)
    // Resolve the real MinIO client: bare `mc` may be GNU Midnight Commander,
    // whose invocation silently broke bucket provisioning at signup (#1371).
    let mc_path = {
        let configured = std::env::var("MC_PATH").unwrap_or_default();
        let from_config = botcloud::integration::resolve_mc_binary(&configured);
        // resolve_mc_binary falls back to `configured` when nothing exists on
        // disk; keep /tmp/mc as the last-resort legacy default in that case.
        if from_config == "mc" { "/tmp/mc".to_string() } else { from_config }
    };
    let mc_alias = std::env::var("MC_ALIAS").unwrap_or_else(|_| "local".to_string());
    if let Some(ref cfg) = app_state.config {
        let endpoint = &cfg.drive.endpoint;
        let access_key = &cfg.drive.access_key;
        let secret_key = &cfg.drive.secret_key;
        if !access_key.is_empty() && !secret_key.is_empty() {
            match botsecurity::command_guard::SafeCommand::new(&mc_path)
                .and_then(|c| c.arg("alias"))
                .and_then(|c| c.arg("set"))
                .and_then(|c| c.arg(&mc_alias))
                .and_then(|c| c.arg(endpoint))
                .and_then(|c| c.arg(access_key))
                .and_then(|c| c.arg(secret_key))
                .and_then(|c| c.arg("--api"))
                .and_then(|c| c.arg("s3v4"))
            {
                Ok(cmd) => match cmd.execute() {
                    Ok(out) if out.status.success() => {
                        tracing::info!("mc alias '{mc_alias}' configured via {mc_path}");
                    }
                    Ok(out) => {
                        let stderr = String::from_utf8_lossy(&out.stderr);
                        tracing::error!("mc alias set failed ({}): {}", mc_path, stderr.trim());
                    }
                    Err(e) => tracing::error!("mc alias set exec failed: {e}"),
                },
                Err(e) => tracing::error!("mc alias set guard rejected: {e}"),
            }
        } else {
            tracing::info!("Drive credentials from Vault are empty, mc alias not configured");
        }
    }

    // Load all SaaS config from directory_config.json (written during init from Vault)
    let (base_url, jwt_secret, templates_dir, mc_path, mc_alias,
         directory_api_url, directory_service_token, directory_external_domain,
         directory_allow_insecure_http) = {
        let config_path = format!("{}/conf/system/directory_config.json", botcore::shared::utils::get_stack_path());
        match std::fs::read_to_string(&config_path) {
            Ok(content) => {
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(json) => {
                        let j = |key: &str| json.get(key).and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string());
                        let configured_token = j("service_token");
                        let api_url = j("base_url");
                        let service_token = resolve_directory_service_token(
                            api_url.as_deref().unwrap_or_default(),
                            configured_token.clone(),
                        );
                        if service_token != configured_token {
                            // Persist the working token so future boots reuse it.
                            if let Some(new_token) = &service_token {
                                if let Ok(mut json) = content.clone().parse::<serde_json::Value>() {
                                    json["service_token"] = serde_json::Value::String(new_token.clone());
                                    let _ = std::fs::write(&config_path, serde_json::to_string_pretty(&json).unwrap_or(content.clone()));
                                }
                            }
                        }
                        (
                            j("saas_base_url").unwrap_or_default(),
                            j("saas_jwt_secret").unwrap_or_else(crate::main_module::directory_setup::resolve_saas_jwt_secret),
                            j("bot_templates_dir").unwrap_or_else(|| "work/templates/bots".to_string()),
                            j("mc_path").map(|p| botcloud::integration::resolve_mc_binary(&p)).unwrap_or_else(|| "/tmp/mc".to_string()),
                            j("mc_alias").unwrap_or_else(|| "local".to_string()),
                            api_url,
                            service_token,
                            j("external_domain"),
                            json.get("allow_insecure_http").and_then(|v| v.as_bool()).unwrap_or(false),
                        )
                    }
                    Err(_) => (String::new(), crate::main_module::directory_setup::resolve_saas_jwt_secret(), "work/templates/bots".to_string(),
                              "/tmp/mc".to_string(), "local".to_string(), None, None, None, false),
                }
            }
            Err(_) => (String::new(), crate::main_module::directory_setup::resolve_saas_jwt_secret(), "work/templates/bots".to_string(),
                      "/tmp/mc".to_string(), "local".to_string(), None, None, None, false),
        }
    };
    let saas_config = SaasConfig {
        base_url,
        jwt_secret,
        mc_path,
        mc_alias,
        templates_dir,
        directory_api_url,
        directory_service_token,
        directory_external_domain,
        directory_allow_insecure_http,
    };
    let saas_config_for_api = saas_config.clone();
    let saas_service = Arc::new(SaasService::new(
        Arc::new(botbilling::api::BillingApiState {
            pool: Arc::new(app_state.conn.clone()),
            get_default_bot: Some(botbilling::query_first_bot as fn(&mut diesel::PgConnection) -> uuid::Uuid),
        }),
        stripe,
        saas_config,
    ));
    Router::new()
        .merge(cloud_ui::configure_cloud_ui_routes().with_state(saas_service.clone()))
        .merge(api::configure_cloud_api_routes(saas_config_for_api).with_state(saas_service.clone()))
        .merge(botcloud::webhook::configure_webhook_routes().with_state(saas_service))
}
#[cfg(feature = "marketing")]
pub(crate) fn make_marketing_router(app_state: &Arc<AppState>) -> Router<()> {
    let base = botmarketing::state::AppState {
        conn: Arc::new(app_state.conn.clone()),
        // #1441 B — a bot id, not a branch id: `botmarketing::state::get_scope`
        // resolves org/branch through `bots.id`, so the previous value made
        // every campaign land in the nil branch and the CRM Campaigns tab stayed
        // empty.
        get_default_bot: Arc::new(|conn: &mut diesel::PgConnection| {
            let bot = resolve_default_bot_id(conn);
            (bot, "default".to_string())
        }),
        send_email: Arc::new(|_: &str, _: &str, _: &str, _: uuid::Uuid, _: Option<&str>| -> Result<String, String> { Ok("stub".to_string()) }),
        send_whatsapp: Arc::new(|_: uuid::Uuid, _: &str, _: &str, _: Option<&str>, _: Option<&str>| -> Result<String, String> { Ok("stub".to_string()) }),
        get_config: Arc::new(|_: &uuid::Uuid, _: &str, _: Option<&str>| -> Result<String, String> { Ok("stub".to_string()) }),
        llm_generate: Arc::new(|_: &str, _: &serde_json::Value, _: &str, _: &str| -> Result<String, String> { Ok("stub".to_string()) }),
        worker: None,
    };
    let worker_state = Arc::new(base.clone());
    let marketing_state = Arc::new(
        base.with_worker(botmarketing::campaign::CampaignWorker::new(worker_state)),
    );
    crate::marketing::routes::configure_marketing_routes().with_state(marketing_state)
}
#[cfg(feature = "telegram")]
pub(crate) fn make_telegram_router(app_state: &Arc<AppState>) -> Router<()> {
    crate::telegram::webhook::configure().with_state(Arc::new(bottelegram::ChannelState {
        conn: Arc::new(app_state.conn.clone()),
        get_default_bot: Arc::new(|conn: &mut diesel::PgConnection| {
            let branch = resolve_default_branch(conn);
            (branch, "default".to_string())
        }),
        get_config: crate::main_module::routes::channel_support::make_channel_config_reader(app_state),
        put_media: crate::main_module::routes::channel_support::make_put_media_fn(app_state),
        stream_response: {
            let app_state = app_state.clone();
            Arc::new(move |msg: botlib::models::UserMessage, tx: tokio::sync::mpsc::Sender<botlib::models::BotResponse>| {
                let state = app_state.clone();
                tokio::spawn(async move {
                    let sink = crate::core::bot::pipeline::MpscChannelSink(tx);
                    let _ = crate::core::bot::pipeline::run_pipeline_for_channel(
                        &state, &msg, &sink,
                    ).await.map_err(|e| e.to_string())?;
                    Ok(())
                })
            })
        },
        attendant_broadcast: None,
    }))
}
#[cfg(feature = "instagram")]
pub(crate) fn make_instagram_router(app_state: &Arc<AppState>) -> Router<()> {
    crate::instagram::webhook::configure().with_state(Arc::new(botinstagram::state::ChannelState {
        get_config: crate::main_module::routes::channel_support::make_channel_config_reader_by_handle(app_state),
        stream_response: {
            let app_state = app_state.clone();
            Arc::new(move |msg: botlib::models::UserMessage, tx: tokio::sync::mpsc::Sender<botlib::models::BotResponse>| {
                let state = app_state.clone();
                tokio::spawn(async move {
                    let sink = crate::core::bot::pipeline::MpscChannelSink(tx);
                    let _ = crate::core::bot::pipeline::run_pipeline_for_channel(
                        &state, &msg, &sink,
                    ).await.map_err(|e| e.to_string())?;
                    Ok(())
                })
            })
        },
        attendant_broadcast: None,
    }))
}
#[cfg(feature = "msteams")]
pub(crate) fn make_msteams_router(app_state: &Arc<AppState>) -> Router<()> {
    crate::msteams::webhook::configure().with_state(Arc::new(botmsteams::state::ChannelState {
        conn: Arc::new(app_state.conn.clone()),
        get_default_bot: Arc::new(|conn: &mut diesel::PgConnection| {
            let branch = resolve_default_branch(conn);
            (branch, "default".to_string())
        }),
        get_config: crate::main_module::routes::channel_support::make_channel_config_reader(app_state),
        stream_response: {
            let app_state = app_state.clone();
            Arc::new(move |msg: botlib::models::UserMessage, tx: tokio::sync::mpsc::Sender<botlib::models::BotResponse>| {
                let state = app_state.clone();
                tokio::spawn(async move {
                    let sink = crate::core::bot::pipeline::MpscChannelSink(tx);
                    let _ = crate::core::bot::pipeline::run_pipeline_for_channel(
                        &state, &msg, &sink,
                    ).await.map_err(|e| e.to_string())?;
                    Ok(())
                })
            })
        },
        attendant_broadcast: None,
    }))
}
#[cfg(feature = "attendant")]
pub(crate) fn make_attendant_router(app_state: &Arc<AppState>) -> Router<()> {
    Router::new()
        .merge(crate::attendance::routes::configure_attendance_routes().with_state(Arc::new(botattendance::AttendanceConfig {
            pool: Arc::new(app_state.conn.clone()),
            master_key: botsecurity_crypto::encryption::load_master_encryption_key(),
            get_default_bot: Arc::new(|conn: &mut _| resolve_default_branch(conn)),
            llm_generate: Arc::new(|_: &str, _: &serde_json::Value, _: &str, _: &str| -> Result<String, Box<dyn std::error::Error + Send + Sync>> { Ok(String::new()) }),
            process_content: Arc::new(|_: &str, _: &str| -> String { String::new() }),
            config_get: Arc::new(|_: &uuid::Uuid, _: &str| -> String { String::new() }),
            send_bot_response: None,
            broadcast_notification: None,
            save_message: None,
        })))
}
