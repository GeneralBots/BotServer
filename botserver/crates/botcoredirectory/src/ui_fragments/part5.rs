use super::*;

pub async fn group_detail(
    State(state): State<Arc<DirectoryApiState>>,
    Path(id): Path<uuid::Uuid>,
) -> Html<String> {
    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => return Html(String::new()),
    };

    #[derive(Debug, diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct GroupRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        display_name: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        description: Option<String>,
        #[diesel(sql_type = diesel::sql_types::Bool)]
        is_active: bool,
    }

    let group: Option<GroupRow> = diesel::sql_query(
        "SELECT name, display_name, description, is_active FROM rbac_groups WHERE id = $1",
    )
    .bind::<diesel::sql_types::Uuid, _>(id)
    .get_result(&mut conn)
    .ok();

    match group {
        Some(g) => Html(format!(
            r##"<div class="detail-section">
    <div class="detail-label">Group Name</div>
    <div class="detail-value">{display_name}</div>
</div>
<div class="detail-section">
    <div class="detail-label">Slug</div>
    <div class="detail-value">@{name}</div>
</div>
<div class="detail-section">
    <div class="detail-label">Description</div>
    <div class="detail-value">{desc}</div>
</div>
<div class="detail-section">
    <div class="detail-label">Status</div>
    <div class="detail-value">{status}</div>
</div>"##,
            display_name = html_escape(&g.display_name),
            name = html_escape(&g.name),
            desc = html_escape(g.description.as_deref().unwrap_or("—")),
            status = if g.is_active { "Active" } else { "Inactive" },
        )),
        None => Html(r#"<p class="text-muted">Group not found</p>"#.to_string()),
    }
}
pub async fn create_user(
    State(state): State<Arc<DirectoryApiState>>,
    axum::extract::Form(form): axum::extract::Form<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let username = form.get("username").cloned().unwrap_or_default();
    let email = form.get("email").cloned().unwrap_or_default();
    let password = form.get("password").cloned().unwrap_or_default();
    let role = form.get("role").cloned().unwrap_or_else(|| "user".to_string());
    let status = form.get("status").cloned().unwrap_or_else(|| "active".to_string());

    if username.trim().is_empty() || email.trim().is_empty() || password.len() < 8 {
        return (
            StatusCode::BAD_REQUEST,
            Html("Username, email and a password of at least 8 characters are required".to_string()),
        );
    }

    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("Database unavailable".to_string()),
            );
        }
    };

    let password_hash = match hash_password(&password) {
        Ok(h) => h,
        Err(e) => {
            log::error!("Password hashing failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("Failed to hash password".to_string()),
            );
        }
    };

    let user_id = uuid::Uuid::new_v4();
    let is_admin = role == "admin";
    let is_active = status != "inactive";

    let result = diesel::sql_query(
        "INSERT INTO users (id, username, email, password_hash, is_active, is_admin, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())",
    )
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .bind::<diesel::sql_types::Text, _>(&username)
    .bind::<diesel::sql_types::Text, _>(&email)
    .bind::<diesel::sql_types::Text, _>(&password_hash)
    .bind::<diesel::sql_types::Bool, _>(is_active)
    .bind::<diesel::sql_types::Bool, _>(is_admin)
    .execute(&mut conn);

    match result {
        Ok(_) => {
            log::info!("User {username} created via admin UI");
            (
                StatusCode::OK,
                Html(load_users_rows(
                    &state,
                    &SearchQuery { q: None, role: None, status: None, page: None },
                )),
            )
        }
        Err(e) => {
            log::error!("Failed to create user: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Html("Failed to create user".to_string()))
        }
    }
}
pub async fn update_user(
    State(state): State<Arc<DirectoryApiState>>,
    Path(id): Path<uuid::Uuid>,
    axum::extract::Form(form): axum::extract::Form<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let username = form.get("username").cloned().unwrap_or_default();
    let email = form.get("email").cloned().unwrap_or_default();
    let role = form.get("role").cloned().unwrap_or_else(|| "user".to_string());
    let status = form.get("status").cloned().unwrap_or_else(|| "active".to_string());

    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("Database unavailable".to_string()),
            );
        }
    };

    let is_admin = role == "admin";
    let is_active = status != "inactive";

    let result = diesel::sql_query(
        "UPDATE users SET username = $1, email = $2, is_admin = $3, is_active = $4, updated_at = NOW() WHERE id = $5",
    )
    .bind::<diesel::sql_types::Text, _>(&username)
    .bind::<diesel::sql_types::Text, _>(&email)
    .bind::<diesel::sql_types::Bool, _>(is_admin)
    .bind::<diesel::sql_types::Bool, _>(is_active)
    .bind::<diesel::sql_types::Uuid, _>(id)
    .execute(&mut conn);

    match result {
        Ok(_) => (
            StatusCode::OK,
            Html(load_users_rows(
                &state,
                &SearchQuery { q: None, role: None, status: None, page: None },
            )),
        ),
        Err(e) => {
            log::error!("Failed to update user {}: {}", id, e);
            (StatusCode::INTERNAL_SERVER_ERROR, Html("Failed to update user".to_string()))
        }
    }
}
pub(super) fn hash_password(password: &str) -> anyhow::Result<String> {
    use argon2::password_hash::{rand_core::OsRng, SaltString};
    use argon2::{Argon2, PasswordHasher};

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Password hashing failed: {}", e))?
        .to_string();
    Ok(hash)
}
pub(super) fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
