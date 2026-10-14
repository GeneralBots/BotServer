use std::sync::Arc;

use botcore::shared::state::AppState;
use uuid::Uuid;

use super::attachments::find_staged_attachment;
use super::sink::ChannelSink;

pub async fn run_tool_exec(
    state: &Arc<AppState>,
    bot_uuid: Uuid,
    session_id: Uuid,
    user_id: Uuid,
    bot_name: &str,
    tool_name: &str,
    channel: &str,
    sink: &dyn ChannelSink,
) {
    let work_path = botcore::shared::utils::get_work_path();
    let read_tool = |gbdialog_dir: &str| -> Option<String> {
        let ast_p = format!("{gbdialog_dir}/{tool_name}.ast");
        let bas_p = ast_p.replace(".ast", ".bas");
        std::fs::read_to_string(&ast_p).ok()
            .or_else(|| std::fs::read_to_string(&bas_p).ok())
    };

    let primary_rel = format!("{bot_name}.gborg/{bot_name}.gbai/{bot_name}.gbdialog/");
    let tool_content = if crate::core::bot::ws::handler::verify_path_within_workdir(&primary_rel) {
        let gbdialog_dir = format!("{work_path}/{bot_name}.gborg/{bot_name}.gbai/{bot_name}.gbdialog/");
        read_tool(&gbdialog_dir)
    } else {
        None
    };

    let ast_content = tool_content.unwrap_or_default();

    if !ast_content.is_empty() {
        // Declarative gate: only execute tools the bot script associated
        // with this session via USE TOOL (e.g. inside IF role = "admin").
        if !crate::core::bot::tool_context::is_tool_associated_with_session(
            &state.conn, &session_id, tool_name,
        ) {
            log::info!(
                "TOOL_EXEC: tool '{tool_name}' not associated with session {session_id}, skipping"
            );
            // The user pressed a button; silence would look like a broken bot.
            let message = format!(
                "A ação '{tool_name}' não está disponível para esta conversa."
            );
            let resp = botlib::models::BotResponse::new(
                bot_uuid.to_string(),
                session_id.to_string(),
                user_id.to_string(),
                &message,
                channel,
            );
            let _ = sink.send_bot_response(&resp).await;
            return;
        }
        let state_for_tool = state.clone();
        let tool_name_clone = tool_name.to_string();
        let channel_name = channel.to_string();
        let session_for_tool = botlib::models::UserSession {
            id: session_id, user_id, branch_id: Uuid::nil(), bot_id: bot_uuid,
            title: String::new(),
            context_data: serde_json::json!({"channel": channel_name}),
            current_tool: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let tool_run = tokio::task::spawn_blocking(move || {
            let mut svc = crate::basic::ScriptService::new(
                state_for_tool.clone(), session_for_tool,
            );
            svc.load_bot_config_params(&state_for_tool, bot_uuid);
            svc.run(&ast_content).map_err(|e| e.to_string())
        });
        // The script runs on a blocking task, so the report goes out after it
        // completes: a failure must reach the chat, not only the log.
        // Awaited, like the LLM tool-call path already does, so a failure can
        // be reported to the user instead of only reaching the log. The tool's
        // own TALK output reaches the sink while the script runs.
        if let Ok(Err(detail)) = tool_run.await {
            log::warn!("Tool '{tool_name_clone}' execution error: {detail}");
            let message = format!(
                "Não consegui executar '{tool_name_clone}'. Tente novamente em instantes."
            );
            let resp = botlib::models::BotResponse::new(
                bot_uuid.to_string(),
                session_id.to_string(),
                user_id.to_string(),
                &message,
                channel,
            );
            let _ = sink.send_bot_response(&resp).await;
        }
    }
}

pub fn is_placeholder_value(value: &str) -> bool {
    let lower = value.trim().to_lowercase();
    if lower.is_empty() { return true; }
    let placeholders = [
        "seu nome", "sua data", "seu telefone", "seu email",
        "nome completo", "nome da crianca", "nome do responsavel",
        "data preferencial", "data de nascimento", "endereco completo",
        "nao informado", "não informado", "nao informada",
        "coloque", "informe", "digite",
    ];
    for p in &placeholders {
        if lower.contains(p) { return true; }
    }
    if lower.starts_with("seu ") || lower.starts_with("sua ") { return true; }
    false
}

fn all_args_are_placeholder(args: &str) -> bool {
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(args) {
        if let Some(obj) = parsed.as_object() {
            if obj.is_empty() { return true; }
            let total = obj.len();
            let placeholder_count = obj.values().filter(|v| {
                match v {
                    serde_json::Value::String(s) => is_placeholder_value(s),
                    _ => true,
                }
            }).count();
            return placeholder_count > total / 2;
        }
    }
    false
}

pub fn is_generic_greeting(text: &str) -> bool {
    let trimmed = text.trim().to_lowercase();
    let greetings = [
        "ola", "oi", "hey", "hello", "hi", "bom dia", "boa tarde", "boa noite",
        "olá", "oie", "bem-vindo", "e ai", "e aí", "tudo bem", "td bem",
    ];
    greetings.contains(&trimmed.as_str())
}

pub async fn run_llm_tool_call(
    sink: &dyn ChannelSink,
    state: &Arc<AppState>,
    bot_uuid: Uuid,
    session_id: Uuid,
    user_id: Uuid,
    bot_name: &str,
    full_response: &str,
    rx: &mut tokio::sync::mpsc::Receiver<botlib::models::BotResponse>,
    user_text: &str,
) {
    use crate::core::bot::ws::handler::validate_bot_name;
    use crate::core::bot::ws::handler::verify_path_within_workdir;
    use botcore::shared::utils::get_work_path;
    let tool_call_trigger = "\"__tool_call__\":";
    let tc_start = match full_response.find(tool_call_trigger) {
        Some(pos) => full_response[..pos].rfind('{').unwrap_or(pos),
        None => return,
    };
    let tc_json = &full_response[tc_start..];
    if let Ok(tool_call) = serde_json::from_str::<serde_json::Value>(tc_json) {
        let raw_tool_name = tool_call.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let mut tool_args_owned = tool_call
            .get("arguments")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Chat attachments are staged under the bot's Drive `inbox/` before
        // the LLM call. Web attachments (pipeline::exec) append
        // "[User attached a file stored at inbox/…; …]" after the typed text;
        // channel adapters (bottelegram/botwhatsapp media) send
        // "[image] inbox/…" with the caption on the following line. Closed-
        // contract filing tools such as classify_media take `path`/`caption`;
        // when the model returns the tool call without carrying the staged
        // path, inject it from the user message so execution does not depend
        // on the model copying it verbatim.
        //
        // The staged path from the CURRENT message is authoritative: when the
        // model copies a path from an EARLIER turn's marker (history echo), the
        // tool operates on an already-filed object and fails with a confusing
        // 404. Only when the current message carries no attachment marker do
        // the model's arguments stand.
        if let Some(staged) = find_staged_attachment(user_text) {
            let mut parsed: serde_json::Value =
                serde_json::from_str(&tool_args_owned).unwrap_or_else(|_| serde_json::json!({}));
            match parsed.as_object_mut() {
                Some(obj) => {
                    let model_path = obj
                        .get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    let stale = !model_path.is_empty() && model_path != staged.path;
                    obj.insert("path".to_string(), serde_json::Value::String(staged.path.clone()));
                    obj.insert("caption".to_string(), serde_json::Value::String(staged.caption));
                    if stale {
                        log::info!(
                            "Tool '{raw_tool_name}': model echoed path '{model_path}' from history; overridden with current message attachment '{}'",
                            staged.path
                        );
                    }
                }
                None => {
                    log::warn!(
                        "Tool '{raw_tool_name}': arguments must be an object; staged attachment not injected"
                    );
                }
            }
            match serde_json::to_string(&parsed) {
                Ok(s) => tool_args_owned = s,
                Err(e) => log::warn!("Failed to serialize injected tool args: {e}"),
            }
        }
        let tool_args: &str = tool_args_owned.as_str();

        log::info!("LLM tool_call: executing tool '{raw_tool_name}' with args: {tool_args}");
        if raw_tool_name.is_empty() { return; }

        if all_args_are_placeholder(tool_args) {
            log::info!("All tool args are placeholder - asking user for real data instead of executing");
            let msg = "Para agendar o servico, preciso de algumas informacoes. Por favor, me diga os dados solicitados um de cada vez.".to_string();
            let resp = botlib::models::BotResponse::new(
                bot_uuid.to_string(), session_id.to_string(),
                user_id.to_string(), &msg, sink.channel_type(),
            );
            let _ = sink.send_bot_response(&resp).await;
            return;
        }

        if is_generic_greeting(user_text) {
            log::info!("Blocking tool '{raw_tool_name}' on short generic user message: '{user_text}'");
            return;
        }

        let tool_name = match validate_bot_name(&raw_tool_name) {
            Ok(n) => n,
            Err(e) => {
                log::warn!("LLM tool_call: invalid tool name '{raw_tool_name}': {e}");
                return;
            }
        };

        let work_path = get_work_path();
        let rel_tool_path = format!("{bot_name}.gborg/{bot_name}.gbai/{bot_name}.gbdialog/{tool_name}.ast");
        if !verify_path_within_workdir(&rel_tool_path) {
            log::error!("Path traversal detected in LLM tool_call for tool: {tool_name}");
            return;
        }

        let ast_path = format!("{work_path}/{bot_name}.gborg/{bot_name}.gbai/{bot_name}.gbdialog/{tool_name}.ast");
        let ast_content = match tokio::fs::read_to_string(&ast_path).await {
            Ok(c) if !c.is_empty() => c,
            _ => {
                let bas_path = ast_path.replace(".ast", ".bas");
                tokio::fs::read_to_string(&bas_path).await.unwrap_or_default()
            }
        };

        if ast_content.is_empty() {
            // A tool call naming a catalog command (`api.find`,
            // `tasks.autotask.create`, …) is the model reaching for the
            // declarative surface through the native tool channel. It has no
            // `.ast`, so this path used to return in silence and the user got
            // no answer at all — observed on Telegram: the model asked for
            // `api.find`, nothing came back. Reroute to the command executor.
            if let Some(command) = crate::apps::commands::command_by_name(&tool_name) {
                log::info!(
                    "tool_call '{tool_name}' is catalog command '{}' (app {}); routing to the command executor",
                    command.name, command.app
                );
                let api_call = format!(
                    "{{\"__api_call__\": {{\"name\": \"{tool_name}\", \"params\": {tool_args}, \"compose\": true}}}}"
                );
                // Composing the answer needs a provider AND the bot's model and
                // key: the reroute used to pass empty strings, so every composed
                // answer failed and the user got "Dados obtidos, mas falhei ao
                // redigir a resposta" instead of the command's result.
                let provider = state.llm_provider.clone();
                let provider = provider.as_ref();
                let (model, key) = bot_llm_credentials(&state.conn, bot_uuid);
                // Boxed: the command path can reroute a session tool back
                // into this executor, so the two futures are mutually
                // recursive.
                Box::pin(super::api_call::handle_api_call(
                    sink, state, provider, &model, &key, bot_uuid, session_id, user_id,
                    bot_name, &api_call, user_text, rx,
                ))
                .await;
                return;
            }
            // Unknown name with no script: say so instead of going silent.
            log::warn!(
                "tool_call '{tool_name}' has no compiled script and is not a catalog command; ignored"
            );
            let message = format!(
                "Não consegui executar '{tool_name}': essa ação não está disponível para mim agora."
            );
            let resp = botlib::models::BotResponse::new(
                bot_uuid.to_string(),
                session_id.to_string(),
                user_id.to_string(),
                &message,
                sink.channel_type(),
            );
            let _ = sink.send_bot_response(&resp).await;
            return;
        }

        {
            // Declarative gate: only execute tools the bot script associated
            // with this session via USE TOOL (e.g. inside IF role = "admin").
            if !crate::core::bot::tool_context::is_tool_associated_with_session(
                &state.conn, &session_id, &tool_name,
            ) {
                log::info!(
                    "LLM tool_call: tool '{tool_name}' not associated with session {session_id}, skipping"
                );
                return;
            }
            let state_for_tool = state.clone();
            let work_path_for_mcp = work_path.clone();
            let bot_name_for_mcp = bot_name.to_string();
            let tool_name_for_mcp = tool_name.clone();

            let parsed_args: serde_json::Value = serde_json::from_str(tool_args).unwrap_or(serde_json::Value::Null);
            let injected_args = parsed_args.clone();
            let channel_name = sink.channel_type();
            let mut context_data = if parsed_args.is_object() {
                parsed_args
            } else {
                serde_json::json!({})
            };
            if let Some(obj) = context_data.as_object_mut() {
                obj.insert("channel".to_string(), serde_json::Value::String(channel_name.to_string()));
            }

            let session_for_tool = botlib::models::UserSession {
                id: session_id, user_id, branch_id: Uuid::nil(), bot_id: bot_uuid,
                title: String::new(),
                context_data,
                current_tool: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            let tool_name_err = tool_name.clone();
            let tool_run = tokio::task::spawn_blocking(move || {
                let mut svc = crate::basic::ScriptService::new(
                    state_for_tool.clone(), session_for_tool,
                );
                svc.load_bot_config_params(&state_for_tool, bot_uuid);
                let mut injected_param_names: std::collections::HashSet<String> = std::collections::HashSet::new();
                if let Some(obj) = injected_args.as_object() {
                    for (key, value) in obj {
                        injected_param_names.insert(key.clone());
                        match value {
                            serde_json::Value::String(s) => {
                                let clean_val = if is_placeholder_value(s) { String::new() } else { s.clone() };
                                let _ = svc.set_variable(key, &clean_val);
                            }
                            serde_json::Value::Number(n) => {
                                let _ = svc.set_variable(key, &n.to_string());
                            }
                            serde_json::Value::Bool(b) => {
                                let _ = svc.set_variable(key, if *b { "true" } else { "false" });
                            }
                            _ => {}
                        }
                    }
                }
                let mcp_path = format!("{bot_name_for_mcp}.gborg/{bot_name_for_mcp}.gbai/{bot_name_for_mcp}.gbdialog/{tool_name_for_mcp}.mcp.json");
                let mcp_full = std::path::Path::new(&work_path_for_mcp).join(&mcp_path);
                if mcp_full.exists() {
                    if let Ok(mcp_content) = std::fs::read_to_string(&mcp_full) {
                        if let Ok(mcp_val) = serde_json::from_str::<serde_json::Value>(&mcp_content) {
                            if let Some(props) = mcp_val.get("input_schema").and_then(|s| s.get("properties")).and_then(|p| p.as_object()) {
                                for (param_name, _) in props {
                                    let clean_name = param_name.trim_end_matches(":string").to_string();
                                    if !injected_param_names.contains(&clean_name) {
                                        let _ = svc.set_variable(&clean_name, "");
                                    }
                                }
                            }
                        }
                    }
                }
                svc.run(&ast_content).map_err(|e| e.to_string())
            }).await;

            // A failed tool used to reach only the log, so the user watched the
            // turn stop with no answer at all (observed on the web tool
            // button: "Variable not found: path"). The detail stays in the
            // log; the user is told which action failed.
            if let Ok(Err(detail)) = tool_run {
                log::warn!("Tool '{tool_name_err}' execution error: {detail}");
                let message = format!(
                    "Não consegui executar '{tool_name_err}'. Tente novamente em instantes."
                );
                let resp = botlib::models::BotResponse::new(
                    bot_uuid.to_string(),
                    session_id.to_string(),
                    user_id.to_string(),
                    &message,
                    sink.channel_type(),
                );
                let _ = sink.send_bot_response(&resp).await;
            }

            // Drain rx to forward tool responses to the sink
            for _ in 0..50 {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                match rx.try_recv() {
                    Ok(response) => {
                        let _ = sink.send_bot_response(&response).await;
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => continue,
                    Err(_) => break,
                }
            }
        }
    }
}

/// Model and key configured for a bot, needed when a rerouted command composes
/// an answer. Mirrors the chat pipeline, including the environment fallback.
fn bot_llm_credentials(
    conn: &diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>,
    bot_id: Uuid,
) -> (String, String) {
    use botcore::config::ConfigManager;
    let cfg = ConfigManager::new(conn.clone());
    let mut key = cfg.get_config(&bot_id, "llm-key", Some("")).unwrap_or_default();
    let mut model = cfg.get_config(&bot_id, "llm-model", Some("")).unwrap_or_default();
    if let Ok(v) = std::env::var("LLM_KEY") {
        if !v.is_empty() { key = v; }
    }
    if let Ok(v) = std::env::var("LLM_MODEL") {
        if !v.is_empty() { model = v; }
    }
    if model.is_empty() {
        log::warn!("bot {bot_id} has no llm-model; composed answers are unavailable");
    }
    (model, key)
}
