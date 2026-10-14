pub mod api;
pub mod provisioning;
pub mod auth_routes;
pub mod bootstrap;
pub mod client;
pub mod router;
pub mod scim;
pub mod ui_api;
pub mod ui_fragments;
pub mod users;
pub mod groups;
pub mod organizations;

pub use botlib::db_pool::DbPool;

pub use client::{ZitadelClient, ZitadelConfig};
pub type AuthService = ZitadelClient;

use anyhow::Result;
use botlib::traits::{BoxFutureString, BoxFutureUnit, BoxFutureValue, BoxFutureVecValue};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[cfg(feature = "drive")]
use crate::provisioning::S3ClientStub;

pub use provisioning::{BotAccess, UserAccount, UserProvisioningService, UserRole};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryConfig {
    pub url: String,
    pub admin_token: String,
    pub project_id: String,
    pub oauth_enabled: bool,
}

impl Default for DirectoryConfig {
    fn default() -> Self {
        Self {
            url: "https://localhost:8300".to_string(),
            admin_token: String::new(),
            project_id: "default".to_string(),
            oauth_enabled: true,
        }
    }
}

pub struct DirectoryService {
    config: DirectoryConfig,
    provisioning: Arc<UserProvisioningService>,
}

impl std::fmt::Debug for DirectoryService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirectoryService")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl DirectoryService {
    #[cfg(feature = "drive")]
    pub fn new(
        config: DirectoryConfig,
        db_pool: Pool<ConnectionManager<PgConnection>>,
        s3_client: Arc<S3ClientStub>,
    ) -> Result<Self> {
        let provisioning = Arc::new(UserProvisioningService::new(
            db_pool,
            Some(s3_client),
            config.url.clone(),
        ));
        Ok(Self { config, provisioning })
    }

    #[cfg(not(feature = "drive"))]
    pub fn new(
        config: DirectoryConfig,
        db_pool: Pool<ConnectionManager<PgConnection>>,
    ) -> Result<Self> {
        let provisioning = Arc::new(UserProvisioningService::new(
            db_pool,
            None,
            config.url.clone(),
        ));
        Ok(Self { config, provisioning })
    }

    pub async fn create_user(&self, account: UserAccount) -> Result<()> {
        self.provisioning.provision_user(&account).await
    }

    pub async fn delete_user(&self, username: &str) -> Result<()> {
        self.provisioning.deprovision_user(username).await
    }

    pub fn get_provisioning_service(&self) -> Arc<UserProvisioningService> {
        Arc::clone(&self.provisioning)
    }

    pub fn get_url(&self) -> String {
        self.config.url.clone()
    }

    pub fn is_oauth_enabled(&self) -> bool {
        self.config.oauth_enabled
    }

    pub fn get_project_id(&self) -> String {
        self.config.project_id.clone()
    }

    pub fn get_config(&self) -> &DirectoryConfig {
        &self.config
    }
}


impl botlib::traits::AuthServiceTrait for client::ZitadelClient {
    fn api_url(&self) -> String {
        client::ZitadelClient::api_url(self)
    }

    fn client_id(&self) -> String {
        client::ZitadelClient::client_id(self)
    }

    fn client_secret(&self) -> String {
        client::ZitadelClient::client_secret(self)
    }

    fn get_access_token(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>> {
        let client = self.clone();
        Box::pin(async move {
            client.get_access_token().await.map_err(|e| e.to_string())
        })
    }

    fn get_user_by_token(
        &self,
        token: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<serde_json::Value>, String>> + Send>> {
        let client = self.clone();
        let token = token.to_string();
        Box::pin(async move {
            let result = client.introspect_token(&token).await.map_err(|e| e.to_string())?;
            if result.get("active").and_then(|v| v.as_bool()).unwrap_or(false) {
                Ok(Some(result))
            } else {
                Ok(None)
            }
        })
    }

    fn create_user(
        &self,
        email: &str,
        first_name: &str,
        last_name: &str,
        username: Option<&str>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>> {
        let email = email.to_string();
        let first_name = first_name.to_string();
        let last_name = last_name.to_string();
        let username = username.map(String::from);
        let client = self.clone();
        Box::pin(async move {
            client.create_user(&email, &first_name, &last_name, username.as_deref()).await.map_err(|e| e.to_string())
        })
    }

    fn add_org_member(
        &self,
        org_id: &str,
        user_id: &str,
        roles: Vec<String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>> {
        let org_id = org_id.to_string();
        let user_id = user_id.to_string();
        let client = self.clone();
        Box::pin(async move {
            client.add_org_member(&org_id, &user_id, roles).await.map_err(|e| e.to_string())
        })
    }

    fn set_user_password(
        &self,
        user_id: &str,
        password: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>> {
        let user_id = user_id.to_string();
        let password = password.to_string();
        let client = self.clone();
        Box::pin(async move {
            client.set_user_password(&user_id, &password, false).await.map_err(|e| e.to_string())
        })
    }

    fn list_users(
        &self,
        limit: i64,
        offset: i64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send>> {
        let client = self.clone();
        Box::pin(async move {
            let users = client.list_users(limit as u32, offset as u32).await.map_err(|e| e.to_string())?;
            Ok(serde_json::json!(users))
        })
    }

    fn list_organizations(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send>> {
        let client = self.clone();
        Box::pin(async move {
            let resp = client
                .http_post(format!("{}/management/v1/orgs/_search", client.api_url())).await.map_err(|e| e.to_string())?
                .json(&serde_json::json!({}))
                .send().await.map_err(|e| e.to_string())?;
            let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            let orgs = data.get("result").and_then(|r| r.as_array()).cloned().unwrap_or_default();
            Ok(serde_json::json!(orgs))
        })
    }

    fn get_organization(
        &self,
        org_id: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send>> {
        let client = self.clone();
        let org_id = org_id.to_string();
        Box::pin(async move {
            client.get_organization(&org_id).await.map_err(|e| e.to_string())
        })
    }

    fn create_organization(
        &self,
        name: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>> {
        let client = self.clone();
        let name = name.to_string();
        Box::pin(async move {
            client.create_organization(&name).await.map_err(|e| e.to_string())
        })
    }

    fn http_get(
        &self,
        url: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send>> {
        let client = self.clone();
        Box::pin(async move {
            let resp = client.http_get(url).await.map_err(|e| e.to_string())?.send().await.map_err(|e| e.to_string())?;
            let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            Ok(data)
        })
    }

    fn http_post(
        &self,
        url: String,
        body: serde_json::Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send>> {
        let client = self.clone();
        Box::pin(async move {
            let resp = client.http_post(url).await.map_err(|e| e.to_string())?.json(&body).send().await.map_err(|e| e.to_string())?;
            let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            Ok(data)
        })
    }

    fn http_put(
        &self,
        url: String,
        body: serde_json::Value,
    ) -> BoxFutureValue {
        let client = self.clone();
        Box::pin(async move {
            let resp = client.http_put(url).await.map_err(|e| e.to_string())?.json(&body).send().await.map_err(|e| e.to_string())?;
            let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            Ok(data)
        })
    }

    fn http_patch(
        &self,
        url: String,
        body: serde_json::Value,
    ) -> BoxFutureValue {
        let client = self.clone();
        Box::pin(async move {
            let resp = client.http_patch(url).await.map_err(|e| e.to_string())?.json(&body).send().await.map_err(|e| e.to_string())?;
            let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            Ok(data)
        })
    }

    fn http_delete(
        &self,
        url: String,
    ) -> BoxFutureValue {
        let client = self.clone();
        Box::pin(async move {
            let resp = client.http_delete(url).await.map_err(|e| e.to_string())?.send().await.map_err(|e| e.to_string())?;
            if resp.status().is_success() {
                Ok(serde_json::Value::Null)
            } else {
                let text = resp.text().await.unwrap_or_default();
                Err(format!("DELETE failed: {}", text))
            }
        })
    }

    fn get_user(
        &self,
        user_id: &str,
    ) -> BoxFutureValue {
        let client = self.clone();
        let user_id = user_id.to_string();
        Box::pin(async move {
            client.get_user(&user_id).await.map_err(|e| e.to_string())
        })
    }

    fn search_users(
        &self,
        query: &str,
    ) -> BoxFutureVecValue {
        let client = self.clone();
        let query = query.to_string();
        Box::pin(async move {
            client.search_users(&query).await.map_err(|e| e.to_string())
        })
    }

    fn search_users_by_phone(
        &self,
        phone: &str,
    ) -> BoxFutureVecValue {
        let client = self.clone();
        let phone = phone.to_string();
        Box::pin(async move {
            client.search_users_by_phone(&phone).await.map_err(|e| e.to_string())
        })
    }

    fn search_users_by_email(
        &self,
        email: &str,
    ) -> BoxFutureVecValue {
        let client = self.clone();
        let email = email.to_string();
        Box::pin(async move {
            client.search_users_by_email(&email).await.map_err(|e| e.to_string())
        })
    }

    fn search_users_by_metadata(
        &self,
        key: &str,
        value: &str,
    ) -> BoxFutureVecValue {
        let client = self.clone();
        let key = key.to_string();
        let value = value.to_string();
        Box::pin(async move {
            client.search_users_by_metadata(&key, &value).await.map_err(|e| e.to_string())
        })
    }

    fn find_or_create_user_by_phone(
        &self,
        phone: &str,
        first_name: &str,
        last_name: &str,
    ) -> BoxFutureString {
        let client = self.clone();
        let phone = phone.to_string();
        let first_name = first_name.to_string();
        let last_name = last_name.to_string();
        Box::pin(async move {
            client.find_or_create_user_by_phone(&phone, &first_name, &last_name).await.map_err(|e| e.to_string())
        })
    }

    fn get_user_memberships(
        &self,
        user_id: &str,
        offset: i64,
        limit: i64,
    ) -> BoxFutureValue {
        let client = self.clone();
        let user_id = user_id.to_string();
        Box::pin(async move {
            client.get_user_memberships(&user_id, offset as u32, limit as u32).await.map_err(|e| e.to_string())
        })
    }

    fn remove_org_member(
        &self,
        org_id: &str,
        user_id: &str,
    ) -> BoxFutureUnit {
        let client = self.clone();
        let org_id = org_id.to_string();
        let user_id = user_id.to_string();
        Box::pin(async move {
            client.remove_org_member(&org_id, &user_id).await.map_err(|e| e.to_string())
        })
    }

    fn get_org_members(
        &self,
        org_id: &str,
    ) -> BoxFutureVecValue {
        let client = self.clone();
        let org_id = org_id.to_string();
        Box::pin(async move {
            client.get_org_members(&org_id).await.map_err(|e| e.to_string())
        })
    }
}
