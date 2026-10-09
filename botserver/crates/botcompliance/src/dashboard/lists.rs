use super::*;

pub(super) async fn handle_failing_controls(State(pool): State<Arc<DbPool>>) -> impl IntoResponse {
    let branch = branch_id_for(&pool);
    let conn = pool.clone();

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
            check_type: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            status: String,
        }
        diesel::sql_query(
            "SELECT id, check_type, status FROM compliance_checks WHERE branch_id = $1 AND (status = 'fail' OR status IS NULL) ORDER BY updated_at DESC LIMIT 25",
        )
        .bind::<diesel::sql_types::Uuid, _>(branch)
        .load::<Row>(&mut db_conn)
        .unwrap_or_default()
    })
    .await
    .unwrap_or_default();

    let mut html = String::from("<div class=\"controls-list\">");
    if rows.is_empty() {
        html.push_str("<div class=\"empty-state\"><p>All controls are passing</p></div>");
    }
    for r in &rows {
        html.push_str(&format!(
            "<div class=\"control-item\" data-control-id=\"{id}\"><span class=\"control-name\">{name}</span><span class=\"badge badge-danger\">{status}</span></div>",
            id = r.id,
            name = html_escape(&r.check_type),
            status = html_escape(if r.status.is_empty() { "pending" } else { &r.status }),
        ));
    }
    html.push_str("</div>");
    Html(html)
}

pub(super) async fn handle_evidence(State(pool): State<Arc<DbPool>>) -> impl IntoResponse {
    let branch = branch_id_for(&pool);
    let conn = pool.clone();

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
            file_path: String,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
            description: Option<String>,
        }
        diesel::sql_query(
            "SELECT id, file_path, description FROM compliance_evidence WHERE branch_id = $1 ORDER BY created_at DESC LIMIT 25",
        )
        .bind::<diesel::sql_types::Uuid, _>(branch)
        .load::<Row>(&mut db_conn)
        .unwrap_or_default()
    })
    .await
    .unwrap_or_default();

    let mut html = String::from("<div class=\"evidence-list\">");
    if rows.is_empty() {
        html.push_str("<div class=\"empty-state\"><p>No evidence items yet</p></div>");
    }
    for r in &rows {
        html.push_str(&format!(
            "<div class=\"evidence-item\" data-evidence-id=\"{id}\"><span class=\"evidence-file\">{file}</span><span class=\"evidence-desc\">{desc}</span></div>",
            id = r.id,
            file = html_escape(&r.file_path),
            desc = html_escape(r.description.as_deref().unwrap_or("")),
        ));
    }
    html.push_str("</div>");
    Html(html)
}

pub(super) async fn handle_audit_log(State(pool): State<Arc<DbPool>>) -> impl IntoResponse {
    let branch = branch_id_for(&pool);
    let conn = pool.clone();

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
            action: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            resource_type: String,
            #[diesel(sql_type = diesel::sql_types::Timestamptz)]
            created_at: chrono::DateTime<chrono::Utc>,
        }
        diesel::sql_query(
            "SELECT id, action, resource_type, created_at FROM compliance_audit_log WHERE branch_id = $1 ORDER BY created_at DESC LIMIT 25",
        )
        .bind::<diesel::sql_types::Uuid, _>(branch)
        .load::<Row>(&mut db_conn)
        .unwrap_or_default()
    })
    .await
    .unwrap_or_default();

    let mut html = String::from("<div class=\"log-list\">");
    if rows.is_empty() {
        html.push_str("<div class=\"empty-state\"><p>No audit events yet</p></div>");
    }
    for r in &rows {
        html.push_str(&format!(
            "<div class=\"log-item\" data-log-id=\"{id}\"><span class=\"log-action\">{action}</span><span class=\"log-target\">{target}</span><span class=\"log-time\">{time}</span></div>",
            id = r.id,
            action = html_escape(&r.action),
            target = html_escape(&r.resource_type),
            time = r.created_at.format("%Y-%m-%d %H:%M"),
        ));
    }
    html.push_str("</div>");
    Html(html)
}
