use super::*;

#[cfg(feature = "vibe")]
pub(super) async fn create_site(
    config: botcore::config::AppConfig,
    #[cfg(feature = "drive")] _s3: Option<Arc<dyn botlib::traits::DriveRepository>>,
    #[cfg(not(feature = "drive"))] _s3: Option<()>,
    llm: Option<Arc<dyn botlib::traits::LLMProvider>>,
    _pool: botcore::shared::utils::DbPool,
    _bot_id: String,
    params: SiteCreationParams,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let alias_str = params.alias.to_string();
    let template_dir_str = params.template_dir.to_string();
    let prompt_str = params.prompt.to_string();

    info!("CREATE SITE (vibe): {} from template {}", alias_str, template_dir_str);

    let base_path = PathBuf::from(&config.site_path);
    let template_path = base_path.join(&template_dir_str);

    let combined_content = load_templates(&template_path)?;
    let generated_html = generate_html_from_prompt(llm, &combined_content, &prompt_str).await?;

    let (forgejo_url, alm_token, alm_org) = botcoresecrets::alm_config();
    let forgejo_token = alm_token;
    let org = if alm_org.is_empty() { "generalbots".to_string() } else { alm_org };

    let client = botdeployment::ForgejoClient::new(forgejo_url, forgejo_token);
    let repo = client.create_repository(&org, &alias_str, &prompt_str, false).await
        .map_err(|e| format!("Failed to create ALM repo: {e}"))?;

    let mut app = botdeployment::GeneratedApp::new(
        alias_str.clone(),
        prompt_str.clone(),
    );
    app.add_text_file("index.html".to_string(), generated_html);

    let schema = r#"{"tables": {}, "version": 1}"#;
    app.add_text_file("schema.json".to_string(), schema.to_string());

    client.push_app(&repo.clone_url, &app, "main").await
        .map_err(|e| format!("Failed to push to ALM: {e}"))?;

    let ci_cd = std::env::var("CI_CD_ENABLED")
        .unwrap_or_else(|_| "true".to_string());
    if ci_cd == "true" {
        client.create_cicd_workflow(
            &repo.clone_url,
            &botdeployment::ProjectType::Site,
            &botdeployment::DeployTarget::CaddyStatic,
            &botdeployment::DeploymentEnvironment::Production,
        ).await.map_err(|e| format!("Failed to create CI/CD: {e}"))?;
    }

    let domain = std::env::var("SITE_DOMAIN")
        .unwrap_or_else(|_| "gb.solutions".to_string());
    let url = format!("https://{alias_str}.{domain}/");

    info!("CREATE SITE (vibe): {} completed at {} (ALM: {})", alias_str, url, repo.html_url);
    Ok(url)
}
#[cfg(not(feature = "vibe"))]
#[cfg(feature = "drive")]
pub(super) async fn create_site(
    config: botcore::config::AppConfig,
    s3: Option<Arc<dyn botlib::traits::DriveRepository>>,
    llm: Option<Arc<dyn botlib::traits::LLMProvider>>,
    pool: botcore::shared::utils::DbPool,
    bot_id: String,
    params: SiteCreationParams,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let alias_str = params.alias.to_string();
    let template_dir_str = params.template_dir.to_string();
    let prompt_str = params.prompt.to_string();

    info!("CREATE SITE: {} from template {}", alias_str, template_dir_str);

    let base_path = PathBuf::from(&config.site_path);
    let template_path = base_path.join(&template_dir_str);

    let combined_content = load_templates(&template_path)?;
    let generated_html = generate_html_from_prompt(llm, &combined_content, &prompt_str).await?;

    let drive_path = format!("apps/{alias_str}");
    let location = resolve_site_location(&pool, &bot_id)?;
    store_to_drive(s3.as_ref(), &location, &drive_path, &generated_html).await?;

    let serve_path = base_path.join(&alias_str);
    sync_to_serve_path(&serve_path, &generated_html, &template_path)?;

    info!("CREATE SITE: {} completed, available at /apps/{}", alias_str, alias_str);
    Ok(format!("/apps/{alias_str}"))
}
#[cfg(not(feature = "vibe"))]
#[cfg(not(feature = "drive"))]
pub(super) async fn create_site(
    config: botcore::config::AppConfig,
    _s3: Option<()>,
    llm: Option<Arc<dyn botlib::traits::LLMProvider>>,
    _pool: botcore::shared::utils::DbPool,
    bot_id: String,
    params: SiteCreationParams,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let alias_str = params.alias.to_string();
    let template_dir_str = params.template_dir.to_string();
    let prompt_str = params.prompt.to_string();

    info!("CREATE SITE: {} from template {}", alias_str, template_dir_str);

    let base_path = PathBuf::from(&config.site_path);
    let template_path = base_path.join(&template_dir_str);

    let combined_content = load_templates(&template_path)?;
    let generated_html = generate_html_from_prompt(llm, &combined_content, &prompt_str).await?;

    let serve_path = base_path.join(&alias_str);
    sync_to_serve_path(&serve_path, &generated_html, &template_path)?;

    let _ = bot_id;
    info!("CREATE SITE: {} completed, available at /apps/{}", alias_str, alias_str);
    Ok(format!("/apps/{alias_str}"))
}
pub(super) fn load_templates(template_path: &std::path::Path) -> Result<String, Box<dyn Error + Send + Sync>> {
    let mut combined_content = String::new();

    if !template_path.exists() {
        return Err(format!("Template directory not found: {}", template_path.display()).into());
    }

    for entry in fs::read_dir(template_path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.extension().is_some_and(|ext| ext == "html") {
            let mut file = fs::File::open(&path).map_err(|e| e.to_string())?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .map_err(|e| e.to_string())?;

            use std::fmt::Write;
            let _ = writeln!(combined_content, "<!-- TEMPLATE: {} -->", path.display());
            combined_content.push_str(&contents);
            combined_content.push_str("\n\n--- TEMPLATE SEPARATOR ---\n\n");

            debug!("Loaded template: {}", path.display());
        }
    }

    if combined_content.is_empty() {
        return Err("No HTML templates found in template directory".into());
    }

    Ok(combined_content)
}
pub(super) async fn generate_html_from_prompt(
    llm: Option<Arc<dyn botlib::traits::LLMProvider>>,
    templates: &str,
    prompt: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let full_prompt = format!(
        r#"You are an expert HTML/HTMX developer. Generate a complete HTML application.

TEMPLATE FILES FOR REFERENCE:
{}

USER REQUEST:
{}

REQUIREMENTS:
1. Clone the template structure and styling
2. Use ONLY local _assets (htmx.min.js, app.js, styles.css) - NO external CDNs
3. Use HTMX for all data operations:
   - hx-get="/api/db/TABLE" for lists
   - hx-post="/api/db/TABLE" for create
   - hx-put="/api/db/TABLE/ID" for update
   - hx-delete="/api/db/TABLE/ID" for delete
4. Include search with hx-trigger="keyup changed delay:300ms"
5. Generate semantic, accessible HTML
6. App context is automatic - just use /api/db/* paths

OUTPUT: Complete index.html file only, no explanations."#,
        templates, prompt
    );

    let html = match llm {
        Some(provider) => {
            let messages = json!([{
                "role": "user",
                "content": full_prompt
            }]);

            match provider
                .generate(&full_prompt, &messages, "gpt-4o-mini", "")
                .await
            {
                Ok(response) => {
                    let cleaned = extract_html_from_response(&response);
                    if cleaned.contains("<html") || cleaned.contains("<!DOCTYPE") {
                        info!("LLM generated HTML ({} bytes)", cleaned.len());
                        cleaned
                    } else {
                        warn!("LLM response doesn't contain valid HTML, using placeholder");
                        generate_placeholder_html(prompt)
                    }
                }
                Err(e) => {
                    warn!("LLM generation failed: {}, using placeholder", e);
                    generate_placeholder_html(prompt)
                }
            }
        }
        None => {
            debug!("No LLM provider configured, using placeholder HTML");
            generate_placeholder_html(prompt)
        }
    };

    debug!("Generated HTML ({} bytes)", html.len());
    Ok(html)
}
pub(super) fn extract_html_from_response(response: &str) -> String {
    let trimmed = response.trim();

    if trimmed.starts_with("```html") {
        let without_prefix = trimmed.strip_prefix("```html").unwrap_or(trimmed);
        let without_suffix = without_prefix
            .trim()
            .strip_suffix("```")
            .unwrap_or(without_prefix);
        return without_suffix.trim().to_string();
    }

    if trimmed.starts_with("```") {
        let without_prefix = trimmed.strip_prefix("```").unwrap_or(trimmed);
        let without_suffix = without_prefix
            .trim()
            .strip_suffix("```")
            .unwrap_or(without_prefix);
        return without_suffix.trim().to_string();
    }

    trimmed.to_string()
}
pub(super) fn generate_placeholder_html(prompt: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>App</title>
<script src="_assets/htmx.min.js"></script>
<link rel="stylesheet" href="_assets/styles.css">
</head>
<body>
<header>
<h1>Generated App</h1>
<p>Prompt: {}</p>
</header>

<main>
<section>
<h2>Data</h2>
<div id="data-list"
 hx-get="/api/db/items"
 hx-trigger="load"
 hx-swap="innerHTML">
Loading...
</div>

<form hx-post="/api/db/items"
  hx-target="#data-list"
  hx-swap="afterbegin">
<input name="name" placeholder="Name" required>
<button type="submit">Add</button>
</form>
</section>
</main>

<script src="_assets/app.js"></script>
</body>
</html>"##,
        prompt
    )
}
