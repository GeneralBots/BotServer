use uuid::Uuid;

pub fn is_placeholder_value(value: &str) -> bool {
    let lower = value.trim().to_lowercase();
    if lower.is_empty() {
        return true;
    }
    let placeholders = [
        "seu nome",
        "sua data",
        "seu telefone",
        "seu email",
        "nome completo",
        "nome da crianca",
        "nome do responsavel",
        "data preferencial",
        "data de nascimento",
        "endereco completo",
        "nao informado",
        "não informado",
        "nao informada",
        "coloque",
        "informe",
        "digite",
    ];
    for p in &placeholders {
        if lower.contains(p) {
            return true;
        }
    }
    if lower.starts_with("seu ") || lower.starts_with("sua ") {
        return true;
    }
    false
}

pub(super) fn all_args_are_placeholder(args: &str) -> bool {
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(args) {
        if let Some(obj) = parsed.as_object() {
            if obj.is_empty() {
                return true;
            }
            let total = obj.len();
            let placeholder_count = obj
                .values()
                .filter(|v| match v {
                    serde_json::Value::String(s) => is_placeholder_value(s),
                    _ => true,
                })
                .count();
            return placeholder_count > total / 2;
        }
    }
    false
}

pub fn is_generic_greeting(text: &str) -> bool {
    let trimmed = text.trim().to_lowercase();
    let greetings = [
        "ola",
        "oi",
        "hey",
        "hello",
        "hi",
        "bom dia",
        "boa tarde",
        "boa noite",
        "olá",
        "oie",
        "bem-vindo",
        "e ai",
        "e aí",
        "tudo bem",
        "td bem",
    ];
    greetings.contains(&trimmed.as_str())
}

/// Model and key configured for a bot, needed when a rerouted command composes
/// an answer. Mirrors the chat pipeline, including the environment fallback.
pub(super) fn bot_llm_credentials(
    conn: &diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>,
    bot_id: Uuid,
) -> (String, String) {
    use botcore::config::ConfigManager;
    let cfg = ConfigManager::new(conn.clone());
    let mut key = cfg.get_config(&bot_id, "llm-key", Some("")).unwrap_or_default();
    let mut model = cfg.get_config(&bot_id, "llm-model", Some("")).unwrap_or_default();
    if let Ok(v) = std::env::var("LLM_KEY") {
        if !v.is_empty() {
            key = v;
        }
    }
    if let Ok(v) = std::env::var("LLM_MODEL") {
        if !v.is_empty() {
            model = v;
        }
    }
    if model.is_empty() {
        log::warn!("bot {bot_id} has no llm-model; composed answers are unavailable");
    }
    (model, key)
}
