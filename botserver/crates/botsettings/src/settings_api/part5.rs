use super::*;

pub async fn users_create(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Form(form): axum::extract::Form<UserCreateForm>,
) -> impl IntoResponse {
    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Html("Database unavailable".to_string())),
    };
    let caller_id = match resolve_user_id(&state, &headers) {
        Ok(id) => id,
        Err(e) => return e,
    };
    if !is_admin_user(&state, caller_id) {
        return (StatusCode::FORBIDDEN, Html("Admin role required".to_string()));
    }

    let username = form.username.unwrap_or_default();
    let email = form.email.unwrap_or_default();
    let password = form.password.unwrap_or_default();
    if username.trim().is_empty() || email.trim().is_empty() || password.len() < 8 {
        return (StatusCode::BAD_REQUEST, Html("Username, email and a password of at least 8 characters are required".to_string()));
    }

    let hash = match hash_password(&password) {
        Ok(h) => h,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Html("Failed to hash password".to_string())),
    };
    let is_admin = form.role.as_deref() == Some("admin");

    let inserted = diesel::sql_query(
        "INSERT INTO users (id, username, email, password_hash, is_active, is_admin, created_at, updated_at)
         VALUES ($1, $2, $3, $4, true, $5, NOW(), NOW()) ON CONFLICT (email) DO NOTHING",
    )
    .bind::<diesel::sql_types::Uuid, _>(uuid::Uuid::new_v4())
    .bind::<diesel::sql_types::Text, _>(&username)
    .bind::<diesel::sql_types::Text, _>(&email)
    .bind::<diesel::sql_types::Text, _>(&hash)
    .bind::<diesel::sql_types::Bool, _>(is_admin)
    .execute(&mut conn);

    match inserted {
        Ok(1) => (StatusCode::OK, Html(format!("User <strong>{}</strong> created", html_escape(&username)))),
        Ok(_) => (StatusCode::CONFLICT, Html("A user with this email already exists".to_string())),
        Err(e) => {
            log::error!("users_create insert failed: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Html("Failed to create user".to_string()))
        }
    }
}
pub fn hash_password(password: &str) -> anyhow::Result<String> {
    use argon2::password_hash::{rand_core::OsRng, SaltString};
    use argon2::{Argon2, PasswordHasher};

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    Ok(argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Password hashing failed: {e}"))?
        .to_string())
}
pub fn random_key(len: usize) -> String {
    use rand::Rng;
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::rng();
    (0..len)
        .map(|_| CHARS[rng.random_range(0..CHARS.len())] as char)
        .collect()
}
