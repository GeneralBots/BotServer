use std::sync::Arc;
use axum::extract::ws::{Message, WebSocket};
use botcore::shared::state::AppState;
use futures_util::SinkExt;
use log::{error, info, warn};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::core::bot::ws::handler::validate_bot_name;
use crate::core::bot::ws::handler::verify_path_within_workdir;
use crate::core::bot::ws::message::load_bot_styles_css;

pub async fn process_llm_response(
    ws_sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    rx: &mut mpsc::Receiver<botlib::models::BotResponse>,
    state: &Arc<AppState>,
    bot_uuid: Uuid,
    session_id: Uuid,
    user_id: Uuid,
    bot_name: &str,
    messages: &serde_json::Value,
    user_text: &str,
) {
    let (stream_tx, mut stream_rx) = mpsc::channel::<String>(100);
    let mut full_response = String::new();

    let bot_llm_provider: Option<(Arc<dyn botlib::traits::LLMProvider>, String, String)> = {
        use botcore::config::ConfigManager;
        let cfg = ConfigManager::new(state.conn.clone());
        let mut llm_url = cfg.get_config(&bot_uuid, "llm-url", Some("")).unwrap_or_default();
        let mut llm_key = cfg.get_config(&bot_uuid, "llm-key", Some("")).unwrap_or_default();
        let mut llm_model = cfg.get_config(&bot_uuid, "llm-model", Some("")).unwrap_or_default();
        // Env var overrides for production (take priority over DB config)
        if let Ok(val) = std::env::var("LLM_URL") { if !val.is_empty() { llm_url = val; } }
        if let Ok(val) = std::env::var("LLM_KEY") { if !val.is_empty() { llm_key = val; } }
        if let Ok(val) = std::env::var("LLM_MODEL") { if !val.is_empty() { llm_model = val; } }
        if !llm_url.is_empty() {
            let provider = crate::llm::create_llm_provider_from_url(&llm_url, if llm_model.is_empty() { None } else { Some(llm_model.clone()) }, None, None);
            Some((Arc::new(crate::llm::BotlibLLMProviderWrapper::new(provider, llm_model.clone(), llm_key.clone())) as Arc<dyn botlib::traits::LLMProvider>, llm_key, llm_model))
        } else {
            None
        }
    };

    let answer_mode = {
        #[cfg(feature = "chat")]
        {
            crate::core::bot::answer_mode::get_answer_mode(
                state, &session_id,
            ).await
        }
        #[cfg(not(feature = "chat"))]
        {
            crate::core::bot::answer_mode::AnswerMode::Default
        }
    };
    if answer_mode != crate::core::bot::answer_mode::AnswerMode::Default {
        let mode_response = match answer_mode {
            crate::core::bot::answer_mode::AnswerMode::Data => {
                crate::core::bot::answer_mode::generate_data_response(
                    state, user_text, bot_uuid, bot_name, session_id, user_id,
                ).await
            }
            crate::core::bot::answer_mode::AnswerMode::Chart => {
                crate::core::bot::answer_mode::generate_chart_response(
                    state, user_text, bot_uuid, bot_name, session_id, user_id,
                ).await
            }
            _ => return,
        };
        match mode_response {
            Ok(resp) => {
                if let Ok(json) = serde_json::to_string(&resp) {
                    let _ = ws_sender.send(Message::Text(json)).await;
                }
                let mut sm = state.session_manager.lock().await;
                let _ = sm.save_message(session_id, user_id, 2, &resp.content, 2);
            }
            Err(e) => {
                let err_resp = serde_json::json!({
                    "bot_id": bot_uuid.to_string(),
                    "user_id": user_id.to_string(),
                    "session_id": session_id.to_string(),
                    "channel": "web",
                    "content": format!("<p>Error: {}</p>", e),
                    "message_type": 2, "is_complete": true,
                    "suggestions": [], "switchers": [],
                    "context_length": 0, "context_max_length": 0,
                });
                let _ = ws_sender.send(Message::Text(err_resp.to_string())).await;
            }
        }
        return;
    }

    let env_key = std::env::var("LLM_KEY").unwrap_or_default();
    let env_model = std::env::var("LLM_MODEL").unwrap_or_default();
    info!("stream: bot_llm_provider.is_some={}, fallback_key_len={}", bot_llm_provider.is_some(), env_key.len());
    match bot_llm_provider.or_else(|| state.llm_provider.clone().map(move |p| (p, env_key.clone(), env_model.clone()))) {
        Some((ref llm, ref llm_key, ref llm_model)) => {
            let state_clone = state.clone();
            let prompt_clone = user_text.to_string();
            let messages_clone = messages.clone();
            let llm = llm.clone();
            let llm_key_clone = llm_key.clone();
            let llm_model_clone = llm_model.clone();
            let bot_uuid_s = bot_uuid.to_string();
            let session_id_s = session_id.to_string();
            let bot_name_clone = bot_name.to_string();

            let style_css = load_bot_styles_css(&bot_name_clone);

            let session_tools = {
                let sid: Uuid = match session_id_s.parse() {
                    Ok(id) => id,
                    Err(_) => Uuid::new_v4(),
                };
                let pool = state_clone.conn.clone();
                let name = bot_name_clone.clone();
                let sid_for_block = sid;
                match tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    tokio::task::spawn_blocking(move || {
                        crate::core::bot::tool_context::get_session_tools(
                            &pool, &name, &sid_for_block,
                        )
                    }),
                ).await {
                    Ok(Ok(Ok(tools))) => {
                        info!("Loaded {} tools for LLM session {}", tools.len(), session_id_s);
                        tools
                    }
                    Ok(Ok(Err(e))) => {
                        warn!("get_session_tools error for {}: {}", session_id_s, e);
                        Vec::new()
                    }
                    Ok(Err(e)) => {
                        warn!("get_session_tools spawn_blocking join error for {}: {}", session_id_s, e);
                        Vec::new()
                    }
                    Err(_) => {
                        warn!("get_session_tools TIMEOUT after 10s for session {}", session_id_s);
                        Vec::new()
                    }
                }
            };

            let _stream_handle = tokio::spawn(async move {
                info!("LLM spawn task starting: model={}, key_len={}, key_first_20={}", llm_model_clone, llm_key_clone.len(), llm_key_clone.chars().take(20).collect::<String>());
                let tools_arg = if session_tools.is_empty() { None } else { Some(session_tools) };
                if let Err(e) = llm.generate_stream(&prompt_clone, &messages_clone, stream_tx, &llm_model_clone, &llm_key_clone, tools_arg.as_ref()).await {
                    error!("LLM stream error: {}", e);
                } else {
                    info!("LLM spawn task completed successfully");
                }
            });

            {
                let mut init_msg = serde_json::json!({
                    "bot_id": bot_uuid_s,
                    "user_id": user_id.to_string(),
                    "session_id": session_id_s,
                    "channel": "web",
                    "content": "",
                    "message_type": 2,
                    "is_complete": false,
                    "thinking": true,
                    "suggestions": [],
                    "switchers": [],
                    "context_length": 0,
                    "context_max_length": 0,
                });
                if !style_css.is_empty() {
                    init_msg["css"] = serde_json::Value::String(style_css.clone());
                } else {
                    info!("CSS NOT included - style_css was empty for bot {}", bot_name_clone);
                }
                let _ = ws_sender.send(Message::Text(init_msg.to_string())).await;
            }

            let mut reasoning_accumulated = String::new();

            let mut keepalive_interval = tokio::time::interval(std::time::Duration::from_millis(800));
            keepalive_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    chunk = stream_rx.recv() => {
                        match chunk {
                            Some(chunk) => {
                                if chunk.contains("\"__reasoning__\"") {
                                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&chunk) {
                                        if let Some(r) = val.get("__reasoning__").and_then(|v| v.as_str()) {
                                            reasoning_accumulated.push_str(r);
                                        }
                                    }
                                    continue;
                                }
                                if chunk.contains("\"__tool_call__\"") {
                                    full_response.push_str(&chunk);
                                    continue;
                                }
                                full_response.push_str(&chunk);
                                let chunk_resp = serde_json::json!({
                                    "bot_id": bot_uuid_s,
                                    "user_id": user_id.to_string(),
                                    "session_id": session_id_s,
                                    "channel": "web",
                                    "content": chunk,
                                    "message_type": 2,
                                    "is_complete": false,
                                    "suggestions": [],
                                    "switchers": [],
                                    "context_length": 0,
                                    "context_max_length": 0,
                                });
                                if ws_sender.send(Message::Text(chunk_resp.to_string())).await.is_err() {
                                    break;
                                }
                            }
                            None => break,
                        }
                    }
                    _ = keepalive_interval.tick() => {
                        let _ = ws_sender.send(Message::Text(serde_json::json!({
                            "bot_id": bot_uuid_s,
                            "user_id": user_id.to_string(),
                            "session_id": session_id_s,
                            "channel": "web",
                            "content": "",
                            "message_type": 2,
                            "is_complete": false,
                            "thinking": true,
                            "suggestions": [],
                            "switchers": [],
                            "context_length": 0,
                            "context_max_length": 0,
                        }).to_string())).await;
                    }
                }
            }

            let mut final_content = full_response.trim_end().to_string();
            if let Some(pos) = final_content.rfind("{\"__tool_call__\":") {
                final_content = final_content[..pos].trim_end().to_string();
            }

            let final_resp = serde_json::json!({
                "bot_id": bot_uuid_s,
                "user_id": user_id.to_string(),
                "session_id": session_id_s,
                "channel": "web",
                "content": final_content,
                "message_type": 2,
                "is_complete": true,
                "reasoning": reasoning_accumulated.trim(),
                "suggestions": [],
                "switchers": [],
                "context_length": 0,
                "context_max_length": 0,
            });
            if ws_sender.send(Message::Text(final_resp.to_string())).await.is_err() {
                let mut pending = state.pending_stream_responses.lock().await;
                pending.insert(session_id_s.clone(), final_content);
            }

            let tool_call_trigger = "\"__tool_call__\":".to_string();
            if full_response.contains(&tool_call_trigger) {
                if let Ok(tool_call) = serde_json::from_str::<serde_json::Value>(&full_response) {
                    let raw_tool_name = tool_call.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let tool_args = tool_call.get("arguments").and_then(|v| v.as_str()).unwrap_or("");
                    info!("LLM tool_call: executing tool '{}' with args: {}", raw_tool_name, tool_args);
                    if !raw_tool_name.is_empty() {
                        let tool_name = match validate_bot_name(&raw_tool_name) {
                            Ok(n) => n,
                            Err(e) => {
                                warn!("LLM tool_call: invalid tool name '{}': {}", raw_tool_name, e);
                                return;
                            }
                        };

                        let work_path = botcore::shared::utils::get_work_path();
                        let rel_tool_path = format!("{bot_name}.gborg/{bot_name}.gbai/{bot_name}.gbdialog/{tool_name}.ast");
                        if !verify_path_within_workdir(&rel_tool_path) {
                            error!("Path traversal detected in LLM tool_call for tool: {}", tool_name);
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

                        if !ast_content.is_empty() {
                            let state_for_tool = state.clone();
                            let tool_name_cl = tool_name.clone();
                            let work_path_for_mcp = work_path.clone();
                            let bot_name_for_mcp = bot_name.to_string();
                            let tool_name_for_mcp = tool_name.clone();

                            let parsed_args: serde_json::Value = serde_json::from_str(tool_args).unwrap_or(serde_json::Value::Null);
                            let injected_args = parsed_args.clone();
                            let context_data = if parsed_args.is_object() {
                                parsed_args
                            } else {
                                serde_json::Value::Null
                            };
                            info!("Tool '{}' context_data: {:?}", tool_name, context_data);

                            let session_for_tool = botlib::models::UserSession {
                                id: session_id, user_id, branch_id: Uuid::nil(), bot_id: bot_uuid,
                                title: String::new(),
                                context_data,
                                current_tool: None,
                                created_at: chrono::Utc::now(),
                                updated_at: chrono::Utc::now(),
                            };
                            let _ = tokio::task::spawn_blocking(move || {
                                let mut svc = crate::basic::ScriptService::new(
                                    state_for_tool.clone(), session_for_tool,
                                );
                                svc.load_bot_config_params(&state_for_tool, bot_uuid);
                                let mut injected_param_names: std::collections::HashSet<String> = std::collections::HashSet::new();
                                if let Some(obj) = injected_args.as_object() {
                                    for (key, value) in obj {
                                        let rhai_key = key.to_lowercase();
                                        injected_param_names.insert(rhai_key.clone());
                                        match value {
                                            serde_json::Value::String(s) => {
                                                let _ = svc.set_variable(&rhai_key, s);
                                            }
                                            serde_json::Value::Number(n) => {
                                                let _ = svc.set_variable(&rhai_key, &n.to_string());
                                            }
                                            serde_json::Value::Bool(b) => {
                                                let _ = svc.set_variable(&rhai_key, if *b { "true" } else { "false" });
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
                                                    let clean_name = param_name.trim_end_matches(":string").to_lowercase();
                                                    if !injected_param_names.contains(&clean_name) {
                                                        let _ = svc.set_variable(&clean_name, "");
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                if let Err(e) = svc.run(&ast_content) {
                                    warn!("Tool '{}' execution error: {}", tool_name_cl, e);
                                }
                            }).await;

                            for _ in 0..50 {
                                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                match rx.try_recv() {
                                    Ok(response) => {
                                        if let Ok(json) = serde_json::to_string(&response) {
                                            let _ = ws_sender.send(Message::Text(json)).await;
                                        }
                                    }
                                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => continue,
                                    Err(_) => break,
                                }
                            }
                        }
                    }
                }
            }

            {
                let mut sm = state.session_manager.lock().await;
                let _ = sm.save_message(session_id, user_id, 2, &full_response, 2);
            }
        }
        None => {
            info!("No LLM provider");
            let fallback = format!("Recebi: \"{}\"", user_text);
            {
                let mut sm = state.session_manager.lock().await;
                let _ = sm.save_message(session_id, user_id, 2, &fallback, 2);
            }
            let _ = ws_sender.send(Message::Text(serde_json::json!({
                "bot_id": bot_uuid.to_string(),
                "user_id": user_id.to_string(),
                "session_id": session_id.to_string(),
                "channel": "web",
                "content": fallback,
                "message_type": 2,
                "is_complete": true,
                "suggestions": [],
                "switchers": [],
                "context_length": 0,
                "context_max_length": 0,
            }).to_string())).await;
        }
    }
}
