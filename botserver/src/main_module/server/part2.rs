use super::*;

/// The tower layers the server applies to every route.
pub(super) struct ServerLayers {
    pub cors: tower_http::cors::CorsLayer,
    pub csrf_manager: Arc<CsrfManager>,
    pub panic_config: PanicHandlerConfig,
    pub security_headers: axum::Extension<SecurityHeadersConfig>,
    pub rate_limit: axum::Extension<Arc<CombinedRateLimiter>>,
    pub rbac_manager: Arc<crate::security::rbac_middleware::RbacManager>,
    pub auth_middleware_state: crate::security::auth::AuthMiddlewareState,
}

pub(super) fn apply_middleware(app: Router, app_state: Arc<AppState>, layers: ServerLayers) -> Router {
    let ServerLayers {
        cors,
        csrf_manager,
        panic_config,
        security_headers: security_headers_extension,
        rate_limit: rate_limit_extension,
        rbac_manager,
        auth_middleware_state,
    } = layers;
    async fn csrf_cookie_injector(
        manager: Arc<CsrfManager>,
        request: axum::http::Request<axum::body::Body>,
        next: axum::middleware::Next,
    ) -> axum::response::Response {
        let mut response = next.run(request).await;
        let has_csrf = response
            .headers()
            .get_all(axum::http::header::SET_COOKIE)
            .iter()
            .any(|v| v.to_str().ok().is_some_and(|s| s.contains("csrf_token")));
        if !has_csrf {
            let token = manager.generate_signed_token();
            let cookie = manager.build_cookie(&token);
            if let Ok(cv) = cookie.parse::<axum::http::HeaderValue>() {
                response
                    .headers_mut()
                    .append(axum::http::header::SET_COOKIE, cv);
            }
        }
        response
    }

    app
        .layer(axum::middleware::from_fn({
            let name = app_state.config.as_ref()
                .map(|c| format!("botserver:{}", c.server.host))
                .unwrap_or_else(|| "botserver".to_string());
            move |req, next| {
                let name = name.clone();
                async move { botcore::tracing::tracing_middleware_fn(name, req, next).await }
            }
        }))
        .layer(axum::middleware::from_fn({
            let csrf_manager = csrf_manager.clone();
            move |req, next| {
                let mgr = csrf_manager.clone();
                async move { csrf_cookie_injector(mgr, req, next).await }
            }
        }))
        .layer(axum::middleware::from_fn(security_headers_middleware))
        .layer(security_headers_extension)
        .layer(rate_limit_extension)
        .layer(axum::middleware::from_fn(request_id_middleware))
        .layer(axum::middleware::from_fn({
            let csrf_manager = csrf_manager.clone();
            move |req, next| {
                let mgr = csrf_manager.clone();
                async move { csrf_middleware(mgr, req, next).await }
            }
        }))
        .layer(axum::middleware::from_fn(
            move |req: axum::http::Request<axum::body::Body>, next: axum::middleware::Next| {
                let rbac = rbac_manager.clone();
                async move { crate::security::rbac_middleware::rbac_middleware_fn(req, next, rbac).await }
            },
        ))
        // Resolves DB-backed roles (rbac_user_groups) for cloud-JWT/Zitadel
        // authenticated users, upgrading admins BEFORE the RBAC check runs.
        // Runs after auth (outer) and before rbac (inner) by layer order.
        .layer(axum::middleware::from_fn(
            crate::security::rbac_role_resolver_middleware,
        ))
        .layer(axum::middleware::from_fn(
            move |req: axum::http::Request<axum::body::Body>, next: axum::middleware::Next| {
                let state = auth_middleware_state.clone();
                async move {
                    crate::security::auth::auth_middleware_with_providers(req, next, state).await
                }
            },
        ))
        .layer(axum::middleware::from_fn(move |req, next| {
            let config = panic_config.clone();
            async move {
                crate::security::panic_handler::panic_handler_middleware_with_config(req, next, &config).await
            }
        }))
        .layer(axum::Extension(app_state))
        .layer(cors)
        .layer(axum::middleware::from_fn(crate::security::cors::strip_proxy_cors_middleware))
        .layer(TraceLayer::new_for_http())
}
#[cfg(feature = "calendar")]
pub(super) fn add_calendar_routes(r: Router, s: &Arc<AppState>) -> Router {
    r.merge(crate::calendar::configure_calendar_routes().with_state(Arc::new(s.conn.clone())))
        .merge(crate::calendar::configure_calendar_ui_routes().with_state(Arc::new(s.conn.clone())))
        .merge(crate::calendar::create_caldav_router().with_state(Arc::new(s.conn.clone())))
}
#[cfg(feature = "mail")]
pub(super) fn add_mail_routes(r: Router, s: &Arc<AppState>) -> Router {
    let email_state = crate::email::models::AppState {
        pool: Arc::new(s.conn.clone()),
        get_default_bot: Arc::new(|conn: &mut diesel::PgConnection| {
            // Resolve the real workspace branch instead of the previous
            // hardcoded nil (issue #734); falls back to nil global scope only
            // when no bot is flagged default yet.
            use diesel::prelude::*;
            #[derive(diesel::QueryableByName)]
            #[diesel(check_for_backend(diesel::pg::Pg))]
            struct BranchRow {
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                branch_id: uuid::Uuid,
            }
            diesel::sql_query(
                "SELECT branch_id FROM bots WHERE is_default_for_branch = TRUE LIMIT 1",
            )
            .get_result::<BranchRow>(conn)
            .map(|r| (r.branch_id, "default".to_string()))
            .unwrap_or_else(|_| (uuid::Uuid::nil(), "default".to_string()))
        }),
        secrets_provider: Arc::new(|_key: &str| Err("secrets not available".to_string())),
    };
    crate::email::poller::spawn_imap_sync_worker(s.conn.clone());
    r.merge(crate::email::routes::configure(Arc::new(email_state)))
}
#[cfg(feature = "slides")]
pub(super) fn add_slides_routes(r: Router, s: &Arc<AppState>) -> Router {
    r.merge(crate::slides::configure_slides_routes(s.clone()))
}
#[cfg(feature = "plan")]
pub(super) fn add_plan_routes(r: Router, s: &Arc<AppState>) -> Router {
    crate::plan::set_pool(s.conn.clone());
    crate::plan::load_from_db();
    r.merge(crate::plan::configure_plan_routes())
}
#[cfg(feature = "video")]
pub(super) fn add_video_routes(r: Router, s: &Arc<AppState>) -> Router {
    r.merge(crate::video::configure_video_routes().with_state(Arc::new(botvideo::routes::AppState { conn: s.conn.clone(), cache: None })))
        .merge(crate::video::configure_video_ui_routes().with_state(Arc::new(botvideo::routes::AppState { conn: s.conn.clone(), cache: None })))
}
#[cfg(feature = "workspaces")]
pub(super) fn add_workspaces_routes(r: Router, s: &Arc<AppState>) -> Router {
    let make_state = || Arc::new(botworkspaces::WorkspacesState {
        pool: Arc::new(s.conn.clone()),
        get_default_bot: (|_conn: &mut diesel::PgConnection| uuid::Uuid::nil()) as fn(&mut diesel::PgConnection) -> uuid::Uuid,
    });
    r.merge(crate::workspaces::configure_workspaces_routes().with_state(make_state()))
        .merge(crate::workspaces::configure_workspaces_ui_routes().with_state(make_state()))
}
#[cfg(feature = "billing")]
pub(super) fn add_products_routes(r: Router, s: &Arc<AppState>) -> Router {
    let make_state = || Arc::new(botproducts::ProductsState {
        pool: Arc::new(s.conn.clone()),
        get_default_bot: Some((|_conn: &mut diesel::PgConnection| uuid::Uuid::nil()) as fn(&mut diesel::PgConnection) -> uuid::Uuid),
    });
    r.merge(crate::products::configure_products_routes().with_state(make_state()))
        .merge(crate::products::configure_products_api_routes().with_state(make_state()))
}
#[cfg(feature = "tickets")]
pub(super) fn add_tickets_routes(r: Router, s: &Arc<AppState>) -> Router {
    let make_state = || Arc::new(bottickets::TicketsState {
        pool: Arc::new(s.conn.clone()),
        get_default_bot: (|_conn: &mut diesel::PgConnection| uuid::Uuid::nil()) as fn(&mut diesel::PgConnection) -> uuid::Uuid,
    });
    r.merge(crate::tickets::configure_tickets_routes().with_state(make_state()))
        .merge(crate::tickets::ui::configure_tickets_ui_routes().with_state(make_state()))
        .merge(crate::tickets::itsm::configure_itsm_routes().with_state(make_state()))
        .merge(crate::tickets::itsm::configure_itsm_ui_routes().with_state(make_state()))
}
#[cfg(feature = "people")]
pub(super) fn add_people_routes(r: Router, s: &Arc<AppState>) -> Router {
    let people_state = Arc::new(crate::people::PeopleState {
        pool: Arc::new(s.conn.clone()),
        get_default_bot: Arc::new(|_conn: &mut diesel::PgConnection| uuid::Uuid::nil()),
    });
    r.merge(crate::people::configure_people_routes().with_state(people_state.clone()))
        .merge(crate::people::ui::configure_people_ui_routes().with_state(people_state))
}
#[cfg(feature = "attendant")]
pub(super) fn add_attendant_routes(r: Router, s: &Arc<AppState>) -> Router {
    r.merge(crate::attendant::configure_attendant_routes().with_state(Arc::new(botattendant::AttendantConfig {
        pool: Arc::new(s.conn.clone()),
        get_default_bot: (|_conn: &mut diesel::PgConnection| uuid::Uuid::nil()) as fn(&mut diesel::PgConnection) -> uuid::Uuid,
    })))
    .merge(crate::attendant::configure_attendant_ui_routes().with_state(Arc::new(botattendant::AttendantConfig {
        pool: Arc::new(s.conn.clone()),
        get_default_bot: (|_conn: &mut diesel::PgConnection| uuid::Uuid::nil()) as fn(&mut diesel::PgConnection) -> uuid::Uuid,
    })))
}
pub(crate) async fn listen(app: Router, port: u16) -> std::io::Result<()> {
    crate::main_module::listen_impl::listen(app, port).await
}
