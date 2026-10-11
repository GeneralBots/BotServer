use axum::{routing::post, Router};
use std::sync::Arc;
use botcore::shared::state::AppState;


/// Applies the first half of the feature-block merges (directory, apps,
/// workspace tabs, integrations, AI OS routers, memory, editor, database,
/// tasks, analytics, docs, research) onto the api/sub routers. Every block
/// is an independent `#[cfg]` merge preserved verbatim from the original
/// monolithic `inner_build_sub_router`.
pub(super) async fn merge_part_a(
    app_state: &Arc<AppState>,
    port: u16,
    api_router: &mut Router<Arc<AppState>>,
    sub_router: &mut Router<()>,
) {
    {
        let directory_api_state = Arc::new(crate::directory::api::DirectoryApiState {
            conn: app_state.conn.clone(),
            base_url: format!("http://localhost:{}", port),
        });
        *api_router = api_router.clone().merge(crate::directory::api::configure_user_routes().with_state(directory_api_state));
    }

    *api_router = api_router.clone().merge(crate::apps::register(Router::new()));

    *api_router = api_router
        .clone()
        .merge(super::workspace_tabs::configure_workspace_tabs_routes());

    #[cfg(feature = "tax")]
    { *api_router = api_router.clone().merge(bottax::configure()); }

    #[cfg(feature = "vision")]
    { *api_router = api_router.clone().merge(botvision::configure()); }

    #[cfg(feature = "erp")]
    { *api_router = api_router.clone().merge(boterp::configure()); }

    // botsources owns the /api/integrations/* namespace (connectors + ETL);
    // the legacy botintegrations router that used to shadow-mount here is
    // deleted (#1350). The modern connection control plane below occupies a
    // different namespace (/api/bots/:bot_id/integration-*) and mounts
    // unconditionally.

    // Canonical tenant-scoped integration connection control plane (#939).
    // Mounted only when the secrets manager initializes; a Vault outage skips
    // the mount (fail closed) while the rest of the API keeps serving.
    #[cfg(feature = "integrations")]
    match botcoresecrets::SecretsManager::get_clone() {
        Ok(secrets_manager) => {
            let connections_state = std::sync::Arc::new(botintegrations::IntegrationState::new(
                app_state.conn.clone(),
                botintegrations::secrets::ConnectionVault::new(secrets_manager),
            ));
            *api_router = api_router
                .clone()
                .merge(botintegrations::configure_connection_routes().with_state(connections_state.clone()));
            botintegrations::automations::spawn(connections_state.clone());
            botintegrations::token_refresh::spawn(connections_state.clone());
        }
        Err(error) => {
            log::error!(
                "integration connection control plane not mounted (secrets manager init failed): {error}"
            );
        }
    }

    #[cfg(feature = "hr")]
    { *api_router = api_router.clone().merge(bothr::configure()); }

    #[cfg(feature = "sales")]
    { *api_router = api_router.clone().merge(botsales::configure()); }

    #[cfg(feature = "minutes")]
    { *api_router = api_router.clone().merge(botminutes::configure()); }

    #[cfg(feature = "templates")]
    { *api_router = api_router.clone().merge(bottemplates::configure()); }

    #[cfg(feature = "itsm")]
    { *api_router = api_router.clone().merge(botitsm::configure()); }

    #[cfg(feature = "pos")]
    { *api_router = api_router.clone().merge(botpos::configure()); }

    #[cfg(feature = "handoff")]
    { *api_router = api_router.clone().merge(bothandoff::configure()); }

    #[cfg(feature = "kyc")]
    { *api_router = api_router.clone().merge(botkyc::configure()); }

    #[cfg(feature = "timeclock")]
    { *api_router = api_router.clone().merge(bottimeclock::configure()); }

    #[cfg(feature = "contacts")]
    {
        let contacts_state = Arc::new(crate::contacts::CrateState {
            db_pool: app_state.conn.clone(),
            get_default_bot: Arc::new(|_c: &mut diesel::PgConnection| uuid::Uuid::nil()),
            trigger_contact_change: Arc::new(|_c: &mut diesel::PgConnection, _id: uuid::Uuid, _kind: &str, _by: uuid::Uuid| {}),
            trigger_deal_stage_change: Arc::new(
                |_c: &mut diesel::PgConnection, _id: uuid::Uuid, _old: &str, _new: &str, _by: uuid::Uuid| {},
            ),
        });
        // #1452 — seed default pipeline stages for branches with CRM activity
        // but no stage rows (idempotent; keeps the kanban data-driven).
        crate::contacts::stages::seed_default_stages(&contacts_state);
        *api_router = api_router.clone().merge(crate::contacts::routes::configure_all_routes().with_state(contacts_state));
    }

    // ===== AI OS routers (issues #1167-#1179) =====
    #[cfg(feature = "agent-vm")]
    {
        let agent_state = Arc::new(botagent::AgentService::new(app_state.conn.clone()));
        crate::core::bot::agent_vm_hook::init_with(agent_state.clone());
        *api_router = api_router
            .clone()
            .merge(botagent::configure_routes().with_state(agent_state));
    }

    #[cfg(feature = "automations")]
    {
        let automation_state = Arc::new(
            botautomation::AutomationService::new(app_state.conn.clone())
                .with_llm(Arc::new(|_system: &str, _user: &str, _params: &str| {
                    Err("LLM not wired for automations".to_string())
                }))
                .with_delivery(Arc::new(|_channel: &str, _to: &str, _subject: &str, _body: &str| {
                    Ok(())
                })),
        );
        botautomation::scheduler::spawn_scheduler(automation_state.clone());
        *api_router = api_router
            .clone()
            .merge(botautomation::configure_routes().with_state(automation_state));
    }

    #[cfg(feature = "marketplace")]
    {
        let marketplace_state = Arc::new(botmarketplace::MarketplaceService::new(app_state.conn.clone()));
        let seed_state = marketplace_state.clone();
        tokio::spawn(async move {
            match botmarketplace::seed_if_empty(&seed_state).await {
                Ok(count) if count > 0 => log::info!("marketplace seeded {count} starter skills"),
                Ok(_) => {}
                Err(e) => log::error!("marketplace seed failed: {e}"),
            }
        });
        *api_router = api_router
            .clone()
            .merge(botmarketplace::configure_routes().with_state(marketplace_state));
    }

    #[cfg(feature = "consent")]
    {
        crate::core::bot::consent_gate::init(app_state.conn.clone());
        let consent_state = Arc::new(botconsent::ConsentService::new(app_state.conn.clone()));
        consent_state.ensure_sweeper();
        *api_router = api_router
            .clone()
            .merge(botconsent::configure_routes().with_state(consent_state));
    }

    #[cfg(feature = "memory-os")]
    {
        let memory_state = Arc::new(botmemory::MemoryService::new(
            app_state.conn.clone(),
            Arc::new(|_system: &str, _user: &str, _params: &str| {
                Err("LLM not wired for memory extraction".to_string())
            }),
        ));
        crate::core::bot::memory_hook::init_with(memory_state.clone());
        *api_router = api_router
            .clone()
            .merge(botmemory::configure_routes().with_state(memory_state));
    }

    #[cfg(feature = "connectors")]
    { *api_router = api_router.clone().merge(botconnectors::configure()); }

    #[cfg(feature = "browser-policy")]
    {
        let browser_policy_state =
            Arc::new(botbrowserpolicy::BrowserPolicyService::new(app_state.conn.clone()));
        *api_router = api_router
            .clone()
            .merge(botbrowserpolicy::configure_routes().with_state(browser_policy_state));
    }

    #[cfg(feature = "channel-bindings")]
    {
        let bindings_state =
            Arc::new(botchannelbindings::ChannelBindingsService::new(app_state.conn.clone()));
        *api_router = api_router
            .clone()
            .merge(botchannelbindings::configure_routes().with_state(bindings_state));
    }

    *sub_router = sub_router.clone().merge(crate::core::i18n::configure_i18n_routes().with_state(app_state.clone()));
    *sub_router = sub_router.clone().merge(crate::security::configure_protection_routes().with_state(app_state.clone()));
    *sub_router = sub_router.clone().merge(crate::settings::configure_settings_routes().with_state(app_state.clone()));
    *sub_router = sub_router.clone().merge(crate::jukebox::configure_jukebox_routes());
    *sub_router = sub_router.clone().merge(botcore::shared::admin::configure().with_state(app_state.clone()));
    *sub_router = sub_router.clone().merge(botcore::shared::analytics::configure().with_state(app_state.clone()));
    *sub_router = sub_router.clone().merge(botcore::organization_invitations::configure().with_state(app_state.clone()));

    // BotCoder IDE APIs
    #[cfg(feature = "editor")]
    { *sub_router = sub_router.clone().merge(boteditor::configure().with_state(app_state.clone())); }
    #[cfg(feature = "database")]
    { *sub_router = sub_router.clone().merge(botdatabase::configure().with_state(app_state.clone())); }
    #[cfg(feature = "git")]
    { *sub_router = sub_router.clone().merge(botgit::configure().with_state(app_state.clone())); }
    #[cfg(feature = "desktop")]
    {
        *sub_router = sub_router.clone().merge(botdesktop::routes::configure_routes(Some(app_state.conn.clone())));
    }
    *sub_router = sub_router.clone().merge(crate::api::system::configure_system_routes().with_state(app_state.clone()));
    #[cfg(feature = "meet")]
    {
        *sub_router = sub_router.clone().merge(crate::meet::configure().with_state(app_state.clone()));
    }

    #[cfg(feature = "chat")]
    {
        *sub_router = sub_router.clone().merge(super::chat_handlers::configure_chat_routes().with_state(app_state.clone()));
        *sub_router = sub_router.clone().merge(super::chat_history::configure_chat_history_routes().with_state(app_state.clone()));
    }

    *sub_router = sub_router.clone().merge(super::misc_handlers::configure_misc_routes().with_state(app_state.clone()));
    *sub_router = sub_router.clone().merge(super::bot_tree::configure_bot_tree_routes().with_state(app_state.clone()));
    *sub_router = sub_router.clone().merge(super::unified_search::configure_unified_search_routes().with_state(app_state.clone()));
    #[cfg(feature = "tasks")]
    {
        *sub_router = sub_router.clone().merge(crate::tasks::configure_tasks_routes().with_state(Arc::new(bottasks::state::TasksState {
            pool: app_state.conn.clone(),
            run_command: Arc::new(|_cmd: &str, _args: &[&str]| -> Result<String, String> { Ok(String::new()) }),
            call_llm: Arc::new(|_prompt: &str, _ctx: &str| Box::pin(async { Ok(String::new()) })),
            get_config: Arc::new(|_key: &str| -> Result<String, String> { Ok(String::new()) }),
            cache_get: Arc::new(|_key: String| Box::pin(async { Ok(None) })),
            cache_set: Arc::new(|_key: String, _val: String, _ttl: Option<u64>| Box::pin(async { Ok(()) })),
        })));
    }

    #[cfg(feature = "analytics")]
    {
        *sub_router = sub_router.clone().merge(crate::analytics::routes::create_analytics_router(Arc::new(app_state.conn.clone())));
        *sub_router = sub_router.clone().merge(crate::analytics::insights::configure_insights_routes().with_state(Arc::new(app_state.conn.clone())));
    }

    #[cfg(feature = "docs")]
    {
        *sub_router = sub_router.clone().merge(crate::docs::configure_docs_routes().with_state(Arc::new(botdocs::state::DocState {
            pool: Arc::new(app_state.conn.clone()),
            drive: app_state.drive.clone().unwrap_or_else(|| Arc::new(crate::drive::NoopDrive)),
            bucket_name: app_state.bucket_name.clone(),
            history: tokio::sync::Mutex::new(std::collections::HashMap::new()),
        })));
    }

    #[cfg(feature = "paper")]
    {
        *sub_router = sub_router.clone().merge(crate::paper::configure_paper_routes().with_state(super::feature_routers::make_paper_state(app_state)));
    }

    #[cfg(feature = "research")]
    {
        #[derive(Debug, Clone)]
        struct ResearchAppState {
            pool: diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>,
            llm: Option<Arc<dyn botlib::traits::LLMProvider>>,
        }
        impl crate::research::ResearchState for ResearchAppState {
            fn db_pool(&self) -> &diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>> {
                &self.pool
            }
            fn llm_provider(&self) -> Option<Arc<dyn botlib::traits::LLMProvider>> {
                self.llm.clone()
            }
            fn bot_id(&self) -> Option<uuid::Uuid> {
                self.resolve_bot_id()
            }
        }

        impl ResearchAppState {
            fn resolve_bot_id(&self) -> Option<uuid::Uuid> {
                use diesel::prelude::*;
                let mut conn = match self.pool.get() {
                    Ok(c) => c,
                    Err(e) => {
                        log::error!("Research bot_id DB connection error: {e}");
                        return None;
                    }
                };
                #[derive(diesel::QueryableByName)]
                #[diesel(check_for_backend(diesel::pg::Pg))]
                struct BotIdRow {
                    #[diesel(sql_type = diesel::sql_types::Uuid)]
                    id: uuid::Uuid,
                }
                diesel::sql_query(
                    "SELECT id FROM bots WHERE is_default_for_branch = TRUE ORDER BY created_at ASC LIMIT 1",
                )
                .get_result::<BotIdRow>(&mut conn)
                .ok()
                .map(|r| r.id)
            }
        }

        let research_state = Arc::new(ResearchAppState {
            pool: app_state.conn.clone(),
            llm: app_state.llm_provider.clone(),
        });
        *sub_router = sub_router.clone().merge(crate::research::configure_research_routes().with_state(research_state.clone()));
        *sub_router = sub_router.clone().merge(crate::research::ui::configure_research_ui_routes().with_state(research_state));
    }

    #[cfg(feature = "search")]
    {
        let search_state = Arc::new(crate::search::SearchService::new(Arc::new(app_state.conn.clone()), None));
        *sub_router = sub_router.clone().merge(crate::search::configure_search_routes().with_state(search_state));
    }

    #[cfg(any(feature = "research", feature = "llm"))]
    {
        *api_router = api_router.clone().route(
            "/api/website/force-recrawl",
            post(crate::core::kb::website_crawler_service::handle_force_recrawl)
        );
    }

    #[cfg(feature = "designer")]
    {
        *sub_router = sub_router.clone().merge(super::feature_routers::make_designer_router(app_state));
    }

    #[cfg(feature = "dashboards")]
    {
        *sub_router = sub_router.clone().merge(super::feature_routers::make_dashboards_router(app_state));
    }
}
