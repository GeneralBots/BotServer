use super::*;

pub async fn handle_list_repositories(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let repos: Vec<RepositoryInfo> = vec![RepositoryInfo {
        id: "1".to_string(),
        name: "generalbots".to_string(),
        owner: "generalbots".to_string(),
        description: "General Bots unified workspace".to_string(),
        url: "https://github.com/generalbots/generalbots".to_string(),
        language: Some("Rust".to_string()),
        stars: 150,
        forks: 45,
        status: "connected".to_string(),
        last_sync: Some("2024-01-15T10:30:00Z".to_string()),
    }];

    let mut html = String::new();
    html.push_str("<div class=\"repos-grid\">");

    for repo in &repos {
        let status_class = if repo.status == "connected" { "connected" } else { "disconnected" };
        let status_text = if repo.status == "connected" { "Connected" } else { "Disconnected" };
        let language = repo.language.as_deref().unwrap_or("Unknown");
        let last_sync = repo.last_sync.as_deref().unwrap_or("Never");

        let _ = write!(
            html,
            r#"<div class="repo-card">
            <div class="repo-header">
            <div class="repo-icon">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22"></path>
            </svg>
            </div>
            <div class="repo-info">
            <h4 class="repo-name">{}</h4>
            <span class="repo-owner">{}</span>
            </div>
            <span class="repo-status {}">{}</span>
            </div>
            <p class="repo-description">{}</p>
            <div class="repo-meta">
            <span class="repo-meta-item">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"></circle>
            </svg>
            {}</span>
            <span class="repo-meta-item">⭐ {}</span>
            <span class="repo-meta-item">🍴 {}</span>
            <span class="repo-meta-item">Last sync: {}</span>
            </div>
            <div class="repo-actions">
            <button class="btn-browse" onclick="window.open('{}', '_blank')">Browse</button>
            </div></div>"#,
            html_escape(&repo.name),
            html_escape(&repo.owner),
            status_class, status_text,
            html_escape(&repo.description),
            language, repo.stars, repo.forks, last_sync,
            html_escape(&repo.url)
        );
    }

    if repos.is_empty() {
        html.push_str(r#"<div class="empty-state">
        <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22"></path>
        </svg>
        <h3>No Repositories</h3><p>Connect your GitHub repositories to get started</p></div>"#);
    }

    html.push_str("</div>");
    Html(html)
}
pub async fn handle_connect_repository(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    Json(ApiResponse::success(format!("Repository {} connected", id)))
}
pub async fn handle_disconnect_repository(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    Json(ApiResponse::success(format!(
        "Repository {} disconnected", id
    )))
}
pub async fn handle_list_apps(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let apps: Vec<AppInfo> = vec![AppInfo {
        id: "1".to_string(),
        name: "crm".to_string(),
        app_type: "htmx".to_string(),
        description: "Customer relationship management".to_string(),
        url: "/crm".to_string(),
        created_at: "2024-01-10T14:00:00Z".to_string(),
        status: "active".to_string(),
    }];

    let mut html = String::new();
    html.push_str("<div class=\"apps-grid\">");

    for app in &apps {
        let app_icon = match app.app_type.as_str() {
            "htmx" => "📱",
            "react" => "⚛️",
            "vue" => "💚",
            _ => "🔷",
        };

        let _ = write!(
            html,
            r#"<div class="app-card">
            <div class="app-header">
            <div class="app-icon">{}</div>
            <div class="app-info">
            <h4 class="app-name">{}</h4>
            <span class="app-type">{}</span>
            </div></div>
            <p class="app-description">{}</p>
            <div class="app-actions">
            <button class="btn-open" onclick="window.location.href='{}'">Open</button>
            <button class="btn-edit">Edit</button>
            </div></div>"#,
            app_icon,
            html_escape(&app.name),
            html_escape(&app.app_type),
            html_escape(&app.description),
            html_escape(&app.url)
        );
    }

    if apps.is_empty() {
        html.push_str(r#"<div class="empty-state">
        <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
        <rect x="3" y="3" width="7" height="7"></rect>
        <rect x="14" y="3" width="7" height="7"></rect>
        <rect x="14" y="14" width="7" height="7"></rect>
        <rect x="3" y="14" width="7" height="7"></rect>
        </svg>
        <h3>No Apps</h3><p>Create your first app to get started</p></div>"#);
    }

    html.push_str("</div>");
    Html(html)
}
pub async fn handle_prompts(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let category = params.category.unwrap_or_else(|| "all".to_string());
    let prompts = get_prompts_data(&category);

    let mut html = String::new();
    html.push_str("<div class=\"panel-layout\">");
    html.push_str("<aside class=\"categories-sidebar\">");
    html.push_str("<h3>Categories</h3>");
    html.push_str("<div class=\"category-list\">");

    let categories = vec![
        ("all", "All Prompts", "📋"),
        ("writing", "Writing", "✍️"),
        ("coding", "Coding", "💻"),
        ("analysis", "Analysis", "📊"),
        ("creative", "Creative", "🎨"),
        ("business", "Business", "💼"),
        ("education", "Education", "📚"),
    ];

    for (id, name, icon) in &categories {
        let active = if *id == category { " active" } else { "" };
        let _ = write!(
            html,
            "<button class=\"category-item{}\" hx-get=\"/api/sources/prompts?category={}\" hx-target=\"#content-area\" hx-swap=\"innerHTML\"><span class=\"category-icon\">{}</span><span class=\"category-name\">{}</span></button>",
            active, id, icon, name
        );
    }

    html.push_str("</div></aside>");
    html.push_str("<div class=\"content-main\"><div class=\"prompts-grid\" id=\"prompts-grid\">");

    for prompt in &prompts {
        let _ = write!(
            html,
            "<div class=\"prompt-card\"><div class=\"prompt-header\"><span class=\"prompt-icon\">{}</span><h4>{}</h4></div><p class=\"prompt-description\">{}</p><div class=\"prompt-footer\"><span class=\"prompt-category\">{}</span><button class=\"btn-use\" onclick=\"usePrompt('{}')\">Use</button></div></div>",
            prompt.icon, html_escape(&prompt.title), html_escape(&prompt.description), html_escape(&prompt.category), html_escape(&prompt.id)
        );
    }

    if prompts.is_empty() {
        html.push_str("<div class=\"empty-state\"><p>No prompts found in this category</p></div>");
    }

    html.push_str("</div></div></div>");
    Html(html)
}
pub async fn handle_templates(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let templates = get_templates_data();

    let mut html = String::new();
    html.push_str("<div class=\"templates-container\">");
    html.push_str("<div class=\"templates-header\"><h3>Bot Templates</h3><p>Pre-built bot configurations ready to deploy</p></div>");
    html.push_str("<div class=\"templates-grid\">");

    for template in &templates {
        let _ = write!(
            html,
            "<div class=\"template-card\"><div class=\"template-icon\">{}</div><div class=\"template-info\"><h4>{}</h4><p>{}</p><div class=\"template-meta\"><span class=\"template-category\">{}</span></div></div><div class=\"template-actions\"><button class=\"btn-preview\">Preview</button><button class=\"btn-use-template\">Use Template</button></div></div>",
            template.icon, html_escape(&template.name), html_escape(&template.description), html_escape(&template.category)
        );
    }

    html.push_str("</div></div>");
    Html(html)
}
pub async fn handle_news(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let news_items = vec![
        ("📢", "General Bots 6.0 Released", "Major update with improved performance and new features", "2 hours ago"),
        ("🔌", "New MCP Server Integration", "Connect to external tools more easily with our new MCP support", "1 day ago"),
        ("📊", "Analytics Dashboard Update", "Real-time metrics and improved visualizations", "3 days ago"),
        ("🔒", "Security Enhancement", "Enhanced encryption and authentication options", "1 week ago"),
    ];

    let mut html = String::new();
    html.push_str("<div class=\"news-container\">");
    html.push_str("<div class=\"news-header\"><h3>Latest News</h3><p>Updates and announcements from the General Bots team</p></div>");
    html.push_str("<div class=\"news-list\">");

    for (icon, title, description, time) in &news_items {
        let _ = write!(
            html,
            "<div class=\"news-item\"><div class=\"news-icon\">{}</div><div class=\"news-content\"><h4>{}</h4><p>{}</p><span class=\"news-time\">{}</span></div></div>",
            icon, html_escape(title), html_escape(description), time
        );
    }

    html.push_str("</div></div>");
    Html(html)
}
pub async fn handle_llm_tools(
    State(state): State<Arc<AppState>>,
    Query(params): Query<BotQuery>,
) -> impl IntoResponse {
    let bot_id = params.bot_id.unwrap_or_else(|| "default".to_string());
    let work_path = get_work_path_or_default(&state.get_work_path);

    let keywords = get_keywords_or_default(&state.get_keywords);
    let loader = make_mcp_loader(&state.mcp_loader, &work_path, &bot_id);
    let scan_result = loader.load();
    let mcp_tools_count: usize = scan_result.servers.iter().map(|s| s.tools.len()).sum();

    let mut html = String::new();
    html.push_str("<div class=\"tools-container\">");
    let _ = write!(
        html,
        "<div class=\"tools-header\"><h3>LLM Tools</h3><p>All tools available for Tasks and LLM invocation</p><div class=\"tools-stats\"><span class=\"stat\"><strong>{}</strong> BASIC keywords</span><span class=\"stat\"><strong>{}</strong> MCP tools</span></div></div>",
        keywords.len(), mcp_tools_count
    );

    html.push_str("<div class=\"tools-grid\">");
    for keyword in keywords.iter().take(20) {
        let _ = write!(
            html,
            "<span class=\"keyword-tag\">{}</span>",
            html_escape(keyword)
        );
    }
    if keywords.len() > 20 {
        let _ = write!(
            html,
            "<span class=\"keyword-more\">+{} more...</span>",
            keywords.len() - 20
        );
    }
    html.push_str("</div></div>");

    Html(html)
}
