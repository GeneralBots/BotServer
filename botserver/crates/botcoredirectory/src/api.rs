use crate::{BotAccess, UserAccount, UserProvisioningService, UserRole};
use anyhow::Result;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Router,
};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

pub struct DirectoryApiState {
    pub conn: DbPool,
    pub base_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub organization: String,
    pub is_admin: bool,
    pub bots: Vec<BotAccessRequest>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BotAccessRequest {
    pub bot_id: String,
    pub bot_name: String,
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub success: bool,
    pub message: String,
    pub user_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ServiceStatusResponse {
    pub directory: bool,
    pub database: bool,
    pub drive: bool,
    pub email: bool,
    pub git: bool,
}

pub fn configure_user_routes() -> Router<Arc<DirectoryApiState>> {
    Router::new()
        .route("/api/directory/users", get(list_users_handler))
        .route("/api/directory/users/:id", get(get_user_handler))
        .route("/api/directory/users/provision", post(provision_user_handler))
        .route("/api/directory/users/:id/deprovision", delete(deprovision_user_handler))
        .route("/api/directory/services/status", get(check_services_status))
        .route(
            "/api/directory/users/create",
            axum::routing::post(crate::ui_fragments::create_user),
        )
        .route(
            "/api/directory/users/:id",
            axum::routing::put(crate::ui_fragments::update_user),
        )
        .merge(crate::ui_fragments::configure_directory_ui_routes())
        .merge(crate::ui_api::configure_directory_api_routes())
}

pub async fn provision_user_handler(
    State(state): State<Arc<DirectoryApiState>>,
    Json(request): Json<CreateUserRequest>,
) -> impl IntoResponse {
    let mut account = UserAccount {
        username: request.username.clone(),
        email: request.email,
        first_name: request.first_name,
        last_name: request.last_name,
        organization: request.organization,
        is_admin: request.is_admin,
        bots: Vec::new(),
    };

    for bot_req in request.bots {
        let role = match bot_req.role.to_lowercase().as_str() {
            "admin" => UserRole::Admin,
            "readonly" | "read_only" => UserRole::ReadOnly,
            _ => UserRole::User,
        };

        account.bots.push(BotAccess {
            bot_id: bot_req.bot_id,
            bot_name: bot_req.bot_name.clone(),
            role,
            home_path: format!("/home/{}", request.username),
        });
    }

    let provisioning = UserProvisioningService::new(state.conn.clone(), None, state.base_url.clone());

    match provisioning.provision_user(&account).await {
        Ok(_) => (
            StatusCode::CREATED,
            Json(UserResponse {
                success: true,
                message: format!("User {} created successfully", account.username),
                user_id: Some(account.username),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(UserResponse {
                success: false,
                message: format!("Failed to provision user: {}", e),
                user_id: None,
            }),
        ),
    }
}

pub async fn deprovision_user_handler(
    State(state): State<Arc<DirectoryApiState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let provisioning = UserProvisioningService::new(state.conn.clone(), None, state.base_url.clone());

    match provisioning.deprovision_user(&id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(UserResponse {
                success: true,
                message: format!("User {} deleted successfully", id),
                user_id: None,
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(UserResponse {
                success: false,
                message: format!("Failed to deprovision user: {}", e),
                user_id: None,
            }),
        ),
    }
}

pub async fn get_user_handler(
    State(state): State<Arc<DirectoryApiState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    use diesel::prelude::*;

    let mut conn = match state.conn.get() {
        Ok(conn) => conn,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": botsecurity::error_sanitizer::log_and_sanitize_json(&e, "botcoredirectory::api", None)})),
            );
        }
    };

    let Ok(user_uuid) = Uuid::parse_str(&id) else {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid user ID format"})));
    };

    let user_result: Result<(Uuid, String, String, bool), _> = schema::users::table
        .filter(schema::users::id.eq(user_uuid))
        .select((schema::users::id, schema::users::username, schema::users::email, schema::users::is_admin))
        .first(&mut conn);

    match user_result {
        Ok((user_id, username, email, is_admin)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "id": user_id.to_string(),
                "username": username,
                "email": email,
                "is_admin": is_admin
            })),
        ),
        Err(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "User not found"}))),
    }
}

pub async fn list_users_handler(State(state): State<Arc<DirectoryApiState>>) -> impl IntoResponse {
    use diesel::prelude::*;

    let mut conn = match state.conn.get() {
        Ok(conn) => conn,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": botsecurity::error_sanitizer::log_and_sanitize_json(&e, "botcoredirectory::api", None)})),
            );
        }
    };

    let users_result: Result<Vec<(Uuid, String, String, bool)>, _> = schema::users::table
        .select((schema::users::id, schema::users::username, schema::users::email, schema::users::is_admin))
        .load(&mut conn);

    match users_result {
        Ok(users_list) => {
            let users_json: Vec<_> = users_list
                .into_iter()
                .map(|(user_id, username, email, is_admin)| {
                    serde_json::json!({
                        "id": user_id.to_string(),
                        "username": username,
                        "email": email,
                        "is_admin": is_admin
                    })
                })
                .collect();
            (StatusCode::OK, Json(serde_json::json!({"users": users_json})))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": botsecurity::error_sanitizer::log_and_sanitize_json(&e, "botcoredirectory::api", None)})),
        ),
    }
}

pub async fn check_services_status(State(state): State<Arc<DirectoryApiState>>) -> impl IntoResponse {
    let mut status = ServiceStatusResponse {
        directory: false,
        database: false,
        drive: false,
        email: false,
        git: false,
    };

    status.database = state.conn.get().is_ok();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default();

    // Server-side request forgery guard: the configured directory base URL
    // must be an http(s) URL with an explicit host before it is requested.
    let directory_ok = url::Url::parse(&state.base_url).is_ok_and(|parsed| {
        matches!(parsed.scheme(), "http" | "https") && parsed.host_str().is_some()
    });
    if directory_ok {
        if let Ok(response) = client.get(format!("{}/healthz", state.base_url)).send().await {
            status.directory = response.status().is_success();
        }
    }

    if let Ok(response) = client.get("https://localhost:8025/health").send().await {
        status.email = response.status().is_success();
    }

    if let Ok(response) = client.get("https://localhost:3000/api/version").send().await {
        status.git = response.status().is_success();
    }

    (StatusCode::OK, Json(status))
}

mod schema {
  use diesel::table;

  table! {
    users (id) {
      id -> Uuid,
      username -> Varchar,
      email -> Varchar,
      password_hash -> Text,
      is_admin -> Bool,
      created_at -> Timestamptz,
    }
  }
}
