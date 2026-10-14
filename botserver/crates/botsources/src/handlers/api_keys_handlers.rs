use crate::state::AppState;
use crate::types::ApiKeyInfo;

use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse},
    Json,
};
use std::fmt::Write;
use std::sync::Arc;

pub async fn handle_list_api_keys(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let config_manager = state.config_manager.clone();
    let default_bot_id = uuid::Uuid::nil();

    let mut keys = Vec::new();

    let llm_key = config_manager.get_config(&default_bot_id, "llm-key", None).unwrap_or_default();
    if !llm_key.is_empty() {
        keys.push(ApiKeyInfo {
            id: "llm-key".to_string(),
            name: "LLM API Key".to_string(),
            provider: "OpenAI Compatible".to_string(),
            key_preview: format!("sk-...{}", &llm_key.chars().take(4).collect::<String>()),
            created_at: "2024-01-01".to_string(),
            last_used: None,
        });
    }

    let openai_key = config_manager.get_config(&default_bot_id, "openai-key", None).unwrap_or_default();
    if !openai_key.is_empty() {
        keys.push(ApiKeyInfo {
            id: "openai-key".to_string(),
            name: "OpenAI API Key".to_string(),
            provider: "OpenAI".to_string(),
            key_preview: format!("sk-...{}", &openai_key.chars().take(4).collect::<String>()),
            created_at: "2024-01-01".to_string(),
            last_used: None,
        });
    }

    let anthropic_key = config_manager.get_config(&default_bot_id, "anthropic-key", None).unwrap_or_default();
    if !anthropic_key.is_empty() {
        keys.push(ApiKeyInfo {
            id: "anthropic-key".to_string(),
            name: "Anthropic API Key".to_string(),
            provider: "Anthropic (Claude)".to_string(),
            key_preview: format!("sk-ant-...{}", &anthropic_key.chars().take(4).collect::<String>()),
            created_at: "2024-01-01".to_string(),
            last_used: None,
        });
    }

    let html = render_api_keys_html(&keys);
    Html(html)
}

fn render_api_keys_html(keys: &[ApiKeyInfo]) -> String {
    let mut html = String::new();
    html.push_str(r#"<div class="section-header">
    <h2>API Keys (BYOK)</h2>
    <button class="btn btn-primary" onclick="showAddKeyModal()">+ Add API Key</button>
    </div>
    <p style="margin-bottom: 24px; color: var(--text-secondary);">
    Manage your own LLM API keys for BYOK (Bring Your Own Key) mode. These keys are stored securely and used for AI features.
    </p>"#);

    if keys.is_empty() {
        html.push_str(r#"<div class="empty-state">
        <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"></path>
        </svg>
        <h3>No API Keys Configured</h3>
        <p>Add your own LLM API keys to enable BYOK mode</p>
        </div>"#);
    } else {
        html.push_str(r#"<div class="servers-grid">"#);
        for key in keys {
            let _ = write!(html, r#"
            <div class="server-card">
            <div class="server-header">
            <div class="server-icon">🔑</div>
            <div class="server-info">
            <div class="server-name">{}</div>
            <span class="server-type">{}</span>
            </div></div>
            <div class="server-description">{}</div>
            <div class="server-meta">
            <span class="server-meta-item">Created: {}</span>
            </div>
            <div class="server-actions" style="margin-top: 12px; display: flex; gap: 8px;">
            <button class="btn btn-sm btn-outline" onclick="deleteApiKey('{}')">Delete</button>
            </div></div>"#,
            key.name, key.provider, key.key_preview, key.created_at, key.id);
        }
        html.push_str("</div>");
    }

    html.push_str(r#"<script>
    function showAddKeyModal() {
    const modal = document.createElement('div');
    modal.className = 'modal-overlay';
    modal.innerHTML = `
    <div class="modal-content" style="background: white; padding: 24px; border-radius: 12px; max-width: 500px; margin: 100px auto;">
    <h3 style="margin-bottom: 16px;">Add API Key</h3>
    <div class="form-group" style="margin-bottom: 16px;">
    <label style="display: block; margin-bottom: 8px; font-weight: 500;">Name</label>
    <input type="text" id="keyName" placeholder="e.g., My OpenAI Key" style="width: 100%; padding: 10px; border: 1px solid #ddd; border-radius: 8px;">
    </div>
    <div class="form-group" style="margin-bottom: 16px;">
    <label style="display: block; margin-bottom: 8px; font-weight: 500;">Provider</label>
    <select id="keyProvider" style="width: 100%; padding: 10px; border: 1px solid #ddd; border-radius: 8px;">
    <option value="llm-key">OpenAI Compatible</option>
    <option value="openai-key">OpenAI</option>
    <option value="anthropic-key">Anthropic (Claude)</option>
    <option value="google-key">Google (Gemini)</option>
    <option value="azure-key">Azure OpenAI</option>
    </select>
    </div>
    <div class="form-group" style="margin-bottom: 16px;">
    <label style="display: block; margin-bottom: 8px; font-weight: 500;">API Key</label>
    <input type="password" id="keyValue" placeholder="sk-..." style="width: 100%; padding: 10px; border: 1px solid #ddd; border-radius: 8px;">
    </div>
    <div style="display: flex; gap: 12px;">
    <button class="btn btn-primary" onclick="saveApiKey()">Save</button>
    <button class="btn btn-outline" onclick="this.closest('.modal-overlay').remove()">Cancel</button>
    </div>
    </div>`;
    document.body.appendChild(modal);
    }

    async function saveApiKey() {
    const provider = document.getElementById('keyProvider').value;
    const keyValue = document.getElementById('keyValue').value;
    if (!keyValue) { alert('Please enter an API key'); return; }
    try {
    const response = await fetch('/api/ui/sources/api-keys', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ provider, key: keyValue }) });
    if (response.ok) { location.reload(); } else { alert('Failed to save API key'); }
    } catch (e) { alert('Error: ' + e.message); }
    }

    async function deleteApiKey(id) {
    if (confirm('Delete this API key?')) {
    try {
    const response = await fetch('/api/ui/sources/api-keys/' + id, { method: 'DELETE' });
    if (response.ok) { location.reload(); }
    } catch (e) { alert('Error: ' + e.message); }
    }
    }
    </script>"#);

    html
}

pub async fn handle_add_api_key(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let provider = payload.get("provider").and_then(|v| v.as_str()).unwrap_or("llm-key");
    let key = payload.get("key").and_then(|v| v.as_str()).unwrap_or("");

    if key.is_empty() {
        return Json(serde_json::json!({ "error": "Key is required" }));
    }

    let config_manager = state.config_manager.clone();
    let default_bot_id = uuid::Uuid::nil();

    if let Err(e) = config_manager.set_config(&default_bot_id, provider, key) {
        return Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e.to_string(), "botsources::handlers::api_keys_handlers", None) }));
    }

    Json(serde_json::json!({ "success": true }))
}

pub async fn handle_delete_api_key(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let config_manager = state.config_manager.clone();
    let default_bot_id = uuid::Uuid::nil();

    if let Err(e) = config_manager.set_config(&default_bot_id, &id, "") {
        return Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e.to_string(), "botsources::handlers::api_keys_handlers", None) }));
    }

    Json(serde_json::json!({ "success": true }))
}
