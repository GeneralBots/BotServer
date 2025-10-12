use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use redis::Client;
use std::sync::Arc;
use uuid::Uuid;

use crate::shared;

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
