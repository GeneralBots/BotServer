use super::*;

pub async fn handle_dns_search(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    use diesel::prelude::*;

    let q = params.get("q").cloned().unwrap_or_default();
    let mut conn = match pool_conn(&state) {
        Ok(c) => c,
        Err(e) => return Html(format!("<tr><td colspan=\"7\">{}</td></tr>", html_escape(&e.1))),
    };

    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct DnsRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)]
        hostname: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        record_type: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        target: String,
        #[diesel(sql_type = diesel::sql_types::Int4)]
        ttl: i32,
        #[diesel(sql_type = diesel::sql_types::Text)]
        status: String,
    }

    let pattern = format!("%{q}%");
    let rows: Vec<DnsRow> = if q.is_empty() {
        diesel::sql_query("SELECT id, hostname, record_type, target, ttl, status FROM dns_records ORDER BY hostname ASC LIMIT 200")
            .load(&mut conn)
            .unwrap_or_default()
    } else {
        diesel::sql_query(
            "SELECT id, hostname, record_type, target, ttl, status FROM dns_records WHERE LOWER(hostname) LIKE LOWER($1) ORDER BY hostname ASC LIMIT 200",
        )
        .bind::<diesel::sql_types::Text, _>(&pattern)
        .load(&mut conn)
        .unwrap_or_default()
    };

    if rows.is_empty() {
        return Html("<tr><td colspan=\"7\" class=\"empty-cell\">No DNS records match</td></tr>".to_string());
    }

    let mut html = String::new();
    for r in &rows {
        html.push_str(&format!(
            r#"<tr>
    <td>{hostname}</td><td>{rtype}</td><td>{target}</td><td>{ttl}</td>
    <td><span class="status-badge {status}">{status}</span></td>
    <td>-</td>
    <td>
        <button class="btn-icon" onclick="openEditModal('{id}')">✏️</button>
        <button class="btn-icon" onclick="openRemoveModal('{id}')">🗑️</button>
    </td>
</tr>"#,
            hostname = html_escape(&r.hostname),
            rtype = html_escape(&r.record_type),
            target = html_escape(&r.target),
            ttl = r.ttl,
            status = html_escape(&r.status),
            id = r.id,
        ));
    }
    Html(html)
}
