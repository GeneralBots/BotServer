use super::*;


// ─────────────────────────────────────────────────────────────────────────────
/// `GET /api/cloud/services`
/// Returns provisioned services (active subscriptions).
/// `GET /api/cloud/bots`
pub(crate) async fn list_bots(
    State(service): State<Arc<SaasService>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut conn = service.pool().get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB: {e}")))?;

    let user_branch_id = get_branch_id_from_jwt(&headers, &mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let admin = is_super_admin(&headers, &mut conn).unwrap_or(false);

    #[derive(QueryableByName, Debug)]
    struct BotRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        slug: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        description: Option<String>,
        #[diesel(sql_type = diesel::sql_types::Bool)]
        is_active: bool,
    }

    let rows: Vec<BotRow> = if admin {
        diesel::sql_query("SELECT id, name, slug, description, is_active FROM bots ORDER BY name")
            .load(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query: {e}")))?
    } else if let Some(bid) = user_branch_id {
        diesel::sql_query(
            "SELECT id, name, slug, description, is_active FROM bots WHERE branch_id = $1 ORDER BY name"
        )
        .bind::<diesel::sql_types::Uuid, _>(bid)
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query: {e}")))?
    } else {
        Vec::new()
    };

    let result: Vec<serde_json::Value> = rows.into_iter().map(|r| {
        serde_json::json!({
            "id": r.id,
            "name": r.name,
            "slug": r.slug,
            "description": r.description,
            "is_active": r.is_active,
        })
    }).collect();

    Ok(Json(serde_json::json!({ "bots": result })))
}

pub(crate) async fn list_services(
    State(service): State<Arc<SaasService>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut conn = service.pool().get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB: {e}")))?;

    let user_branch_id = get_branch_id_from_jwt(&headers, &mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let admin = is_super_admin(&headers, &mut conn).unwrap_or(false);

    use crate::schema_ext::billing_recurring::dsl::*;
    let mut query = billing_recurring
        .select((id, customer_name, description, frequency, status, amount, currency, interval_count, start_date, created_at))
        .into_boxed();

    if let Some(bid) = user_branch_id {
        query = query.filter(branch_id.eq(bid));
    }

    let subs = query
        .load::<(Uuid, String, Option<String>, String, Option<String>, Option<bigdecimal::BigDecimal>, Option<String>, i32, chrono::NaiveDate, chrono::DateTime<chrono::Utc>)>(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query: {e}")))?;

    let result: Vec<serde_json::Value> = subs.into_iter().map(|(sid, cname, desc, freq, stat, amt, cur, _interval, sdate, cdate)| {
        let plan_label = desc.as_deref().unwrap_or(&cname);
        let friendly_desc = if stat.as_deref() == Some("active") && amt.as_ref().map(botbilling::api_models::bd_to_f64) == Some(0.0) {
            "GBO Free Service".to_string()
        } else if stat.as_deref() == Some("trialing") {
            format!("GBO {} Service (Trial)", plan_label)
        } else if stat.as_deref() == Some("active") {
            format!("GBO {} Service", plan_label)
        } else {
            plan_label.to_string()
        };
        serde_json::json!({
            "id": sid,
            "name": friendly_desc,
            "description": desc,
            "status": stat,
            "amount": amt.as_ref().map(|a| a.to_string()),
            "currency": cur,
            "period": if _interval > 1 { format!("every {} {}", _interval, freq) } else { freq.to_string() },
            "created_at": cdate.to_rfc3339(),
            "expires_at": sdate,
        })
    }).collect();

    let mut result = result;

    // When accessing the cloud for the default org (base system), the host that
    // runs the entire stack is itself the VPS — surface it as a service.
    #[derive(QueryableByName)]
    struct DefaultBranchRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        value: Uuid,
    }
    let default_branch_id: Option<Uuid> = diesel::sql_query(
        "SELECT b.id AS value FROM branches b JOIN organizations o ON o.org_id = b.org_id \
         WHERE o.slug = 'default' LIMIT 1"
    )
    .get_result::<DefaultBranchRow>(&mut conn)
    .optional()
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Default branch query: {e}")))?
    .map(|r| r.value);

    let is_default_access = admin
        || user_branch_id.is_none()
        || (user_branch_id.is_some() && default_branch_id == user_branch_id);

    if is_default_access {
        let scheme = headers
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            .filter(|s| !s.is_empty())
            .unwrap_or("http");
        let host = headers
            .get("host")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let dashboard_url = if !host.is_empty() {
            Some(format!("{scheme}://{host}"))
        } else if !service.config.base_url.is_empty() {
            Some(service.config.base_url.clone())
        } else {
            None
        };
        result.insert(0, serde_json::json!({
            "id": Uuid::nil(),
            "name": "Base System VPS (Own Host)",
            "description": "The host that runs the General Bots base system",
            "status": "active",
            "amount": "0",
            "currency": "USD",
            "period": "monthly",
            "created_at": chrono::Utc::now().to_rfc3339(),
            "expires_at": null,
            "is_base_system": true,
            "dashboard_url": dashboard_url,
        }));
    }

    Ok(Json(serde_json::json!({ "services": result })))
}

// ─────────────────────────────────────────────────────────────────────────────
// Invoices
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/cloud/invoices`
pub(crate) async fn list_invoices(
    State(service): State<Arc<SaasService>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut conn = service.pool().get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB: {e}")))?;

    let user_branch_id = get_branch_id_from_jwt(&headers, &mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    use botbilling::schema::billing_invoices::dsl::*;
    let mut query = billing_invoices
        .select((id, invoice_number, customer_name, total, status, issue_date, due_date))
        .into_boxed();

    if let Some(bid) = user_branch_id {
        query = query.filter(branch_id.eq(bid));
    }

    let invs = query
        .order(issue_date.desc())
        .limit(50)
        .load::<(Uuid, String, Option<String>, Option<bigdecimal::BigDecimal>, Option<String>, chrono::NaiveDate, Option<chrono::NaiveDate>)>(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query: {e}")))?;

    let result: Vec<serde_json::Value> = invs.into_iter().map(|(iid, inum, cname, tot, stat, idate, ddate)| {
        serde_json::json!({
            "id": iid,
            "number": inum,
            "customer": cname,
            "total": tot.map(|t| t.to_string()),
            "status": stat,
            "issue_date": idate.to_string(),
            "due_date": ddate.map(|d| d.to_string()),
        })
    }).collect();

    Ok(Json(serde_json::json!({ "invoices": result })))
}

// ─────────────────────────────────────────────────────────────────────────────
// Payment Cards (Stripe SetupIntent) — implemented in `crate::payment_cards`
// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// Store catalogue (products available for purchase)
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/cloud/store`
pub(crate) async fn list_store_items(
    State(_service): State<Arc<SaasService>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Static catalog with doubled prices, invisible providers
    let items = serde_json::json!({
        "items": [
            // ── VPS ──
            { "id":"vps-small",  "category":"compute", "name":"VPS Small",  "icon":"🖥️", "price_type":"fixed", "amount":999,  "currency":"usd", "period":"mo", "description":"4 vCPU · 8 GB RAM · 100 GB NVMe · 2 TB BW" },
            { "id":"vps-medium", "category":"compute", "name":"VPS Medium", "icon":"🖥️", "price_type":"fixed", "amount":1999, "currency":"usd", "period":"mo", "description":"6 vCPU · 16 GB RAM · 200 GB NVMe · 4 TB BW" },
            { "id":"vps-large",  "category":"compute", "name":"VPS Large",  "icon":"🖥️", "price_type":"fixed", "amount":3999, "currency":"usd", "period":"mo", "description":"8 vCPU · 32 GB RAM · 400 GB NVMe · 8 TB BW" },
            { "id":"vps-xl",     "category":"compute", "name":"VPS XL",     "icon":"🖥️", "price_type":"fixed", "amount":7999, "currency":"usd", "period":"mo", "description":"16 vCPU · 64 GB RAM · 800 GB NVMe · 16 TB BW" },
            // ── GPU ──
            { "id":"gpu-basic",      "category":"compute", "name":"GPU Basic",      "icon":"⚡", "price_type":"fixed", "amount":3999,  "currency":"usd", "period":"mo", "description":"RTX 3060 12 GB · 4 vCPU · 8 GB RAM" },
            { "id":"gpu-pro",        "category":"compute", "name":"GPU Pro",        "icon":"⚡", "price_type":"fixed", "amount":9999,  "currency":"usd", "period":"mo", "description":"RTX 4090 (24 GB VRAM) · 8 vCPU · 32 GB RAM" },
            { "id":"gpu-enterprise", "category":"compute", "name":"GPU Enterprise", "icon":"⚡", "price_type":"fixed", "amount":29999, "currency":"usd", "period":"mo", "description":"A100 80 GB · 16 vCPU · 64 GB RAM" },
            // ── Storage ──
            { "id":"storage-50",   "category":"storage", "name":"Storage 50 GB",  "icon":"💾", "price_type":"fixed", "amount":999,  "currency":"usd", "period":"mo", "description":"S3-compatible · 100 GB egress" },
            { "id":"storage-250",  "category":"storage", "name":"Storage 250 GB", "icon":"💾", "price_type":"fixed", "amount":2999, "currency":"usd", "period":"mo", "description":"S3-compatible · 500 GB egress · Versioning" },
            { "id":"storage-1tb",  "category":"storage", "name":"Storage 1 TB",   "icon":"💾", "price_type":"fixed", "amount":5999, "currency":"usd", "period":"mo", "description":"S3-compatible · 2 TB egress · Lifecycle mgmt" },
            { "id":"storage-10tb", "category":"storage", "name":"Storage 10 TB",  "icon":"💾", "price_type":"fixed", "amount":19999,"currency":"usd", "period":"mo", "description":"S3-compatible · 20 TB egress · Priority support" },
            // ── Numbers ──
            { "id":"number-local",    "category":"comms", "name":"Local Number",   "icon":"📞", "price_type":"fixed", "amount":599,  "currency":"usd", "period":"mo", "description":"1 number · SMS + Voice · WhatsApp-ready" },
            { "id":"number-global",   "category":"comms", "name":"Global Bundle",  "icon":"📞", "price_type":"fixed", "amount":1999, "currency":"usd", "period":"mo", "description":"3 numbers · Different countries" },
            { "id":"number-business", "category":"comms", "name":"Business Pack",  "icon":"📞", "price_type":"fixed", "amount":4999, "currency":"usd", "period":"mo", "description":"10 numbers · Any countries" },
            // ── Calls ──
            { "id":"calls-100",  "category":"comms", "name":"100 Min Bundle",  "icon":"📱", "price_type":"fixed", "amount":999,  "currency":"usd", "period":"once", "description":"100 outbound minutes · Global coverage" },
            { "id":"calls-500",  "category":"comms", "name":"500 Min Bundle",  "icon":"📱", "price_type":"fixed", "amount":3999, "currency":"usd", "period":"once", "description":"500 outbound minutes · Priority routing" },
            { "id":"calls-1000", "category":"comms", "name":"1000+ Min Bundle","icon":"📱", "price_type":"fixed", "amount":6999, "currency":"usd", "period":"once", "description":"1000 outbound minutes · Dedicated routes" },
            // ── Domains ──
            { "id":"domain-com",  "category":"domains", "name":".com Domain", "icon":"🌐", "price_type":"fixed", "amount":2199, "currency":"usd", "period":"yr", "description":"Free WHOIS privacy · Managed DNS" },
            { "id":"domain-io",   "category":"domains", "name":".io Domain",  "icon":"🌐", "price_type":"fixed", "amount":7199, "currency":"usd", "period":"yr", "description":"Free WHOIS privacy · Managed DNS" },
            { "id":"domain-ai",   "category":"domains", "name":".ai Domain",  "icon":"🌐", "price_type":"fixed", "amount":15999,"currency":"usd", "period":"yr", "description":"Free WHOIS privacy · Managed DNS" },
        ]
    });
    Ok(Json(items))
}

// ─────────────────────────────────────────────────────────────────────────────
// Profile
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/cloud/profile`
pub(crate) async fn get_profile(
    State(_service): State<Arc<SaasService>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // In production: decode JWT from Authorization header and fetch profile
    Ok(Json(serde_json::json!({
        "name": "",
        "email": "",
        "organization": "",
    })))
}

/// `POST /api/cloud/profile`
pub(crate) async fn update_profile(
    State(_service): State<Arc<SaasService>>,
    Json(body): Json<ProfileUpdateBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Stub: update name/org in DB associated with the authenticated user's JWT
    Ok(Json(serde_json::json!({
        "status": "ok",
        "name": body.name.unwrap_or_default(),
        "organization": body.organization.unwrap_or_default(),
    })))
}

// ─────────────────────────────────────────────────────────────────────────────
// Top-up (Special Offers)
