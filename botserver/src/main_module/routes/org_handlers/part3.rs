use super::*;

pub async fn handle_create_organization(
    State(state): State<Arc<AppState>>,
    axum::extract::Form(form): axum::extract::Form<CreateOrgRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let name = form.name.unwrap_or_default().trim().to_string();
    if name.is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "Organization name is required"
        }))));
    }

    let slug = form
        .slug
        .unwrap_or_else(|| name.to_lowercase().replace(' ', "-"));
    let org_id = uuid::Uuid::new_v4();

    let mut conn = state.conn.get().map_err(|e| {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(
            crate::security::error_sanitizer::log_and_sanitize_json(
                &e,
                "organization create: database pool",
                None,
            ),
        ))
    })?;

    // Resolve a real tenant row — the FK requires an existing tenants.id
    // (fix #840: the previous code bound Uuid::nil(), which never exists,
    // making every insert fail silently).
    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct TenantRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: uuid::Uuid,
    }
    let tenant_id: uuid::Uuid = diesel::sql_query(
        "SELECT id FROM tenants ORDER BY created_at ASC LIMIT 1",
    )
    .get_result::<TenantRow>(&mut conn)
    .optional()
    .map_err(|e| {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(
            crate::security::error_sanitizer::log_and_sanitize_json(
                &e,
                "organization create: tenant lookup",
                None,
            ),
        ))
    })?
    .map(|r| r.id)
    .ok_or((axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
        "error": "No tenant exists — run migrations/bootstrap first"
    }))))?;

    use diesel::RunQueryDsl;
    let inserted = diesel::sql_query(
        "INSERT INTO organizations (org_id, tenant_id, name, slug) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
    )
    .bind::<diesel::sql_types::Uuid, _>(&org_id)
    .bind::<diesel::sql_types::Uuid, _>(&tenant_id)
    .bind::<diesel::sql_types::Text, _>(&name)
    .bind::<diesel::sql_types::Text, _>(&slug)
    .execute(&mut conn)
    .map_err(|e| {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(
            crate::security::error_sanitizer::log_and_sanitize_json(
                &e,
                "organization create: insert",
                None,
            ),
        ))
    })?;

    if inserted == 0 {
        // Slug already exists (ON CONFLICT DO NOTHING) — resolve the existing row.
        let existing: TenantRow = diesel::sql_query(
            "SELECT org_id AS id FROM organizations WHERE slug = $1 LIMIT 1",
        )
        .bind::<diesel::sql_types::Text, _>(&slug)
        .get_result::<TenantRow>(&mut conn)
        .map_err(|e| {
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(
                crate::security::error_sanitizer::log_and_sanitize_json(
                    &e,
                    "organization create: lookup by slug",
                    None,
                ),
            ))
        })?;
        return Ok(Json(serde_json::json!({
            "success": true,
            "org_id": existing.id,
            "name": name,
            "slug": slug,
            "duplicate": true,
        })));
    }

    // Create a default branch so the org has a usable workspace scope.
    let branch_id = uuid::Uuid::new_v4();
    let branch_slug = format!("{slug}-default");
    let _ = diesel::sql_query(
        "INSERT INTO branches (id, org_id, tenant_id, slug, name) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (org_id, slug) DO NOTHING",
    )
    .bind::<diesel::sql_types::Uuid, _>(&branch_id)
    .bind::<diesel::sql_types::Uuid, _>(&org_id)
    .bind::<diesel::sql_types::Uuid, _>(&tenant_id)
    .bind::<diesel::sql_types::Text, _>(&branch_slug)
    .bind::<diesel::sql_types::Text, _>(&name)
    .execute(&mut conn);

    append_audit_log("organization_created", &name);

    Ok(Json(serde_json::json!({
        "success": true,
        "org_id": org_id,
        "branch_id": branch_id,
        "name": name,
        "slug": slug,
    })))
}
