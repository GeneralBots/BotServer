use axum::Router;
use std::sync::Arc;
use botcore::shared::state::AppState;


/// Applies the second half of the feature-block merges (legal, compliance,
/// monitoring, scripting, people, vibe, project, goals, sheet, canvas,
/// fraud, inventory, gl, retail, banking, weba and the channel routers)
/// onto the api/sub routers.
pub(super) async fn merge_part_b(
    app_state: &Arc<AppState>,
    api_router: &mut Router<Arc<AppState>>,
    sub_router: &mut Router<()>,
) {
    #[cfg(feature = "legal")]
    {
        let legal_pool = app_state.conn.clone();
        *sub_router = sub_router.clone().merge(crate::legal::configure_legal_routes().with_state(Arc::new(legal_pool.clone())));
        *sub_router = sub_router.clone().merge(crate::legal::configure_legal_ui_routes().with_state(Arc::new(legal_pool)));
    }

    #[cfg(feature = "compliance")]
    {
        let compliance_pool = app_state.conn.clone();
        *sub_router = sub_router.clone().merge(crate::compliance::configure_compliance_routes().with_state(Arc::new(compliance_pool.clone())));
        *sub_router = sub_router.clone().merge(crate::compliance::ui::configure_compliance_ui_routes().with_state(Arc::new(compliance_pool.clone())));
        *sub_router = sub_router.clone().merge(crate::compliance::dashboard::configure_dashboard_routes().with_state(Arc::new(compliance_pool)));
    }

    #[cfg(feature = "monitoring")]
    {
        struct MonitoringAppState {
            app_state: Arc<AppState>,
            collector: Arc<crate::monitoring::MetricsCollector>,
        }

        impl crate::monitoring::MonitoringState for MonitoringAppState {
            fn active_session_count(&self) -> usize {
                use diesel::prelude::*;
                let mut conn = match self.app_state.conn.get() {
                    Ok(c) => c,
                    Err(_) => return 0,
                };

                #[derive(diesel::QueryableByName)]
                struct CountResult {
                    #[diesel(sql_type = diesel::sql_types::BigInt)]
                    count: i64,
                }

                diesel::sql_query(
                    "SELECT COUNT(*) as count FROM user_sessions WHERE expires_at IS NULL OR expires_at > NOW()",
                )
                .get_result::<CountResult>(&mut conn)
                .map(|r| r.count.max(0) as usize)
                .unwrap_or(0)
            }

            fn is_db_healthy(&self) -> bool {
                use diesel::prelude::*;
                let mut conn = match self.app_state.conn.get() {
                    Ok(c) => c,
                    Err(_) => return false,
                };
                diesel::sql_query("SELECT 1").execute(&mut conn).is_ok()
            }

            fn metrics_collector(&self) -> Arc<crate::monitoring::MetricsCollector> {
                self.collector.clone()
            }

            fn dependencies(&self) -> Vec<crate::monitoring::DependencyStatus> {
                use std::time::Instant;
                let mut deps = Vec::new();

                // PostgreSQL
                {
                    let start = Instant::now();
                    let healthy = self.is_db_healthy();
                    let latency = start.elapsed().as_secs_f64() * 1000.0;
                    deps.push(crate::monitoring::DependencyStatus {
                        name: "PostgreSQL",
                        host: "postgres".to_string(),
                        healthy,
                        latency_ms: latency,
                    });
                }

                // Cache / Valkey
                if let Some(cache) = self.app_state.cache.as_ref() {
                    let start = Instant::now();
                    let healthy = cache
                        .get_connection_with_timeout(std::time::Duration::from_millis(750))
                        .and_then(|mut conn| {
                            redis::cmd("PING").query::<String>(&mut conn)
                        })
                        .map(|reply| reply.to_uppercase() == "PONG")
                        .unwrap_or(false);
                    let latency = start.elapsed().as_secs_f64() * 1000.0;
                    deps.push(crate::monitoring::DependencyStatus {
                        name: "Valkey",
                        host: "cache".to_string(),
                        healthy,
                        latency_ms: latency,
                    });
                }

                // MinIO / Drive
                if let Some(drive) = self.app_state.drive.as_ref() {
                    let start = Instant::now();
                    let healthy = tokio::task::block_in_place(|| {
                        futures::executor::block_on(drive.list_all_buckets())
                            .map(|buckets| !buckets.is_empty())
                            .unwrap_or(false)
                    });
                    let latency = start.elapsed().as_secs_f64() * 1000.0;
                    deps.push(crate::monitoring::DependencyStatus {
                        name: "MinIO",
                        host: "drive".to_string(),
                        healthy,
                        latency_ms: latency,
                    });
                }

                // LLM
                {
                    let url = std::env::var("LLM_URL")
                        .or_else(|_| std::env::var("OLLAMA_HOST"))
                        .ok();
                    let start = Instant::now();
                    let healthy = url
                        .and_then(|u| crate::monitoring::host_port_from_url(&u, 8081))
                        .and_then(|(host, port)| {
                            use std::net::{SocketAddr, TcpStream};
                            let addr: SocketAddr = format!("{host}:{port}").parse().ok()?;
                            Some(TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(750)).is_ok())
                        })
                        .unwrap_or(false);
                    let latency = start.elapsed().as_secs_f64() * 1000.0;
                    deps.push(crate::monitoring::DependencyStatus {
                        name: "LLM",
                        host: "llm".to_string(),
                        healthy,
                        latency_ms: latency,
                    });
                }

                deps
            }
        }

        let collector = Arc::new(crate::monitoring::MetricsCollector::new());
        {
            let c = collector.clone();
            tokio::spawn(async move {
                c.setup_default_alert_rules().await;
                c.start_background_collection().await;
            });
        }

        let monitoring_state = Arc::new(MonitoringAppState {
            app_state: app_state.clone(),
            collector: collector.clone(),
        });
        *sub_router = sub_router.clone().merge(
            crate::monitoring::configure::<MonitoringAppState, crate::monitoring::DefaultMonitoringUrls>()
                .with_state(monitoring_state)
        );
        *sub_router = sub_router.clone().merge(
            crate::monitoring::governance::configure_routes(collector)
        );
    }

    #[cfg(feature = "scripting")]
    { *sub_router = sub_router.clone().merge(crate::basic::keywords::configure_app_server_routes().with_state(app_state.clone())); }

    #[cfg(feature = "people")]
    { *sub_router = sub_router.clone().merge(crate::basic::keywords::configure_db_routes().with_state(app_state.clone())); }

    #[cfg(feature = "vibe")]
    { *sub_router = sub_router.clone().merge(crate::vibe::configure_vibe_routes(app_state).await); }

    #[cfg(feature = "project")]
    {
        let project_service = Arc::new(crate::project::ProjectService::with_pool(app_state.conn.clone()));
        project_service.load_from_db().await;
        let project_router = crate::project::configure(project_service.clone());
        *sub_router = sub_router.clone().merge(project_router.with_state(project_service.clone()));
        *sub_router = sub_router.clone().merge(crate::project::project_ui::configure_project_ui_routes().with_state(project_service));
    }

    #[cfg(all(feature = "analytics", feature = "goals"))]
    {
        let goals_pool = Arc::new(app_state.conn.clone());
        let goals_bot_context: crate::analytics::GetBotContextFn = Arc::new(|| (uuid::Uuid::nil(), uuid::Uuid::nil()));
        let goals_default_bot: crate::analytics::GetDefaultBotFn = Arc::new(|_c: &mut diesel::PgConnection| uuid::Uuid::nil());
        *sub_router = sub_router.clone().merge(crate::analytics::goals::configure_goals_routes().with_state((goals_pool.clone(), goals_bot_context)));
        *sub_router = sub_router.clone().merge(crate::analytics::goals_ui::configure_goals_ui_routes().with_state((goals_pool, goals_default_bot)));
    }

    #[cfg(feature = "analytics")]
    {
        let reports_default_bot: crate::analytics::GetDefaultBotFn =
            Arc::new(|_c: &mut diesel::PgConnection| uuid::Uuid::nil());
        let reports_state = Arc::new(crate::analytics::reports::ReportsState {
            pool: Arc::new(app_state.conn.clone()),
            default_bot: Arc::new(reports_default_bot),
        });
        *sub_router = sub_router.clone().merge(
            crate::analytics::reports::configure_reports_routes().with_state(reports_state),
        );
    }

    #[cfg(feature = "sheet")]
    {
        let sheet_drive = app_state.drive.as_ref().map(|d| {
            Arc::new(crate::sheet::drive_adapter::DriveOpsAdapter(d.clone()))
                as Arc<dyn crate::sheet::state::DriveOps>
        });
        let mut sheet_state = crate::sheet::state::SheetState::new(sheet_drive.clone());
        // Wire xlsx save-back hook: every sheet save also writes back
        // to the original .xlsx in Drive (if loaded from one).
        if let Some(ref drive) = sheet_drive {
            sheet_state.on_save = Some(crate::sheet::storage::create_save_back_hook(drive.clone()));
        }
        let sheet_state = Arc::new(sheet_state);
        // Give the session store a state handle so idle-evicted dirty
        // sessions can persist before dropping.
        sheet_state.sessions.set_state_handle(sheet_state.clone());
        *sub_router = sub_router.clone().merge(
            crate::sheet::routes::configure_sheet_routes()
                .layer(axum::middleware::from_fn(
                    crate::sheet::user_middleware::sheet_user_middleware,
                ))
                .with_state(sheet_state),
        );
    }

    #[cfg(feature = "canvas")]
    { *sub_router = sub_router.clone().merge(super::feature_routers::make_canvas_router(app_state)); }

    #[cfg(feature = "fraud")]
    {
        let fraud_state = Arc::new(crate::fraud::FraudState::new(app_state.conn.clone()));
        *sub_router = sub_router.clone().merge(crate::fraud::configure_fraud_routes().with_state(fraud_state));
    }

    #[cfg(feature = "inventory")]
    {
        let inventory_state = Arc::new(crate::inventory::InventoryState { pool: app_state.conn.clone() });
        *sub_router = sub_router.clone().merge(crate::inventory::configure_inventory_routes().with_state(inventory_state));
    }

    #[cfg(feature = "gl")]
    {
        let gl_state = Arc::new(crate::gl::GlState { pool: app_state.conn.clone() });
        *sub_router = sub_router.clone().merge(crate::gl::configure_gl_routes().with_state(gl_state));
    }

    #[cfg(feature = "retail")]
    { *sub_router = sub_router.clone().merge(botretail::configure()); }

    #[cfg(feature = "banking")]
    { *api_router = api_router.clone().merge(botbanking::configure()); }

    #[cfg(feature = "m365")]
    { *api_router = api_router.clone().merge(botm365::configure()); }

    #[cfg(feature = "weba")]
    {
        let weba_state = Arc::new(crate::weba::WebaState::new());
        *sub_router = sub_router.clone().merge(crate::weba::configure_routes(weba_state));
    }

    *sub_router = sub_router.clone().merge(crate::directory::scim::server::configure_scim_routes().with_state(app_state.clone()));

    #[cfg(feature = "social")]
    { *sub_router = sub_router.clone().merge(super::feature_routers::make_social_router(app_state)); }

    #[cfg(feature = "learn")]
    {
        *sub_router = sub_router.clone().merge(crate::learn::ui::configure_learn_ui_routes());
        *sub_router = sub_router.clone().merge(crate::learn::creator::configure_learn_api_routes().with_state(Arc::new(botlearn::GamificationService::new())));
    }

    #[cfg(feature = "meet")]
    { *sub_router = sub_router.clone().merge(crate::meet::ui::configure_meet_ui_routes().with_state(app_state.clone())); }

    #[cfg(feature = "billing")]
    { *sub_router = sub_router.clone().merge(super::feature_routers::make_billing_router(app_state)); }

    #[cfg(feature = "saas")]
    { *sub_router = sub_router.clone().merge(super::feature_routers::make_saas_router(app_state)); }

    #[cfg(feature = "whatsapp")]
    { *sub_router = sub_router.clone().merge(crate::whatsapp::configure(app_state)); }

    #[cfg(feature = "marketing")]
    { *sub_router = sub_router.clone().merge(super::feature_routers::make_marketing_router(app_state)); }

    #[cfg(feature = "telegram")]
    { *sub_router = sub_router.clone().merge(super::feature_routers::make_telegram_router(app_state)); }

    #[cfg(feature = "instagram")]
    { *sub_router = sub_router.clone().merge(super::feature_routers::make_instagram_router(app_state)); }

    #[cfg(feature = "msteams")]
    { *sub_router = sub_router.clone().merge(super::feature_routers::make_msteams_router(app_state)); }

    #[cfg(feature = "sources")]
    {
        let sources_state = crate::sources::make_sources_state(app_state.conn.clone());
        *sub_router = sub_router.clone().merge(crate::sources::configure_sources_api_routes().with_state(sources_state));
    }

    #[cfg(feature = "attendant")]
    { *sub_router = sub_router.clone().merge(super::feature_routers::make_attendant_router(app_state)); }

    #[cfg(feature = "browser")]
    {
        use std::sync::Arc;
        use tokio::sync::Mutex;
        *sub_router = sub_router.clone().merge(
            crate::browser::api::configure_routes()
                .with_state(Arc::new(Mutex::new(std::collections::HashMap::new()))),
        );
    }

    #[cfg(feature = "player")]
    {
        *sub_router = sub_router.clone().merge(crate::player::configure_player_routes().with_state(app_state.clone()));
    }

    #[cfg(feature = "terminal")]
    { *sub_router = sub_router.clone().merge(crate::api::terminal::routes::configure_terminal_routes(app_state.conn.clone())); }

    // #1247 — Web Push subscription backend (Settings push toggle).
    { *sub_router = sub_router.clone().merge(crate::api::push::configure_push_routes(app_state.conn.clone())); }

    // AutoTask routes. The ops live in `crate::auto_task::ops` so the chat
    // command executor can build the same API for `tasks.autotask.create`.
    {
        let api = crate::autotask_host::ops::build_api(app_state);
        *sub_router = sub_router.clone().merge(botautotask::api::router(
            api.state().clone(),
            api.config_ops().clone(),
            api.llm_ops().clone(),
        ));
    }
}
