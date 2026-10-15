use super::*;

pub fn configure_routes(state: Arc<WebaState>) -> Router {
    Router::new()
        .route("/apps", get(list_apps).post(create_app))
        .route("/apps/:id", get(get_app).put(update_app).delete(delete_app))
        .route("/apps/:id/pages", get(list_pages).post(create_page))
        .route(
            "/apps/:id/pages/:page_id",
            get(get_page).put(update_page).delete(delete_page),
        )
        .route("/apps/:id/publish", post(publish_app))
        .route("/apps/:id/preview", get(preview_app))
        .route("/render/:slug", get(render_app))
        .route("/render/:slug/*path", get(render_page))
        .with_state(state)
}
pub(super) async fn list_apps(
    State(state): State<Arc<WebaState>>,
    Query(query): Query<ListQuery>,
) -> Json<Vec<WebApp>> {
    let apps = state.apps.read().await;
    let mut result: Vec<WebApp> = apps.values().cloned().collect();

    if let Some(status) = query.status {
        result.retain(|app| {
            matches!(
                (&app.status, status.as_str()),
                (WebAppStatus::Draft, "draft")
                    | (WebAppStatus::Published, "published")
                    | (WebAppStatus::Archived, "archived")
            )
        });
    }

    result.sort_by_key(|a| std::cmp::Reverse(a.updated_at));

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(50);
    let result: Vec<WebApp> = result.into_iter().skip(offset).take(limit).collect();

    Json(result)
}
pub(super) async fn create_app(
    State(state): State<Arc<WebaState>>,
    Json(req): Json<CreateAppRequest>,
) -> Json<WebApp> {
    let now = chrono::Utc::now();
    let id = Uuid::new_v4();
    let slug = slugify(&req.name);

    let app = WebApp {
        id,
        name: req.name,
        slug,
        description: req.description,
        template: req.template.unwrap_or_default(),
        status: WebAppStatus::Draft,
        config: WebAppConfig::default(),
        created_at: now,
        updated_at: now,
    };

    let mut apps = state.apps.write().await;
    apps.insert(id, app.clone());

    Json(app)
}
pub(super) async fn get_app(
    State(state): State<Arc<WebaState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<WebApp>, axum::http::StatusCode> {
    let apps = state.apps.read().await;
    apps.get(&id)
        .cloned()
        .map(Json)
        .ok_or(axum::http::StatusCode::NOT_FOUND)
}
pub(super) async fn update_app(
    State(state): State<Arc<WebaState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateAppRequest>,
) -> Result<Json<WebApp>, axum::http::StatusCode> {
    let mut apps = state.apps.write().await;

    let app = apps.get_mut(&id).ok_or(axum::http::StatusCode::NOT_FOUND)?;

    if let Some(name) = req.name {
        app.name.clone_from(&name);
        app.slug = slugify(&name);
    }
    if let Some(description) = req.description {
        app.description = Some(description);
    }
    if let Some(status) = req.status {
        app.status = status;
    }
    if let Some(config) = req.config {
        app.config = config;
    }
    app.updated_at = chrono::Utc::now();

    Ok(Json(app.clone()))
}
pub(super) async fn delete_app(
    State(state): State<Arc<WebaState>>,
    Path(id): Path<Uuid>,
) -> axum::http::StatusCode {
    let mut apps = state.apps.write().await;
    let mut pages = state.pages.write().await;

    pages.retain(|_, page| page.app_id != id);

    if apps.remove(&id).is_some() {
        axum::http::StatusCode::NO_CONTENT
    } else {
        axum::http::StatusCode::NOT_FOUND
    }
}
pub(super) async fn list_pages(
    State(state): State<Arc<WebaState>>,
    Path(app_id): Path<Uuid>,
) -> Json<Vec<WebAppPage>> {
    let pages = state.pages.read().await;
    let result: Vec<WebAppPage> = pages
        .values()
        .filter(|p| p.app_id == app_id)
        .cloned()
        .collect();
    Json(result)
}
pub(super) async fn create_page(
    State(state): State<Arc<WebaState>>,
    Path(app_id): Path<Uuid>,
    Json(req): Json<CreatePageRequest>,
) -> Result<Json<WebAppPage>, axum::http::StatusCode> {
    let apps = state.apps.read().await;
    if !apps.contains_key(&app_id) {
        return Err(axum::http::StatusCode::NOT_FOUND);
    }
    drop(apps);

    let now = chrono::Utc::now();
    let id = Uuid::new_v4();

    let page = WebAppPage {
        id,
        app_id,
        path: req.path,
        title: req.title,
        content: req.content,
        layout: req.layout,
        is_index: req.is_index,
        created_at: now,
        updated_at: now,
    };

    let mut pages = state.pages.write().await;
    pages.insert(id, page.clone());

    Ok(Json(page))
}
pub(super) async fn get_page(
    State(state): State<Arc<WebaState>>,
    Path((app_id, page_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<WebAppPage>, axum::http::StatusCode> {
    let pages = state.pages.read().await;
    pages
        .get(&page_id)
        .filter(|p| p.app_id == app_id)
        .cloned()
        .map(Json)
        .ok_or(axum::http::StatusCode::NOT_FOUND)
}
pub(super) async fn update_page(
    State(state): State<Arc<WebaState>>,
    Path((app_id, page_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<CreatePageRequest>,
) -> Result<Json<WebAppPage>, axum::http::StatusCode> {
    let mut pages = state.pages.write().await;

    let page = pages
        .get_mut(&page_id)
        .filter(|p| p.app_id == app_id)
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    page.path = req.path;
    page.title = req.title;
    page.content = req.content;
    page.layout = req.layout;
    page.is_index = req.is_index;
    page.updated_at = chrono::Utc::now();

    Ok(Json(page.clone()))
}
pub(super) async fn delete_page(
    State(state): State<Arc<WebaState>>,
    Path((app_id, page_id)): Path<(Uuid, Uuid)>,
) -> axum::http::StatusCode {
    let mut pages = state.pages.write().await;

    let exists = pages
        .get(&page_id)
        .map(|p| p.app_id == app_id)
        .unwrap_or(false);

    if exists {
        pages.remove(&page_id);
        axum::http::StatusCode::NO_CONTENT
    } else {
        axum::http::StatusCode::NOT_FOUND
    }
}
pub(super) async fn publish_app(
    State(state): State<Arc<WebaState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<WebApp>, axum::http::StatusCode> {
    let mut apps = state.apps.write().await;
    let app = apps.get_mut(&id).ok_or(axum::http::StatusCode::NOT_FOUND)?;

    app.status = WebAppStatus::Published;
    app.updated_at = chrono::Utc::now();

    Ok(Json(app.clone()))
}
pub(super) async fn preview_app(
    State(state): State<Arc<WebaState>>,
    Path(id): Path<Uuid>,
) -> Result<Html<String>, axum::http::StatusCode> {
    let apps = state.apps.read().await;
    let app = apps.get(&id).ok_or(axum::http::StatusCode::NOT_FOUND)?;

    let pages = state.pages.read().await;
    let index_page = pages.values().find(|p| p.app_id == id && p.is_index);

    let content = index_page
        .map(|p| p.content.clone())
        .unwrap_or_else(|| "<p>No content yet</p>".to_string());

    let html = render_html(app, &content);
    Ok(Html(html))
}
pub(super) async fn render_app(
    State(state): State<Arc<WebaState>>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, axum::http::StatusCode> {
    let apps = state.apps.read().await;
    let app = apps
        .values()
        .find(|a| a.slug == slug && matches!(a.status, WebAppStatus::Published))
        .ok_or(axum::http::StatusCode::NOT_FOUND)?
        .clone();
    drop(apps);

    let pages = state.pages.read().await;
    let index_page = pages.values().find(|p| p.app_id == app.id && p.is_index);

    let content = index_page
        .map(|p| p.content.clone())
        .unwrap_or_else(|| "<p>Page not found</p>".to_string());

    let html = render_html(&app, &content);
    Ok(Html(html))
}
pub(super) async fn render_page(
    State(state): State<Arc<WebaState>>,
    Path((slug, path)): Path<(String, String)>,
) -> Result<impl IntoResponse, axum::http::StatusCode> {
    let apps = state.apps.read().await;
    let app = apps
        .values()
        .find(|a| a.slug == slug && matches!(a.status, WebAppStatus::Published))
        .ok_or(axum::http::StatusCode::NOT_FOUND)?
        .clone();
    drop(apps);

    let normalized_path = format!("/{}", path.trim_start_matches('/'));

    let pages = state.pages.read().await;
    let page = pages
        .values()
        .find(|p| p.app_id == app.id && p.path == normalized_path);

    let content = page
        .map(|p| p.content.clone())
        .unwrap_or_else(|| "<p>Page not found</p>".to_string());

    let html = render_html(&app, &content);
    Ok(Html(html))
}
