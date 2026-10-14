pub mod handlers;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthType {
    None,
    ApiKey,
    Bearer,
    Basic,
}

impl AuthType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ApiKey => "api_key",
            Self::Bearer => "bearer",
            Self::Basic => "basic",
        }
    }

    pub fn from_label(s: &str) -> Self {
        match s {
            "api_key" | "apikey" => Self::ApiKey,
            "bearer" => Self::Bearer,
            "basic" => Self::Basic,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionParam {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposedFunction {
    pub name: String,
    pub description: String,
    pub params: Vec<FunctionParam>,
    pub return_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub description: String,
    pub base_url: String,
    pub permissions: Vec<String>,
    pub auth_type: AuthType,
    pub auth_vault_path: Option<String>,
    pub functions: Vec<ExposedFunction>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStateEntry {
    pub manifest: PluginManifest,
    pub source_bucket: String,
    pub last_loaded: String,
}

impl PluginManifest {
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p == permission)
    }

    pub fn get_function(&self, name: &str) -> Option<&ExposedFunction> {
        self.functions.iter().find(|f| f.name == name)
    }
}

pub const DEFAULT_PLUGINS_PREFIX: &str = "plugins";

pub fn load_manifests_from_drive(
    bucket_name: &str,
    load_from_drive: &crate::LoadFromDriveFn,
) -> Result<Vec<PluginStateEntry>, String> {
    let prefix = format!("{}/", DEFAULT_PLUGINS_PREFIX);
    let mut plugins = Vec::new();

    let entries_result = load_from_drive(bucket_name, &prefix);
    let entries = match entries_result {
        Ok(e) => e,
        Err(e) => {
            log::warn!("No plugin directory found in bucket {}: {}", bucket_name, e);
            return Ok(Vec::new());
        }
    };

    for line in entries.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.ends_with(".json") {
            continue;
        }

        let manifest_path = format!("{}{}", prefix, trimmed);
        match load_from_drive(bucket_name, &manifest_path) {
            Ok(json_str) => {
                match parse_manifest(&json_str) {
                    Ok(manifest) => {
                        log::info!("Loaded plugin manifest: {} v{}", manifest.name, manifest.version);
                        plugins.push(PluginStateEntry {
                            manifest,
                            source_bucket: bucket_name.to_string(),
                            last_loaded: chrono::Utc::now().to_rfc3339(),
                        });
                    }
                    Err(e) => {
                        log::error!("Failed to parse plugin manifest {}: {}", manifest_path, e);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to load manifest file {}: {}", manifest_path, e);
            }
        }
    }

    Ok(plugins)
}

pub fn parse_manifest(json_str: &str) -> Result<PluginManifest, String> {
    let mut manifest: PluginManifest =
        serde_json::from_str(json_str).map_err(|e| format!("JSON parse error: {}", e))?;

    if manifest.name.is_empty() {
        return Err("Plugin name is required".to_string());
    }
    if manifest.version.is_empty() {
        return Err("Plugin version is required".to_string());
    }

    if manifest.id.is_nil() {
        manifest.id = Uuid::new_v4();
    }

    Ok(manifest)
}

pub fn get_active_plugins(plugins: &[PluginStateEntry]) -> Vec<&PluginStateEntry> {
    plugins.iter().filter(|p| p.manifest.enabled).collect()
}

pub fn get_plugins_by_permission<'a>(
    plugins: &'a [PluginStateEntry],
    permission: &str,
) -> Vec<&'a PluginStateEntry> {
    plugins
        .iter()
        .filter(|p| p.manifest.enabled && p.manifest.has_permission(permission))
        .collect()
}

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub use handlers::configure_plugin_routes;
