use super::*;

pub async fn groups_create(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Form(form): axum::extract::Form<GroupCreateForm>,
) -> impl IntoResponse {
    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Html("Database unavailable".to_string())),
    };
    let user_id = match resolve_user_id(&state, &headers) {
        Ok(id) => id,
        Err(e) => return e,
    };
    if !is_admin_user(&state, user_id) {
        return (StatusCode::FORBIDDEN, Html("Admin role required".to_string()));
    }

    let name = form.name.unwrap_or_default();
    let display = form.display_name.clone().unwrap_or_else(|| name.clone());
    if name.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Html("Group name is required".to_string()));
    }

    let _ = diesel::sql_query(
        "INSERT INTO rbac_groups (id, name, display_name, description, is_active, created_by, created_at, updated_at)
         VALUES ($1, $2, $3, $4, true, $5, NOW(), NOW())",
    )
    .bind::<diesel::sql_types::Uuid, _>(uuid::Uuid::new_v4())
    .bind::<diesel::sql_types::Text, _>(&name)
    .bind::<diesel::sql_types::Text, _>(&display)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(form.description)
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .execute(&mut conn);

    (StatusCode::OK, Html(format!("Group <strong>{}</strong> created", html_escape(&display))))
}
