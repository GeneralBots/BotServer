use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use redis::Client;
use sqlx::{PgPool, Row}; // <-- required for .get()
use std::sync::Arc;
use uuid::Uuid;

pub struct AuthService {
    pub pool: PgPool,
    pub redis: Option<Arc<Client>>,
}

impl AuthService {
    #[allow(clippy::new_without_default)]
    pub fn new(pool: PgPool, redis: Option<Arc<Client>>) -> Self {
        Self { pool, redis }
    }

    pub async fn verify_user(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Option<Uuid>, Box<dyn std::error::Error>> {
        let user = sqlx::query(
            "SELECT id, password_hash FROM users WHERE username = $1 AND is_active = true",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = user {
            let user_id: Uuid = row.get("id");
            let password_hash: String = row.get("password_hash");

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

    pub async fn create_user(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<Uuid, Box<dyn std::error::Error>> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = match argon2.hash_password(password.as_bytes(), &salt) {
            Ok(ph) => ph.to_string(),
            Err(e) => {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            }
        };

        let row = sqlx::query(
            "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(username)
        .bind(email)
        .bind(&password_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get::<Uuid, _>("id"))
    }

    pub async fn delete_user_cache(
        &self,
        username: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_multiplexed_async_connection().await?;
            let cache_key = format!("auth:user:{}", username);

            let _: () = redis::Cmd::del(&cache_key).query_async(&mut conn).await?;
        }
        Ok(())
    }

    pub async fn update_user_password(
        &self,
        user_id: Uuid,
        new_password: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = match argon2.hash_password(new_password.as_bytes(), &salt) {
            Ok(ph) => ph.to_string(),
            Err(e) => {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            }
        };

        sqlx::query("UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2")
            .bind(&password_hash)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        if let Some(user_row) = sqlx::query("SELECT username FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?
        {
            let username: String = user_row.get("username");
            self.delete_user_cache(&username).await?;
        }

        Ok(())
    }
}

impl Clone for AuthService {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            redis: self.redis.clone(),
        }
    }
}
