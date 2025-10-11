use log::info;
use rhai::Dynamic;
use rhai::Engine;
use std::error::Error;
use std::fs;
use std::io::Read;
use std::path::PathBuf;

use crate::shared::models::UserSession;
use crate::shared::state::AppState;
use crate::shared::utils;

pub fn create_site_keyword(state: &AppState, _user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();
    engine
        .register_custom_syntax(
            &["CREATE_SITE", "$expr$", ",", "$expr$", ",", "$expr$"],
            true,
            move |context, inputs| {
                if inputs.len() < 3 {
                    return Err("Not enough arguments for CREATE SITE".into());
                }

                let alias = context.eval_expression_tree(&inputs[0])?;
                let template_dir = context.eval_expression_tree(&inputs[1])?;
                let prompt = context.eval_expression_tree(&inputs[2])?;

                let config = state_clone
                    .config
                    .as_ref()
                    .expect("Config must be initialized")
                    .clone();

                let fut = create_site(&config, alias, template_dir, prompt);
                let result =
                    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
                        .map_err(|e| format!("Site creation failed: {}", e))?;

                Ok(Dynamic::from(result))
            },
        )
        .unwrap();
}

async fn create_site(
    config: &crate::config::AppConfig,
    alias: Dynamic,
    template_dir: Dynamic,
    prompt: Dynamic,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let base_path = PathBuf::from(&config.site_path);
    let template_path = base_path.join(template_dir.to_string());
    let alias_path = base_path.join(alias.to_string());

    fs::create_dir_all(&alias_path).map_err(|e| e.to_string())?;

    let mut combined_content = String::new();

    for entry in fs::read_dir(&template_path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.extension().map_or(false, |ext| ext == "html") {
            let mut file = fs::File::open(&path).map_err(|e| e.to_string())?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .map_err(|e| e.to_string())?;

            combined_content.push_str(&contents);
            combined_content.push_str("\n\n--- TEMPLATE SEPARATOR ---\n\n");
        }
    }

    let full_prompt = format!(
        "TEMPLATE FILES:\n{}\n\nPROMPT: {}\n\nGenerate a new HTML file cloning all previous TEMPLATE (keeping only the local _assets libraries use, no external resources), but turning this into this prompt:",
        combined_content,
        prompt.to_string()
    );

    info!("Asking LLM to create site.");
    let llm_result = utils::call_llm(&full_prompt, &config.ai).await?;

    let index_path = alias_path.join("index.html");
    fs::write(index_path, llm_result).map_err(|e| e.to_string())?;

    info!("Site created at: {}", alias_path.display());
    Ok(alias_path.to_string_lossy().into_owned())
}
