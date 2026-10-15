use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    None,
    ApiKey { header_name: String, key: String },
    BearerToken { token: String },
    BasicAuth { username: String, password: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposedFunction {
    pub name: String,
    pub description: Option<String>,
    pub method: String,
    pub path: String,
    pub request_body: Option<serde_json::Value>,
    pub parameters: Vec<FunctionParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionParameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub base_url: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub authentication: AuthType,
    pub functions: Vec<ExposedFunction>,
    pub bot_id: Option<String>,
}

impl PluginManifest {
    pub fn build_request(&self, fn_name: &str) -> Result<PluginRequest, PluginError> {
        let func = self.functions.iter().find(|f| f.name == fn_name)
            .ok_or_else(|| PluginError::FunctionNotFound(fn_name.to_string()))?;

        let url = format!("{}{}", self.base_url.trim_end_matches('/'), func.path);

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        match &self.authentication {
            AuthType::ApiKey { header_name, key } => {
                headers.insert(header_name.clone(), key.clone());
            }
            AuthType::BearerToken { token } => {
                headers.insert("Authorization".to_string(), format!("Bearer {}", token));
            }
            AuthType::BasicAuth { .. } => {}
            AuthType::None => {}
        }

        Ok(PluginRequest {
            url,
            method: func.method.clone(),
            headers,
            body: func.request_body.clone(),
            function_name: fn_name.to_string(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRequest {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<serde_json::Value>,
    pub function_name: String,
}

#[derive(Debug, Clone)]
pub enum PluginError {
    FunctionNotFound(String),
    HttpError(String),
    ParseError(String),
    AuthError(String),
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginError::FunctionNotFound(name) => write!(f, "Plugin function not found: {}", name),
            PluginError::HttpError(msg) => write!(f, "Plugin HTTP error: {}", msg),
            PluginError::ParseError(msg) => write!(f, "Plugin parse error: {}", msg),
            PluginError::AuthError(msg) => write!(f, "Plugin auth error: {}", msg),
        }
    }
}

impl std::error::Error for PluginError {}

pub struct PluginRegistry {
    plugins: RwLock<HashMap<String, PluginManifest>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self { plugins: RwLock::new(HashMap::new()) }
    }

    pub async fn register(&self, manifest: PluginManifest) {
        let mut plugins = self.plugins.write().await;
        plugins.insert(manifest.name.clone(), manifest);
    }

    pub async fn unregister(&self, name: &str) {
        let mut plugins = self.plugins.write().await;
        plugins.remove(name);
    }

    pub async fn get(&self, name: &str) -> Option<PluginManifest> {
        let plugins = self.plugins.read().await;
        plugins.get(name).cloned()
    }

    pub async fn list(&self) -> Vec<PluginManifest> {
        let plugins = self.plugins.read().await;
        plugins.values().cloned().collect()
    }

    pub async fn len(&self) -> usize {
        let plugins = self.plugins.read().await;
        plugins.len()
    }

    /// Returns `true` when no plugin manifest is registered.
    pub async fn is_empty(&self) -> bool {
        let plugins = self.plugins.read().await;
        plugins.is_empty()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedPluginRegistry = Arc<PluginRegistry>;

use std::sync::OnceLock;

static GLOBAL_PLUGIN_REGISTRY: OnceLock<SharedPluginRegistry> = OnceLock::new();

pub fn init_global_registry() -> SharedPluginRegistry {
    let registry = Arc::new(PluginRegistry::new());
    GLOBAL_PLUGIN_REGISTRY.set(registry.clone()).ok();
    registry
}

pub fn global_registry() -> Option<&'static SharedPluginRegistry> {
    GLOBAL_PLUGIN_REGISTRY.get()
}
