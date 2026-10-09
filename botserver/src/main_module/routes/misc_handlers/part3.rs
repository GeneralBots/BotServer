use super::*;

pub async fn handle_autotask_create(
    State(state): State<Arc<AppState>>,
    Form(payload): Form<AutotaskRequest>,
) -> impl IntoResponse {
    let intent = payload.intent.unwrap_or_default().trim().to_string();
    if intent.is_empty() {
        return Json(serde_json::json!({ "ok": false, "error": "intent is required" }));
    }

    let response = if let Some(llm) = state.llm_provider.clone() {
        match llm
            .generate_simple(&format!(
                "You are an automation planner. Break this user request into concrete steps using \
                 available tools (send email, create task, create report, query data). Return a short \
                 plan.\n\nRequest: {intent}"
            ))
            .await
        {
            Ok(plan) => plan,
            Err(e) => {
                log::error!("Autotask LLM error: {e}");
                format!("Could not plan '{intent}'.")
            }
        }
    } else {
        format!("Planned steps for: {intent}")
    };

    Json(serde_json::json!({
        "ok": true,
        "intent": intent,
        "plan": response,
    }))
}

// ---------------------------------------------------------------------------
// Services summary: /api/services/summary
// ---------------------------------------------------------------------------
pub async fn handle_services_summary(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    use diesel::prelude::*;

    let mut conn = pool_conn(&state)?;

    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct ServiceRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        status: String,
    }

    let rows: Vec<ServiceRow> = diesel::sql_query(
        "SELECT 'api' AS name, 'online' AS status",
    )
    .load(&mut conn)
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    let mut services = Vec::new();
    for r in &rows {
        services.push(serde_json::json!({ "name": r.name, "status": r.status }));
    }

    Ok(Json(serde_json::json!({ "services": services, "total": services.len() })))
}

// ---------------------------------------------------------------------------
// Products pricelists: /api/products/pricelists
// ---------------------------------------------------------------------------
pub async fn handle_products_pricelists(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    use diesel::prelude::*;

    let mut conn = pool_conn(&state)?;

    // Issue #738: pricelists are never anonymous. Anonymous callers are
    // already rejected by the auth middleware (this path is not public);
    // authenticated callers without a tenant binding resolve to the global
    // (nil) branch scope, mirroring campaigns and the seeded global catalog.
    let branch_id = botcontacts::scope::branch_from_jwt_pool(&headers, &state.conn)
        .unwrap_or(Uuid::nil());

    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct PriceRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
        price: Option<bigdecimal::BigDecimal>,
        #[diesel(sql_type = diesel::sql_types::Text)]
        currency: String,
    }

    let rows: Vec<PriceRow> = diesel::sql_query(
        "SELECT id, name, price, currency FROM products \
         WHERE branch_id = $1 AND is_public = true \
         ORDER BY name ASC LIMIT 100",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch_id)
    .load(&mut conn)
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    let pricelists: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "name": r.name,
                "price": r.price.map(|p| p.to_string()).unwrap_or_else(|| "0".to_string()),
                "currency": r.currency,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "pricelists": pricelists })))
}

// ---------------------------------------------------------------------------
// Contacts search: /api/contacts/search
// ---------------------------------------------------------------------------
pub async fn handle_contacts_search(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    use diesel::prelude::*;

    let q = params.get("q").cloned().unwrap_or_default();
    let mut conn = pool_conn(&state)?;

    // Tenant scoping (issue #738): the search only ever returns contacts of
    // the authenticated user's workspace branch. The branch is derived from
    // the server-minted JWT claim, the session cache, or the crm_contacts
    // owner row — never from client input. Unauthenticated requests get no
    // rows (the middleware rejects them before this handler anyway).
    let branch_id = botcontacts::scope::branch_from_jwt_pool(&headers, &state.conn)
        .unwrap_or(Uuid::nil());

    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct ContactRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        first_name: Option<String>,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        last_name: Option<String>,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        email: Option<String>,
    }

    pub(super) fn display_name(row: &ContactRow) -> String {
        let full = [row.first_name.as_deref(), row.last_name.as_deref()]
            .into_iter()
            .flatten()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if full.is_empty() {
            row.email.clone().unwrap_or_else(|| "Unknown".to_string())
        } else {
            full
        }
    }

    let pattern = format!("%{q}%");
    let rows: Vec<ContactRow> = if q.is_empty() {
        diesel::sql_query(
            "SELECT id, first_name, last_name, email FROM crm_contacts \
             WHERE branch_id = $1 ORDER BY first_name ASC, last_name ASC LIMIT 20",
        )
        .bind::<diesel::sql_types::Uuid, _>(branch_id)
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?
    } else {
        diesel::sql_query(
            "SELECT id, first_name, last_name, email FROM crm_contacts \
             WHERE branch_id = $1 \
               AND (LOWER(first_name) LIKE LOWER($2) \
                    OR LOWER(last_name) LIKE LOWER($2) \
                    OR LOWER(email) LIKE LOWER($2)) \
             ORDER BY first_name ASC, last_name ASC LIMIT 20",
        )
        .bind::<diesel::sql_types::Uuid, _>(branch_id)
        .bind::<diesel::sql_types::Text, _>(&pattern)
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?
    };

    let contacts: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "name": display_name(r),
                "email": r.email,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "contacts": contacts })))
}

// ---------------------------------------------------------------------------
// DNS records: /api/dns/list, /api/dns/register, /api/dns/remove,
//              /api/dns/:id/edit, /api/dns/search
// ---------------------------------------------------------------------------
pub async fn handle_dns_list(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    use diesel::prelude::*;

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

    let rows: Vec<DnsRow> = diesel::sql_query(
        "SELECT id, hostname, record_type, target, ttl, status FROM dns_records ORDER BY hostname ASC LIMIT 200",
    )
    .load(&mut conn)
    .unwrap_or_default();

    if rows.is_empty() {
        return Html("<tr><td colspan=\"7\" class=\"empty-cell\">No DNS records yet</td></tr>".to_string());
    }

    let mut html = String::new();
    for r in &rows {
        html.push_str(&format!(
            r#"<tr>
    <td>{hostname}</td><td>{rtype}</td><td>{target}</td><td>{ttl}</td>
    <td><span class="status-badge {status}">{status}</span></td>
    <td>{created}</td>
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
            created = "-",
            id = r.id,
        ));
    }
    Html(html)
}
