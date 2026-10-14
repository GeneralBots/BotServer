use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

pub use botlib::db_pool::DbPool;

#[cfg(feature = "drive")]
pub struct S3ClientStub;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccount {
    pub username: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub organization: String,
    pub is_admin: bool,
    pub bots: Vec<BotAccess>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotAccess {
    pub bot_id: String,
    pub bot_name: String,
    pub role: UserRole,
    pub home_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserRole {
    Admin,
    User,
    ReadOnly,
}

pub struct UserProvisioningService {
    db_pool: DbPool,
    #[cfg(feature = "drive")]
    s3_client: Option<Arc<S3ClientStub>>,
    #[cfg(not(feature = "drive"))]
    s3_client: Option<Arc<()>>,
    base_url: String,
}

impl std::fmt::Debug for UserProvisioningService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserProvisioningService")
            .field("base_url", &self.base_url)
            .field("has_s3_client", &self.s3_client.is_some())
            .finish_non_exhaustive()
    }
}

impl UserProvisioningService {
    #[cfg(feature = "drive")]
    pub fn new(db_pool: DbPool, s3_client: Option<Arc<S3ClientStub>>, base_url: String) -> Self {
        Self {
            db_pool,
            s3_client,
            base_url,
        }
    }

    #[cfg(not(feature = "drive"))]
    pub fn new(db_pool: DbPool, _s3_client: Option<Arc<()>>, base_url: String) -> Self {
        Self {
            db_pool,
            s3_client: None,
            base_url,
        }
    }

    pub fn get_base_url(&self) -> String {
        self.base_url.clone()
    }

    pub fn build_profile_url(&self, username: &str) -> String {
        format!("{}/users/{}/profile", self.base_url, username)
    }

    pub async fn provision_user(&self, account: &UserAccount) -> Result<()> {
        log::info!("Provisioning user: {} via directory at {}", account.username, self.base_url);

        let user_id = self.create_database_user(account)?;

        for bot_access in &account.bots {
            self.create_s3_home(account, bot_access).await?;
        }

        if let Err(e) = self.setup_email_account(account, &user_id) {
            log::warn!("Email account creation failed: {}", e);
        }

        self.setup_oauth_config(&user_id, account)?;

        let profile_url = self.build_profile_url(&account.username);
        log::info!("User {} provisioned successfully. Profile: {}", account.username, profile_url);
        Ok(())
    }

    fn create_database_user(&self, account: &UserAccount) -> Result<String> {
        use schema::users;
        use argon2::{
            password_hash::{rand_core::OsRng, SaltString},
            Argon2, PasswordHasher,
        };
        use diesel::prelude::*;

        let user_id = Uuid::new_v4();
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(Uuid::new_v4().to_string().as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Password hashing failed: {}", e))?
            .to_string();

        let mut conn = self
            .db_pool
            .get()
            .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))?;

        diesel::insert_into(users::table)
            .values((
                users::id.eq(user_id),
                users::username.eq(&account.username),
                users::email.eq(&account.email),
                users::password_hash.eq(&password_hash),
                users::is_admin.eq(account.is_admin),
                users::created_at.eq(chrono::Utc::now()),
            ))
            .execute(&mut conn)?;

        Ok(user_id.to_string())
    }

    async fn create_s3_home(&self, account: &UserAccount, bot_access: &BotAccess) -> Result<()> {
        #[cfg(feature = "drive")]
        {
            let _ = account;
            let _ = bot_access;
            log::info!("S3 home creation would happen here for drive feature");
        }

        #[cfg(not(feature = "drive"))]
        {
            let _ = account;
            let _ = bot_access;
            log::debug!("Drive feature not enabled, skipping S3 home creation");
        }

        Ok(())
    }

    #[cfg(feature = "mail")]
    fn setup_email_account(&self, account: &UserAccount, user_uuid: &str) -> Result<()> {
        use schema::user_email_accounts;
        use diesel::prelude::*;

        let mut conn = self
            .db_pool
            .get()
            .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))?;

        // Bind to the REAL user id created by create_database_user — a random
        // UUID here means the Mail app (which queries by user_id) never finds
        // the account.
        let user_uuid = uuid::Uuid::parse_str(user_uuid)
            .map_err(|e| anyhow::anyhow!("Invalid user id: {e}"))?;
        // Real mail server from env (Vault-backed in prod); falls back to the
        // old localhost stub only when unconfigured.
        let imap_server = std::env::var("MAIL_HOST")
            .unwrap_or_else(|_| "localhost".to_string());
        let smtp_server = imap_server.clone();
        let imap_port: i32 = std::env::var("MAIL_IMAP_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(993);
        let smtp_port: i32 = std::env::var("MAIL_SMTP_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(587);
        let password = std::env::var("MAIL_PASS").unwrap_or_else(|_| "oauth".to_string());

        diesel::insert_into(user_email_accounts::table)
            .values((
                user_email_accounts::user_id.eq(user_uuid),
                user_email_accounts::email.eq(&account.email),
                user_email_accounts::imap_server.eq(imap_server),
                user_email_accounts::imap_port.eq(imap_port),
                user_email_accounts::smtp_server.eq(smtp_server),
                user_email_accounts::smtp_port.eq(smtp_port),
                user_email_accounts::username.eq(&account.username),
                user_email_accounts::password_encrypted.eq(password),
                user_email_accounts::is_active.eq(true),
            ))
            .execute(&mut conn)?;

        log::info!("Setup email configuration for: {}", account.email);
        Ok(())
    }

    #[cfg(not(feature = "mail"))]
    fn setup_email_account(&self, _account: &UserAccount, _user_uuid: &str) -> Result<()> {
        log::debug!("Email feature not enabled, skipping email account setup");
        Ok(())
    }

    fn setup_oauth_config(&self, _user_id: &str, account: &UserAccount) -> Result<()> {
        use botcoresecrets::manager::SecretsManager;
        use std::collections::HashMap;

        let services: HashMap<String, String> = [
            ("oauth-drive-enabled", "true"),
            ("oauth-email-enabled", "true"),
            ("oauth-git-enabled", "true"),
            ("oauth-provider", "zitadel"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

        let path = format!("gbo/{}/{}/{}", uuid::Uuid::nil(), uuid::Uuid::nil(), uuid::Uuid::nil());
        let sm = SecretsManager::get()
            .map_err(|e| anyhow::anyhow!("Failed to get SecretsManager: {}", e))?;

        let sm_clone = sm.clone();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create runtime: {}", e))?;

        rt.block_on(async move {
            sm_clone.put_secret(&path, services).await
                .map_err(|e| anyhow::anyhow!("Failed to write OAuth config to Vault: {}", e))
        })?;

        log::info!("Setup OAuth configuration for user: {}", account.username);
        Ok(())
    }

    pub async fn deprovision_user(&self, username: &str) -> Result<()> {
        log::info!("Deprovisioning user: {}", username);

        self.remove_s3_data(username).await?;
        self.remove_email_config(username)?;
        self.remove_user_from_db(username)?;

        log::info!("User {} deprovisioned successfully", username);
        Ok(())
    }

    fn remove_user_from_db(&self, username: &str) -> Result<()> {
        use schema::users;
        use diesel::prelude::*;

        let mut conn = self
            .db_pool
            .get()
            .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))?;
        diesel::delete(users::table.filter(users::username.eq(username))).execute(&mut conn)?;

        Ok(())
    }

    async fn remove_s3_data(&self, username: &str) -> Result<()> {
        #[cfg(feature = "drive")]
        {
            let _ = username;
            log::info!("S3 data removal would happen here for drive feature");
        }

        #[cfg(not(feature = "drive"))]
        {
            let _ = username;
            log::debug!("Drive feature not enabled, bypassing S3 data removal");
        }

        Ok(())
    }

    #[cfg(feature = "mail")]
    fn remove_email_config(&self, username: &str) -> Result<()> {
        use schema::user_email_accounts;
        use diesel::prelude::*;

        let mut conn = self
            .db_pool
            .get()
            .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))?;
        diesel::delete(
            user_email_accounts::table.filter(user_email_accounts::username.eq(username)),
        )
        .execute(&mut conn)?;

        Ok(())
    }

    #[cfg(not(feature = "mail"))]
    fn remove_email_config(&self, _username: &str) -> Result<()> {
        log::debug!("Email feature not enabled, skipping email config removal");
        Ok(())
    }
}

mod schema {
  use diesel::table;
  use diesel::allow_tables_to_appear_in_same_query;

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

  table! {
    bot_configuration (bot_id, config_key) {
      bot_id -> Uuid,
      config_key -> Varchar,
      config_value -> Text,
      is_encrypted -> Bool,
      config_type -> Varchar,
      created_at -> Timestamptz,
      updated_at -> Timestamptz,
    }
  }

  table! {
    user_email_accounts (user_id) {
      user_id -> Uuid,
      email -> Varchar,
      imap_server -> Varchar,
      imap_port -> Int4,
      smtp_server -> Varchar,
      smtp_port -> Int4,
      username -> Varchar,
      password_encrypted -> Text,
      is_active -> Bool,
    }
  }

  allow_tables_to_appear_in_same_query!(users, bot_configuration, user_email_accounts,);
}
