use super::*;

pub async fn handle_models(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Pull the bot's real LLM configuration instead of a hardcoded catalog.
    let config_manager = state.config_manager.clone();
    let default_bot_id = uuid::Uuid::nil();
    let configured_model = config_manager
        .get_config(&default_bot_id, "llm-model", None)
        .unwrap_or_default();
    let configured_provider = config_manager
        .get_config(&default_bot_id, "llm-provider", None)
        .unwrap_or_default();
    let llm_url = config_manager
        .get_config(&default_bot_id, "llm-url", None)
        .unwrap_or_default();

    // Known provider families the runtime can drive (see botllm::LLMProviderType).
    // `match_url` is a lowercase substring tested against the configured llm-url;
    // an empty string means the provider is never auto-detected from URL.
    let catalog: Vec<(&str, &str, &str, &str)> = vec![
        ("🧠", "OpenAI", "GPT, GPT-4o and compatible endpoints", "openai.com"),
        ("🔷", "Anthropic", "Claude 3 and later family", "anthropic.com"),
        ("🦙", "Meta Llama", "Open-weight Llama models (via OpenAI-compatible servers)", ""),
        ("⚡", "Groq", "Fast inference on open models", "groq.com"),
        ("⛰️", "Vertex / Gemini", "Google AI Studio and Vertex endpoints", "googleapis.com"),
        ("☁️", "Amazon Bedrock", "Foundation models via Bedrock", "bedrock"),
    ];

    let provider_lower = configured_provider.to_lowercase();
    let url_lower = llm_url.to_lowercase();

    let mut html = String::new();
    html.push_str("<div class=\"models-container\">");
    html.push_str("<div class=\"models-header\"><h3>AI Models</h3><p>Language models available to your bots</p></div>");

    if !configured_model.is_empty() {
        let _ = write!(
            html,
            "<div class=\"active-model-banner\"><strong>Configured model:</strong> {} <span class=\"model-status active\">Active</span></div>",
            html_escape(&configured_model)
        );
    }

    html.push_str("<div class=\"models-grid\">");

    for (icon, name, description, match_url) in &catalog {
        // The provider actually wired up in config.csv (llm-provider or the
        // llm-url host) is the live one; everything else is listed as available.
        let is_configured = (!provider_lower.is_empty() && name.to_lowercase().contains(&provider_lower))
            || (!match_url.is_empty() && !url_lower.is_empty() && url_lower.contains(match_url));
        let status_class = if is_configured { "model-active" } else { "model-available" };
        let status = if is_configured { "Active" } else { "Available" };
        let _ = write!(
            html,
            "<div class=\"model-card {}\"><div class=\"model-icon\">{}</div><div class=\"model-info\"><div class=\"model-header\"><h4>{}</h4><span class=\"model-provider\">{}</span></div><p>{}</p><div class=\"model-footer\"><span class=\"model-status\">{}</span></div></div></div>",
            status_class, icon, html_escape(name), html_escape(name), html_escape(description), status
        );
    }

    html.push_str("</div></div>");
    Html(html)
}
pub async fn handle_search(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let query = params.q.unwrap_or_default();

    if query.is_empty() {
        return Html("<div class=\"search-prompt\"><p>Enter a search term</p></div>".to_string());
    }

    let query_lower = query.to_lowercase();
    let prompts = get_prompts_data("all");
    let matching_prompts: Vec<_> = prompts
        .iter()
        .filter(|p| {
            p.title.to_lowercase().contains(&query_lower)
            || p.description.to_lowercase().contains(&query_lower)
        })
        .collect();

    let mut html = String::new();
    let _ = write!(html, "<div class=\"search-results\"><div class=\"search-header\"><h3>Search Results for \"{}\"</h3></div>", html_escape(&query));

    if matching_prompts.is_empty() {
        html.push_str("<div class=\"no-results\"><p>No results found</p></div>");
    } else {
        let _ = write!(
            html,
            "<div class=\"result-section\"><h4>Prompts ({})</h4><div class=\"results-grid\">",
            matching_prompts.len()
        );
        for prompt in matching_prompts {
            let _ = write!(
                html,
                "<div class=\"result-item\"><span class=\"result-icon\">{}</span><div class=\"result-info\"><strong>{}</strong><p>{}</p></div></div>",
                prompt.icon, html_escape(&prompt.title), html_escape(&prompt.description)
            );
        }
        html.push_str("</div></div>");
    }

    html.push_str("</div>");
    Html(html)
}
pub async fn handle_mentions_autocomplete(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let query = params.q.unwrap_or_default().to_lowercase();

    #[derive(serde::Serialize)]
    struct MentionItem {
        name: String,
        display: String,
        #[serde(rename = "type")]
        item_type: String,
        icon: String,
        description: String,
    }

    let mut mentions: Vec<MentionItem> = Vec::new();

    let repos = vec![
        ("botserver", "Main bot server", "repo"),
        ("botui", "User interface", "repo"),
        ("botbook", "Documentation", "repo"),
        ("botlib", "Core library", "repo"),
    ];

    for (name, desc, _) in repos {
        if query.is_empty() || name.contains(&query) {
            mentions.push(MentionItem {
                name: name.to_string(),
                display: format!("@{}", name),
                item_type: "repository".to_string(),
                icon: "📁".to_string(),
                description: desc.to_string(),
            });
        }
    }

    let apps = vec![
        ("crm", "Customer management app", "app"),
        ("dashboard", "Analytics dashboard", "app"),
    ];

    for (name, desc, _) in apps {
        if query.is_empty() || name.contains(&query) {
            mentions.push(MentionItem {
                name: name.to_string(),
                display: format!("@{}", name),
                item_type: "app".to_string(),
                icon: "📱".to_string(),
                description: desc.to_string(),
            });
        }
    }

    let bot_id = "default".to_string();
    let work_path = get_work_path_or_default(&state.get_work_path);
    let loader = make_mcp_loader(&state.mcp_loader, &work_path, &bot_id);
    let scan_result = loader.load();

    for server in scan_result.servers {
        if query.is_empty() || server.name.to_lowercase().contains(&query) {
            mentions.push(MentionItem {
                name: server.name.clone(),
                display: format!("@{}", server.name),
                item_type: "mcp".to_string(),
                icon: "🔌".to_string(),
                description: server.description,
            });
        }
    }

    mentions.truncate(10);
    Json(mentions)
}
