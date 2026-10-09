use super::*;

impl Default for ModelRouter {
    fn default() -> Self {
        Self {
            models: HashMap::new(),
            default_model: "default".to_string(),
            routing_strategy: RoutingStrategy::Manual,
        }
    }
}
impl ModelRouter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_config(config_models: &str, bot_id: Uuid, state: &Arc<dyn BasicRuntime>) -> Self {
        let mut router = Self::new();

        let model_names: Vec<&str> = config_models.split(';').collect();

        for name in model_names {
            let name = name.trim();
            if name.is_empty() {
                continue;
            }

            if let Ok(mut conn) = state.db_pool().get() {
                let model_config = load_model_config(&mut conn, bot_id, name);
                if let Some(config) = model_config {
                    router.models.insert(name.to_string(), config);
                }
            }
        }

        if let Some(first_name) = config_models.split(';').next() {
            router.default_model = first_name.trim().to_string();
        }

        router
    }

    pub fn get_model(&self, name: &str) -> Option<&ModelConfig> {
        self.models.get(name)
    }

    pub fn get_default(&self) -> Option<&ModelConfig> {
        self.models.get(&self.default_model)
    }

    pub fn route_query(&self, query: &str) -> &str {
        match self.routing_strategy {
            RoutingStrategy::Auto => self.auto_route(query),
            RoutingStrategy::LoadBalanced => self.load_balanced_route(),
            RoutingStrategy::Fallback | RoutingStrategy::Manual => &self.default_model,
        }
    }

    pub(super) fn auto_route(&self, query: &str) -> &str {
        let query_lower = query.to_lowercase();

        if (query_lower.contains("code")
            || query_lower.contains("program")
            || query_lower.contains("function")
            || query_lower.contains("debug")
            || query_lower.contains("error")
            || query_lower.contains("syntax"))
            && self.models.contains_key("code")
        {
            return "code";
        }

        if (query_lower.contains("analyze")
            || query_lower.contains("explain")
            || query_lower.contains("compare")
            || query_lower.contains("evaluate")
            || query.len() > 500)
            && self.models.contains_key("quality")
        {
            return "quality";
        }

        if (query.len() < 100
            || query_lower.contains("what is")
            || query_lower.contains("define")
            || query_lower.contains("hello"))
            && self.models.contains_key("fast")
        {
            return "fast";
        }

        &self.default_model
    }

    pub(super) fn load_balanced_route(&self) -> &str {
        &self.default_model
    }
}
pub(super) fn load_model_config(
    _conn: &mut diesel::PgConnection,
    bot_id: Uuid,
    model_name: &str,
) -> Option<ModelConfig> {
    let suffix = if model_name == "default" {
        "".to_string()
    } else {
        format!("-{}", model_name)
    };

    let model_key = format!("llm-model{}", suffix);
    let url_key = format!("llm-url{}", suffix);
    let key_key = format!("llm-key{}", suffix);

    let vault_configs = read_bot_config_from_vault(&bot_id)?;

    let model_path = vault_configs.get(&model_key).cloned().unwrap_or_default();
    let url = vault_configs.get(&url_key).cloned().unwrap_or_default();
    let api_key = vault_configs.get(&key_key).cloned().filter(|v| v != "none");

    if model_path.is_empty() && url.is_empty() {
        return None;
    }

    Some(ModelConfig {
        name: model_name.to_string(),
        url,
        model_path,
        api_key,
        max_tokens: None,
        temperature: None,
    })
}
pub fn register_model_routing_keywords(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) {
    use_model_keyword(Arc::clone(&state), user.clone(), engine);
    set_model_routing_keyword(Arc::clone(&state), user.clone(), engine);
    get_current_model_keyword(Arc::clone(&state), user.clone(), engine);
    list_models_keyword(state, user, engine);
}
pub fn use_model_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(["USE", "MODEL", "$expr$"], false, move |context, inputs| {
            let model_name = context
                .eval_expression_tree(&inputs[0])?
                .to_string()
                .trim_matches('"')
                .to_string();

            trace!("USE MODEL '{}' for session: {}", model_name, user_clone.id);

            let state_for_task = Arc::clone(&state_clone);
            let session_id = user_clone.id;
            let model_name_clone = model_name;

            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let _rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        let _ = tx.send(Err(format!("Failed to create runtime: {}", e)));
                        return;
                    }
                };
                let result = set_session_model(&state_for_task, session_id, &model_name_clone);
                let _ = tx.send(result);
            });

            match rx.recv_timeout(std::time::Duration::from_secs(10)) {
                Ok(Ok(msg)) => Ok(Dynamic::from(msg)),
                Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    e.into(),
                    rhai::Position::NONE,
                ))),
                Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    "USE MODEL timed out".into(),
                    rhai::Position::NONE,
                ))),
            }
        })
    {
        log::error!("Failed to register USE MODEL syntax failed: {e}");
    }
}
pub fn set_model_routing_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(
            ["SET", "MODEL", "ROUTING", "$expr$"],
            false,
            move |context, inputs| {
                let strategy_str = context
                    .eval_expression_tree(&inputs[0])?
                    .to_string()
                    .trim_matches('"')
                    .to_lowercase();

                let strategy = match strategy_str.as_str() {
                    "auto" => RoutingStrategy::Auto,
                    "load-balanced" | "loadbalanced" => RoutingStrategy::LoadBalanced,
                    "fallback" => RoutingStrategy::Fallback,
                    _ => RoutingStrategy::Manual,
                };

                trace!(
                    "SET MODEL ROUTING {:?} for session: {}",
                    strategy,
                    user_clone.id
                );

                let state_for_task = Arc::clone(&state_clone);
                let session_id = user_clone.id;
                let strategy_clone = strategy;

                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let result =
                        set_session_routing_strategy(&state_for_task, session_id, strategy_clone);
                    let _ = tx.send(result);
                });

                match rx.recv_timeout(std::time::Duration::from_secs(10)) {
                    Ok(Ok(msg)) => Ok(Dynamic::from(msg)),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        e.into(),
                        rhai::Position::NONE,
                    ))),
                    Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "SET MODEL ROUTING timed out".into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register SET MODEL ROUTING syntax failed: {e}");
    }
}
pub fn get_current_model_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone: Arc<dyn BasicRuntime> = Arc::clone(&state);
    let user_clone = user;

    engine.register_fn("GET CURRENT MODEL", move || -> String {
        let state = Arc::clone(&state_clone);

        if let Ok(mut conn) = state.db_pool().get() {
            get_session_model_sync(&mut conn, user_clone.id)
                .unwrap_or_else(|_| "default".to_string())
        } else {
            "default".to_string()
        }
    });
}
pub fn list_models_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone: Arc<dyn BasicRuntime> = Arc::clone(&state);
    let user_clone = user;

    engine.register_fn("LIST MODELS", move || -> rhai::Array {
        let state = Arc::clone(&state_clone);

        if let Ok(mut conn) = state.db_pool().get() {
            list_available_models_sync(&mut conn, user_clone.bot_id)
                .unwrap_or_default()
                .into_iter()
                .map(Dynamic::from)
                .collect()
        } else {
            rhai::Array::new()
        }
    });
}
