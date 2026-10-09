use super::*;

/// #1441 B — resolves the **bot id** of the default bot. `botmarketing` feeds
/// this value into `get_scope()` (`SELECT org_id, branch_id FROM bots WHERE id =
/// $1`) and into email/trigger writers, so handing it a branch id made every
/// lookup miss: campaigns were written with `branch_id = nil` and the CRM
/// Campaigns view (branch-scoped) showed nothing.
pub(super) fn resolve_default_bot_id(conn: &mut diesel::PgConnection) -> uuid::Uuid {
    use diesel::prelude::*;
    diesel::sql_query("SELECT id FROM bots WHERE is_default_for_branch = TRUE LIMIT 1")
        .get_result::<DefaultBotRow>(conn)
        .optional()
        .ok()
        .flatten()
        .map(|row| row.id)
        .unwrap_or_else(uuid::Uuid::nil)
}
/// Resolves a working Zitadel management token for the SaaS cloud API.
///
/// The configured `service_token` may be stale or invalid (it is a leftover
/// from an older setup and is never renewed). Fall back to the long-lived
/// bootstrap admin PAT (`conf/directory/admin-pat.txt`) and validate the
/// chosen token against the management API so signup user creation and login
/// password checks never run with a dead token.
pub(super) fn resolve_directory_service_token(api_url: &str, configured: Option<String>) -> Option<String> {
    let mut candidates: Vec<String> = Vec::new();
    if let Some(token) = configured.as_ref() {
        if !token.is_empty() {
            candidates.push(token.clone());
        }
    }
    let pat_path = format!(
        "{}/conf/directory/admin-pat.txt",
        botcore::shared::utils::get_stack_path()
    );
    if let Ok(pat) = std::fs::read_to_string(&pat_path) {
        let pat = pat.trim().to_string();
        if !pat.is_empty() {
            candidates.push(pat);
        }
    }
    if api_url.is_empty() || candidates.is_empty() {
        return configured;
    }
    // The probe uses reqwest::blocking, which owns a tokio runtime; running it
    // from the async boot task would drop that runtime in a blocking-forbidden
    // context and panic. Probe from a dedicated thread instead.
    let api_url = api_url.to_string();
    let valid = std::thread::spawn(move || -> Option<String> {
        for token in &candidates {
            let probe_ok = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .ok()
                .and_then(|client| {
                    client
                        .post(format!("{api_url}/management/v1/users/_search"))
                        .header("Authorization", format!("Bearer {token}"))
                        .json(&serde_json::json!({}))
                        .send()
                        .ok()
                })
                .map(|response| response.status().is_success())
                .unwrap_or(false);
            if probe_ok {
                return Some(token.clone());
            }
            tracing::warn!("Directory service token rejected by Zitadel, trying next candidate");
        }
        tracing::error!("No valid directory service token found - SaaS signup/login will fail");
        None
    })
    .join()
    .unwrap_or(None);
    if let Some(token) = valid {
        if configured.as_deref() != Some(token.as_str()) {
            tracing::info!("Using admin PAT as directory service token (configured token was invalid)");
        }
        return Some(token);
    }
    configured
}
pub(super) fn resolve_default_branch_pool(
    _pool: &diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>,
) -> (uuid::Uuid, uuid::Uuid) {
    let mut conn = match _pool.get() {
        Ok(c) => c,
        Err(_) => return (uuid::Uuid::nil(), uuid::Uuid::nil()),
    };
    let branch = resolve_default_branch(&mut conn);
    (branch, branch)
}
#[cfg(feature = "paper")]
pub(super) fn make_paper_state(app_state: &Arc<AppState>) -> Arc<crate::paper::state::PaperState> {
    let drive = app_state.drive.clone();
    let llm_provider = app_state.llm_provider.clone();

    let s3_put = {
        let drive = drive.clone();
        Arc::new(move |bucket: &str, key: &str, data: Vec<u8>, content_type: Option<&str>| {
            let drive = drive.clone();
            let bucket = bucket.to_string();
            let key = key.to_string();
            let ct = content_type.map(|s| s.to_string());
            Box::pin(async move {
                match drive.as_ref() {
                    Some(d) => d.put_object(&bucket, &key, data, ct.as_deref()).await,
                    None => Err("Drive service not available".to_string()),
                }
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>>
        }) as crate::paper::state::S3PutFn
    };

    let s3_get = {
        let drive = drive.clone();
        Arc::new(move |bucket: &str, key: &str| {
            let drive = drive.clone();
            let bucket = bucket.to_string();
            let key = key.to_string();
            Box::pin(async move {
                match drive.as_ref() {
                    Some(d) => d.get_object(&bucket, &key).await,
                    None => Err("Drive service not available".to_string()),
                }
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, String>> + Send>>
        }) as crate::paper::state::S3GetFn
    };

    let s3_delete = {
        let drive = drive.clone();
        Arc::new(move |bucket: &str, key: &str| {
            let drive = drive.clone();
            let bucket = bucket.to_string();
            let key = key.to_string();
            Box::pin(async move {
                match drive.as_ref() {
                    Some(d) => d.delete_object(&bucket, &key).await,
                    None => Err("Drive service not available".to_string()),
                }
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>>
        }) as crate::paper::state::S3DeleteFn
    };

    let s3_list = {
        let drive = drive.clone();
        Arc::new(move |bucket: &str, prefix: &str| {
            let drive = drive.clone();
            let bucket = bucket.to_string();
            let prefix = prefix.to_string();
            Box::pin(async move {
                match drive.as_ref() {
                    Some(d) => d.list_objects(&bucket, Some(&prefix)).await,
                    None => Err("Drive service not available".to_string()),
                }
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>, String>> + Send>>
        }) as crate::paper::state::S3ListFn
    };

    let call_llm = {
        let llm = llm_provider.clone();
        Arc::new(move |prompt: &str, context: &str| {
            let llm = llm.clone();
            let prompt = prompt.to_string();
            let context = context.to_string();
            Box::pin(async move {
                match llm.as_ref() {
                    Some(l) => l.generate_with_context(&prompt, &context).await,
                    None => Err("LLM service not available".to_string()),
                }
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>>
        }) as crate::paper::state::CallLlmFn
    };

    Arc::new(crate::paper::state::PaperState {
        conn: app_state.conn.clone(),
        bucket_name: app_state.bucket_name.clone(),
        s3_put,
        s3_get,
        s3_delete,
        s3_list,
        call_llm,
    })
}
#[cfg(feature = "designer")]
pub(super) fn make_designer_router(app_state: &Arc<AppState>) -> Router<()> {
    let make_state = || Arc::new(botdesigner::DesignerState {
        conn: Arc::new(app_state.conn.clone()),
        get_default_bot: Arc::new(|conn: &mut diesel::PgConnection| {
            let branch = resolve_default_branch(conn);
            (branch, "default".to_string())
        }),
        get_designer_error_context: Arc::new(|_err: &str| -> Option<String> { None }),
        get_content_type: Arc::new(|_p: &str| -> &'static str { "text/html" }),
        get_stack_path: Arc::new(|| "/opt/gbo/stack".to_string()),
        load_from_drive: Arc::new(|_: &str, _: &str| -> Result<String, String> { Err("not available".to_string()) }),
        write_to_drive: Arc::new(|_: &str, _: &str, _: &[u8], _: &str| -> Result<(), String> { Err("not available".to_string()) }),
        call_llm: Arc::new(|_: &str, _: &serde_json::Value| -> Result<String, String> { Ok(String::new()) }),
        get_config: Arc::new(|_: &str, _: &str, _: Option<&str>| -> Result<String, String> { Ok(String::new()) }),
        bucket_name: app_state.bucket_name.clone(),
        site_path: None,
    });
    let state = make_state();
    Router::new()
        .merge(crate::designer::designer_api::configure_designer_routes().with_state(state.clone()))
        .merge(crate::designer::ui::configure_designer_ui_routes().with_state(state.clone()))
        .merge(crate::designer::plugin_manifest::configure_plugin_routes().with_state(state))
}
#[cfg(feature = "dashboards")]
pub(super) fn make_dashboards_router(app_state: &Arc<AppState>) -> Router<()> {
    pub(super) fn default_bot_fn(conn: &mut diesel::PgConnection) -> uuid::Uuid {
        resolve_default_branch(conn)
    }
    Router::new()
        .merge(crate::dashboards::configure_dashboards_routes(Arc::new(botdashboards::DashboardsState {
            pool: app_state.conn.clone(),
            get_default_bot: default_bot_fn,
        })))
        .merge(crate::dashboards::ui::configure_dashboards_ui_routes().with_state(Arc::new(botdashboards::DashboardsState {
            pool: app_state.conn.clone(),
            get_default_bot: default_bot_fn,
        })))
}
#[cfg(feature = "canvas")]
pub(super) fn make_canvas_router(app_state: &Arc<AppState>) -> Router<()> {
    let make_state = || Arc::new(botcanvas::CanvasState {
        pool: Arc::new(app_state.conn.clone()),
        get_bot_context: Arc::new(
            resolve_default_branch_pool as fn(&diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>) -> (uuid::Uuid, uuid::Uuid),
        ),
    });
    let state = make_state();
    Router::new()
        .merge(crate::canvas::configure_canvas_routes().with_state(state.clone()))
        .merge(crate::canvas::configure_canvas_ui_routes().with_state(state))
}
#[cfg(feature = "social")]
pub(super) fn make_social_router(app_state: &Arc<AppState>) -> Router<()> {
    let make_state = || Arc::new(botsocial::SocialState {
        pool: Arc::new(app_state.conn.clone()),
        get_default_bot: Arc::new(|conn: &mut _| {
            let branch = resolve_default_branch(conn);
            (branch, "default".to_string())
        }),
    });
    let state = make_state();
    Router::new()
        .merge(crate::social::configure_social_routes().with_state(state.clone()))
        .merge(crate::social::configure_social_ui_routes().with_state(state))
}
pub(super) fn make_billing_router(app_state: &Arc<AppState>) -> Router<()> {
    let make_state = || Arc::new(botbilling::api::BillingApiState {
        pool: Arc::new(app_state.conn.clone()),
        get_default_bot: Some(
            (|conn: &mut diesel::PgConnection| {
                use diesel::prelude::*;
                #[derive(diesel::QueryableByName)]
                #[diesel(check_for_backend(diesel::pg::Pg))]
                struct BranchRow {
                    #[diesel(sql_type = diesel::sql_types::Uuid)]
                    branch_id: uuid::Uuid,
                }
                diesel::sql_query(
                    "SELECT branch_id FROM bots WHERE is_default_for_branch = TRUE ORDER BY created_at ASC LIMIT 1",
                )
                .get_result::<BranchRow>(conn)
                .map(|r| r.branch_id)
                .unwrap_or_else(|_| uuid::Uuid::nil())
            }) as fn(&mut diesel::PgConnection) -> uuid::Uuid,
        ),
    });
    Router::new()
        .merge(crate::billing::billing_ui::configure_billing_routes().with_state(make_state()))
        .merge(crate::billing::billing_admin::configure_admin_billing_routes().with_state(make_state()))
        .merge(crate::billing::erp_ui::configure_erp_ui_routes().with_state(make_state()))
        .merge(crate::billing::api::configure_billing_api_routes().with_state(make_state()))
}
