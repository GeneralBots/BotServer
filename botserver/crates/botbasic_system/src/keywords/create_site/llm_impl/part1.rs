use super::*;

pub fn create_site_keyword(state: Arc<AppState>, user: UserSession, engine: &mut Engine) {
    let state_clone = state;
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(
            ["CREATE", "SITE", "$expr$", ",", "$expr$", ",", "$expr$"],
            true,
            move |context, inputs| {
                if inputs.len() < 3 {
                    return Err("Not enough arguments for CREATE SITE".into());
                }
                let alias = context.eval_expression_tree(&inputs[0])?;
                let template_dir = context.eval_expression_tree(&inputs[1])?;
                let prompt = context.eval_expression_tree(&inputs[2])?;

                let config = match state_clone.config.as_ref() {
                    Some(c) => <botcore::config::AppConfig as Clone>::clone(c),
                    None => {
                        return Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "Config must be initialized".into(),
                            rhai::Position::NONE,
                        )));
                    }
                };

                let bot_id = user_clone.bot_id.to_string();

                let params = SiteCreationParams {
                    alias,
                    template_dir,
                    prompt,
                };
                let state_for_thread = state_clone.clone();
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build();
                    let result = match rt {
                        Ok(rt) => rt.block_on(async {
                            create_site(
                                config,
                                state_for_thread.drive.clone(),
                                state_for_thread.llm_provider.clone(),
                                state_for_thread.conn.clone(),
                                bot_id,
                                params,
                            ).await
                        }),
                        Err(e) => Err(format!("Runtime creation failed: {e}").into()),
                    };
                    let _ = tx.send(result);
                });
                let result = rx.recv().unwrap_or(Err("Failed to receive result".into()))
                    .map_err(|e| format!("Site creation failed: {e}"))?;
                Ok(Dynamic::from(result))
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
