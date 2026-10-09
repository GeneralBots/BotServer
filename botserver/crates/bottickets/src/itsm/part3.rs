use super::*;

pub(super) async fn render_kb_list(state: &Arc<TicketsState>) -> String {
    let branch_id = resolve_branch_id(state);
    let conn = state.pool.clone();

    let rows = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {e}");
                return Vec::new();
            }
        };
        #[derive(diesel::QueryableByName)]
        #[diesel(check_for_backend(diesel::pg::Pg))]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            id: Uuid,
            #[diesel(sql_type = diesel::sql_types::Text)]
            title: String,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
            category: Option<String>,
        }
        diesel::sql_query(
            "SELECT id, title, category FROM ticket_kb_articles WHERE branch_id = $1 AND is_published = TRUE ORDER BY title ASC LIMIT 100",
        )
        .bind::<diesel::sql_types::Uuid, _>(branch_id)
        .load::<Row>(&mut db_conn)
        .unwrap_or_default()
    })
    .await
    .unwrap_or_default();

    if rows.is_empty() {
        return "<div class=\"empty-state\"><p>No KB articles yet</p></div>".to_string();
    }

    let mut html = String::from("<div class=\"kb-articles-list\">");
    for r in &rows {
        html.push_str(&format!(
            "<div class=\"kb-article-item\" data-article-id=\"{id}\"><h4>{title}</h4><span class=\"badge\">{category}</span></div>",
            id = r.id,
            title = html_escape(&r.title),
            category = html_escape(r.category.as_deref().unwrap_or("General")),
        ));
    }
    html.push_str("</div>");
    html
}
pub fn configure_itsm_ui_routes() -> Router<Arc<TicketsState>> {
    Router::new()
        .route("/api/ui/tickets/cis", get(handle_ci_fragment))
        .route("/api/ui/tickets/kb", get(handle_kb_fragment))
}
pub(super) async fn handle_ci_fragment(State(state): State<Arc<TicketsState>>) -> impl IntoResponse {
    Html(render_ci_list(&state).await)
}
pub(super) async fn handle_kb_fragment(State(state): State<Arc<TicketsState>>) -> impl IntoResponse {
    Html(render_kb_list(&state).await)
}
