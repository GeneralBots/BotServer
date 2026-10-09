use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebApp {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub template: WebAppTemplate,
    pub status: WebAppStatus,
    pub config: WebAppConfig,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum WebAppTemplate {
    #[default]
    Blank,
    Landing,
    Dashboard,
    Form,
    Portal,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum WebAppStatus {
    #[default]
    Draft,
    Published,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebAppConfig {
    pub theme: String,
    pub layout: String,
    pub auth_required: bool,
    pub custom_domain: Option<String>,
    pub meta_tags: HashMap<String, String>,
    pub scripts: Vec<String>,
    pub styles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAppPage {
    pub id: Uuid,
    pub app_id: Uuid,
    pub path: String,
    pub title: String,
    pub content: String,
    pub layout: Option<String>,
    pub is_index: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAppComponent {
    pub id: Uuid,
    pub app_id: Uuid,
    pub name: String,
    pub component_type: ComponentType,
    pub props: serde_json::Value,
    pub children: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentType {
    Container,
    Text,
    Image,
    Button,
    Form,
    Input,
    Table,
    Chart,
    Custom(String),
}

pub struct WebaState {
    apps: RwLock<HashMap<Uuid, WebApp>>,
    pages: RwLock<HashMap<Uuid, WebAppPage>>,
    _components: RwLock<HashMap<Uuid, WebAppComponent>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAppRequest {
    pub name: String,
    pub description: Option<String>,
    pub template: Option<WebAppTemplate>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAppRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<WebAppStatus>,
    pub config: Option<WebAppConfig>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePageRequest {
    pub path: String,
    pub title: String,
    pub content: String,
    pub layout: Option<String>,
    pub is_index: bool,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub status: Option<String>,
}

#[cfg(test)]
mod tests;

mod lib_part1;
mod lib_part2;
mod lib_part3;

pub use lib_part2::*;
pub use lib_part3::*;
