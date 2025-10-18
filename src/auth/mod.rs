use actix_web::{web, HttpResponse, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use log::{error, warn};
use redis::Client;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::shared;
use crate::shared::state::AppState;

pub struct AuthService {
    pub conn: PgConnection,
    pub redis: Option<Arc<Client>>,
}

impl AuthService {
    pub fn new(conn: PgConnection, redis: Option<Arc<Client>>) -> Self {
        Self { conn, redis }
    }

    pub fn verify_user(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<Option<Uuid>, Box<dyn std::error::Error + Send + Sync>> {
        use crate::shared::models::users;

        let user = users::table
            .filter(users::username.eq(username))
            .filter(users::is_active.eq(true))
            .select((users::id, users::password_hash))
            .first::<(Uuid, String)>(&mut self.conn)
            .optional()?;

        if let Some((user_id, password_hash)) = user {
            if let Ok(parsed_hash) = PasswordHash::new(&password_hash) {
                if Argon2::default()
                    .verify_password(password.as_bytes(), &parsed_hash)
                    .is_ok()
                {
                    return Ok(Some(user_id));
                }
            }
        }

        Ok(None)
    }

    pub fn create_user(
        &mut self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<Uuid, Box<dyn std::error::Error + Send + Sync>> {
        use crate::shared::models::users;
        use diesel::insert_into;

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
            .to_string();

        let user_id = Uuid::new_v4();

        insert_into(users::table)
            .values((
                users::id.eq(user_id),
                users::username.eq(username),
                users::email.eq(email),
                users::password_hash.eq(password_hash),
            ))
            .execute(&mut self.conn)?;

        Ok(user_id)
    }

    pub async fn delete_user_cache(
        &self,
        username: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_multiplexed_async_connection().await?;
            let cache_key = format!("auth:user:{}", username);

            let _: () = redis::Cmd::del(&cache_key).query_async(&mut conn).await?;
        }
        Ok(())
    }

    pub fn update_user_password(
        &mut self,
        user_id: Uuid,
        new_password: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::shared::models::users;
        use diesel::update;

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(new_password.as_bytes(), &salt)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
            .to_string();

        update(users::table.filter(users::id.eq(user_id)))
            .set((
                users::password_hash.eq(&password_hash),
                users::updated_at.eq(diesel::dsl::now),
            ))
            .execute(&mut self.conn)?;

        if let Some(username) = users::table
            .filter(users::id.eq(user_id))
            .select(users::username)
            .first::<String>(&mut self.conn)
            .optional()?
        {
            // Note: This would need to be handled differently in async context
            // For now, we'll just log it
            log::info!("Would delete cache for user: {}", username);
        }

        Ok(())
    }
    pub(crate) fn get_user_by_id(
        &mut self,
        _uid: Uuid,
    ) -> Result<Option<shared::models::User>, Box<dyn std::error::Error + Send + Sync>> {
        use crate::shared::models::users;

        let user = users::table
            // TODO:            .filter(users::id.eq(uid))
            .filter(users::is_active.eq(true))
            .first::<shared::models::User>(&mut self.conn)
            .optional()?;

        Ok(user)
    }
}

#[actix_web::get("/api/auth")]
async fn auth_handler(
    data: web::Data<AppState>,
    web::Query(params): web::Query<HashMap<String, String>>,
) -> Result<HttpResponse> {
    let _token = params.get("token").cloned().unwrap_or_default();
    let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
    let bot_id = if let Ok(bot_guid) = std::env::var("BOT_GUID") {
        match Uuid::parse_str(&bot_guid) {
            Ok(uuid) => uuid,
            Err(e) => {
                warn!("Invalid BOT_GUID from env: {}", e);
                return Ok(HttpResponse::BadRequest()
                    .json(serde_json::json!({"error": "Invalid BOT_GUID"})));
            }
        }
    } else {
        warn!("BOT_GUID not set in environment, using nil UUID");
        Uuid::nil()
    };

    let session = {
        let mut sm = data.session_manager.lock().await;
        match sm.get_or_create_user_session(user_id, bot_id, "Auth Session") {
            Ok(Some(s)) => s,
            Ok(None) => {
                error!("Failed to create session");
                return Ok(HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": "Failed to create session"})));
            }
            Err(e) => {
                error!("Failed to create session: {}", e);
                return Ok(HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": e.to_string()})));
            }
        }
    };

    let session_id_clone = session.id.clone();
    let auth_script_path = "./templates/annoucements.gbai/annoucements.gbdialog/auth.bas";
    let auth_script = match std::fs::read_to_string(auth_script_path) {
        Ok(content) => content,
        Err(_) => r#"SET_USER "00000000-0000-0000-0000-000000000001""#.to_string(),
    };

    let script_service = crate::basic::ScriptService::new(Arc::clone(&data), session.clone());
    match script_service
        .compile(&auth_script)
        .and_then(|ast| script_service.run(&ast))
    {
        Ok(result) => {
            if result.to_string() == "false" {
                error!("Auth script returned false, authentication failed");
                return Ok(HttpResponse::Unauthorized()
                    .json(serde_json::json!({"error": "Authentication failed"})));
            }
        }
        Err(e) => {
            error!("Failed to run auth script: {}", e);
            return Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Auth failed"})));
        }
    }

    let session = {
        let mut sm = data.session_manager.lock().await;
        match sm.get_session_by_id(session_id_clone) {
            Ok(Some(s)) => s,
            Ok(None) => {
                error!("Failed to retrieve session");
                return Ok(HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": "Failed to retrieve session"})));
            }
            Err(e) => {
                error!("Failed to retrieve session: {}", e);
                return Ok(HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": e.to_string()})));
            }
        }
    };

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "user_id": session.user_id,
        "session_id": session.id,
        "status": "authenticated"
    })))
}
