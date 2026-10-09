pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use botcore::shared::utils::current_org_id;
pub(crate) use diesel::prelude::*;
pub(crate) use log::{info, trace, warn};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use sha2::{Digest, Sha256};
pub(crate) use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAPISpec {
    pub openapi: Option<String>,
    pub swagger: Option<String>,
    pub info: OpenAPIInfo,
    pub servers: Option<Vec<OpenAPIServer>>,
    pub paths: HashMap<String, HashMap<String, OpenAPIOperation>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAPIInfo {
    pub title: String,
    pub description: Option<String>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAPIServer {
    pub url: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAPIOperation {
    #[serde(rename = "operationId")]
    pub operation_id: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub parameters: Option<Vec<OpenAPIParameter>>,
    #[serde(rename = "requestBody")]
    pub request_body: Option<OpenAPIRequestBody>,
    pub responses: Option<HashMap<String, OpenAPIResponse>>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAPIParameter {
    pub name: String,
    #[serde(rename = "in")]
    pub location: String,
    pub description: Option<String>,
    pub required: Option<bool>,
    pub schema: Option<OpenAPISchema>,
    pub example: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAPISchema {
    #[serde(rename = "type")]
    pub schema_type: Option<String>,
    pub format: Option<String>,
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<String>>,
    pub default: Option<serde_json::Value>,
    pub example: Option<serde_json::Value>,
    pub properties: Option<HashMap<String, OpenAPISchema>>,
    pub required: Option<Vec<String>>,
    pub items: Option<Box<OpenAPISchema>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAPIRequestBody {
    pub description: Option<String>,
    pub required: Option<bool>,
    pub content: Option<HashMap<String, OpenAPIMediaType>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAPIMediaType {
    pub schema: Option<OpenAPISchema>,
    pub example: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAPIResponse {
    pub description: Option<String>,
    pub content: Option<HashMap<String, OpenAPIMediaType>>,
}

#[derive(Debug, Clone)]
pub struct GeneratedEndpoint {
    pub operation_id: String,
    pub method: String,
    pub path: String,
    pub description: String,
    pub parameters: Vec<EndpointParameter>,
    pub base_url: String,
}

#[derive(Debug, Clone)]
pub struct EndpointParameter {
    pub name: String,
    pub param_type: String,
    pub location: String,
    pub description: String,
    pub required: bool,
    pub example: Option<String>,
}

pub struct ApiToolGenerator {
    state: Arc<dyn BasicRuntime>,
    bot_id: Uuid,
    work_path: String,
}

#[derive(Debug, Default)]
pub struct SyncResult {
    pub apis_synced: usize,
    pub tools_generated: usize,
    pub tools_removed: usize,
    pub errors: Vec<String>,
}

use std::fmt::Write;
use std::sync::Arc;
use uuid::Uuid;
use botbasic_types::UserSession;
use rhai::Engine;

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

pub use part5::*;
