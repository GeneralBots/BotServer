use super::*;

pub async fn handle_dns_register(
    State(state): State<Arc<AppState>>,
    Form(form): Form<DnsRecordRequest>,
) -> impl IntoResponse {
    use diesel::prelude::*;

    let hostname = form.hostname.unwrap_or_default().trim().to_string();
    if hostname.is_empty() {
        return Html("<div class=\"dns-error\">Hostname is required</div>".to_string());
    }
    let record_type = form.record_type.unwrap_or_else(|| "A".to_string());
    let target = form.target.unwrap_or_default();
    let ttl = form.ttl.and_then(|t| t.parse::<i32>().ok()).unwrap_or(300);
    let id = Uuid::new_v4();

    let mut conn = match pool_conn(&state) {
        Ok(c) => c,
        Err(e) => return Html(format!("<div class=\"dns-error\">{}</div>", html_escape(&e.1))),
    };

    let _ = diesel::sql_query(
        "INSERT INTO dns_records (id, hostname, record_type, target, ttl, status) VALUES ($1, $2, $3, $4, $5, 'active') ON CONFLICT DO NOTHING",
    )
    .bind::<diesel::sql_types::Uuid, _>(&id)
    .bind::<diesel::sql_types::Text, _>(&hostname)
    .bind::<diesel::sql_types::Text, _>(&record_type)
    .bind::<diesel::sql_types::Text, _>(&target)
    .bind::<diesel::sql_types::Int4, _>(ttl)
    .execute(&mut conn);

    Html(format!(
        r#"<div class="dns-success">DNS record <strong>{hostname}</strong> registered</div>"#,
        hostname = html_escape(&hostname),
    ))
}
pub async fn handle_dns_edit_form(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> impl IntoResponse {
    use diesel::prelude::*;

    let mut conn = match pool_conn(&state) {
        Ok(c) => c,
        Err(e) => return Html(format!("<p>{}</p>", html_escape(&e.1))),
    };

    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct DnsRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        hostname: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        record_type: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        target: String,
        #[diesel(sql_type = diesel::sql_types::Int4)]
        ttl: i32,
    }

    let row: Option<DnsRow> = diesel::sql_query(
        "SELECT hostname, record_type, target, ttl FROM dns_records WHERE id = $1",
    )
    .bind::<diesel::sql_types::Uuid, _>(id)
    .get_result(&mut conn)
    .ok();

    match row {
        Some(r) => Html(format!(
            r##"<form hx-post="/api/dns/register" hx-target="#edit-dns-form-container" hx-swap="innerHTML">
            <div class="form-group"><label>Hostname</label><input type="text" name="hostname" value="{hostname}" required /></div>
            <div class="form-group"><label>Type</label><input type="text" name="record_type" value="{rtype}" /></div>
            <div class="form-group"><label>Target/IP</label><input type="text" name="target" value="{target}" /></div>
            <div class="form-group"><label>TTL</label><input type="number" name="ttl" value="{ttl}" /></div>
            <button type="submit" class="btn-primary btn-sm">Save</button>
        </form>"##,
            hostname = html_escape(&r.hostname),
            rtype = html_escape(&r.record_type),
            target = html_escape(&r.target),
            ttl = r.ttl,
        )),
        None => Html("<p class=\"text-muted\">Record not found</p>".to_string()),
    }
}
