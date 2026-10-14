use std::sync::Arc;

use base64::Engine;
use botcore::shared::state::AppState;
use uuid::Uuid;

use super::sink::ChannelSink;
use super::types::PipelineResult;

pub type PipelineFn = std::sync::Arc<
    dyn Fn(
            botlib::models::UserMessage,
            tokio::sync::mpsc::Sender<botlib::models::BotResponse>,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = PipelineResult<()>> + Send>>
        + Send
        + Sync,
>;

pub async fn process_message_internal(
    sink: &dyn ChannelSink,
    rx: &mut tokio::sync::mpsc::Receiver<botlib::models::BotResponse>,
    state: &Arc<AppState>,
    session_id: Uuid,
    user_id: Uuid,
    bot_uuid: Uuid,
    bot_name: &str,
    start_bas_ran: &mut bool,
    text: &str,
) -> PipelineResult<()> {
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap_or_default();

    // Agent-mode switcher control frame (issue #1167). Consumed before the
    // regular pipeline so it never reaches the LLM; ignored when the
    // agent-vm feature is disabled at boot.
    #[cfg(feature = "agent-vm")]
    if parsed.get("type").and_then(|v| v.as_str()) == Some("agent_mode") {
        if let Some(status) = crate::core::bot::agent_vm_hook::handle_frame(
            &parsed, session_id, user_id, bot_uuid,
        )
        .await
        {
            let resp = botlib::models::BotResponse::new(
                bot_uuid.to_string(),
                session_id.to_string(),
                user_id.to_string(),
                &status,
                sink.channel_type(),
            );
            let _ = sink.send_bot_response(&resp).await;
            return Ok(());
        }
    }

    let mut user_text = parsed
        .get("text")
        .and_then(|v| v.as_str())
        .or_else(|| parsed.get("content").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();
    let mut msg_type = parsed
        .get("message_type")
        .and_then(|v| v.as_i64())
        .unwrap_or(1);
    let active_switchers: Vec<String> = parsed
        .get("active_switchers")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    // Project context is supplied by the chat mention picker as
    // {project_id, project_name}. It is kept separate from entity mentions so
    // an ordinary `@calculator` reference can target Vibe without changing
    // the existing CRM/integration mention contract.
    let project_context = parsed
        .get("project_context")
        .cloned()
        .or_else(|| {
            parsed.get("mentions").and_then(|value| {
                value.as_array()?.iter().find_map(|mention| {
                    if mention.get("kind").and_then(|v| v.as_str()) != Some("project") {
                        return None;
                    }
                    Some(serde_json::json!({
                        "project_id": mention.get("project_id").or_else(|| mention.get("id")),
                        "project_name": mention.get("label").or_else(|| mention.get("name")),
                    }))
                })
            })
        });
    // @-mentions selected in the composer (#939 phase D); resolved against
    // the connection control plane only when the integrations feature is
    // compiled in, exactly like the `integrations.invoke` command arm.
    #[cfg(feature = "integrations")]
    let mentions = super::mentions::parse_lenient_mentions(&parsed);

    {
        let mut sm = state.session_manager.lock().await;
        let _ = sm.get_or_create_session_by_id(session_id, user_id, bot_uuid, "");
    }

    if msg_type == 7 || msg_type == 0 {
        return Ok(());
    }

    let mut is_switcher_replay = false;
    if msg_type == 8 {
        let last_user_msg = {
            let mut sm = state.session_manager.lock().await;
            let history = sm
                .get_conversation_history(session_id, user_id, Some(1))
                .ok();
            history.and_then(|h| {
                h.into_iter()
                    .find(|(role, _)| role == "user")
                    .map(|(_, c)| c)
            })
        };
        if let Some(last_content) = last_user_msg {
            user_text = last_content;
            is_switcher_replay = true;
            msg_type = 1;
        } else {
            return Ok(());
        }
    }

    // Optional chat file attachment (base64) -> stored under inbox/ so the
    // catalog `drive.file` command can organize it into the right folder.
    if let Some(file) = parsed.get("file").and_then(|v| v.as_object()) {
        let fname = file
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let b64 = file
            .get("content_base64")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !fname.is_empty() && !b64.is_empty() {
            if let Some(drive) = state.drive.as_ref() {
                if let Ok(data) = base64::engine::general_purpose::STANDARD.decode(b64) {
                    // Same resolution as the channel stagers: an org-hosted bot
                    // lives under `{slug}.gborg`, so the bucket cannot be
                    // derived from the bot name.
                    let loc = {
                        match state.conn.get() {
                            Ok(mut conn) => {
                                botbasic_core::utils::bot_drive_location_for(&mut conn, bot_uuid)
                            }
                            Err(e) => {
                                log::error!("chat attachment: DB error: {e}");
                                botbasic_core::utils::resolve_bot_drive_location(
                                    bot_name,
                                    None,
                                    None,
                                )
                            }
                        }
                    };
                    let bucket = loc.bucket.clone();
                    let key = loc.key_for(&format!("inbox/{fname}"));
                    match drive.put_object(&bucket, &key, data, None).await {
                        Ok(()) => {
                            log::info!("stored chat attachment: {bucket}/{key}");
                            user_text = format!(
                                "{user_text}\n[User attached a file stored at inbox/{fname}; when a media filing tool such as classify_media is available for this session, call it with path=inbox/{fname} and the user's message as caption; otherwise use the drive.file command to organize it into its folder]"
                            );
                        }
                        Err(e) => log::error!("chat attachment store failed: {e}"),
                    }
                }
            }
        }
    }

    if user_text.starts_with("__TOOL__:") {
        let tool_name = user_text.trim_start_matches("__TOOL__:").trim().to_string();
        if !tool_name.is_empty() {
            let resp = botlib::models::BotResponse::new(
                bot_uuid.to_string(),
                session_id.to_string(),
                user_id.to_string(),
                format!("Tool '{tool_name}' not implemented via legacy path"),
                "web",
            );
            let _ = sink.send_bot_response(&resp).await;
        }
        return Ok(());
    }

    if msg_type == 6 {
        let raw_tool_name = user_text.trim().to_string();
        let tool_name = match crate::core::bot::ws::handler::validate_bot_name(&raw_tool_name) {
            Ok(n) => n,
            Err(e) => {
                log::warn!("TOOL_EXEC: invalid tool name '{}': {}", raw_tool_name, e);
                let resp = botlib::models::BotResponse::new(
                    bot_uuid.to_string(),
                    session_id.to_string(),
                    user_id.to_string(),
                    format!("<p>Invalid tool name: {raw_tool_name}</p>"),
                    "web",
                );
                let _ = sink.send_bot_response(&resp).await;
                return Ok(());
            }
        };

        if !tool_name.is_empty() {
            super::tool_exec::run_tool_exec(
                state,
                bot_uuid,
                session_id,
                user_id,
                bot_name,
                &tool_name,
                sink.channel_type(),
                sink,
            )
            .await;
        }
        return Ok(());
    }

    let runtime: Arc<dyn botbasic_types::BasicRuntime> =
        Arc::new(crate::basic::AppStateBasicRuntime(state.clone()));
    let delivered = crate::basic::keywords::hearing::deliver_hear_input(
        &runtime,
        session_id,
        user_text.clone(),
    );
    if delivered {
        return Ok(());
    }

    if !*start_bas_ran {
        let guards = state.start_bas_guards.lock().await;
        if !guards.contains_key(&session_id) {
            drop(guards);
            *start_bas_ran = super::start_bas::run_start_bas(
                sink, state, bot_uuid, session_id, user_id, bot_name, rx,
            )
            .await
            .unwrap_or(false);
        }
    }
    let mut guards = state.start_bas_guards.lock().await;
    guards.entry(session_id).or_insert(true);

    let post_start_suggestions = {
        #[cfg(feature = "chat")]
        {
            crate::basic::keywords::add_suggestion::get_suggestions(
                state.cache.as_ref(),
                &bot_uuid.to_string(),
                &session_id.to_string(),
            )
        }
        #[cfg(not(feature = "chat"))]
        Vec::new()
    };
    let post_start_switchers = {
        #[cfg(feature = "chat")]
        {
            crate::basic::keywords::switcher::get_switchers(
                state.cache.as_ref(),
                &bot_uuid.to_string(),
                &session_id.to_string(),
            )
        }
        #[cfg(not(feature = "chat"))]
        Vec::new()
    };
    if !post_start_suggestions.is_empty() || !post_start_switchers.is_empty() {
        let resp = botlib::models::BotResponse {
            bot_id: bot_uuid.to_string(),
            user_id: user_id.to_string(),
            session_id: session_id.to_string(),
            channel: "web".to_string(),
            content: String::new(),
            message_type: botlib::message_types::MessageType::BOT_RESPONSE,
            stream_token: None,
            is_complete: true,
            suggestions: post_start_suggestions
                .into_iter()
                .map(|s| botlib::models::Suggestion::new(s.text))
                .collect(),
            switchers: post_start_switchers,
            context_name: None,
            context_length: 0,
            context_max_length: 0,
            reasoning: String::new(),
        };
        let _ = sink.send_bot_response(&resp).await;
    }

    let _ = sink
        .send_bot_response(&botlib::models::BotResponse {
            bot_id: bot_uuid.to_string(),
            user_id: user_id.to_string(),
            session_id: session_id.to_string(),
            channel: "web".to_string(),
            content: String::new(),
            message_type: botlib::message_types::MessageType::BOT_RESPONSE,
            stream_token: None,
            is_complete: false,
            suggestions: Vec::new(),
            switchers: Vec::new(),
            context_name: None,
            context_length: 0,
            context_max_length: 0,
            reasoning: String::new(),
        })
        .await;

    let channel = sink.channel_type();
    let base_system_prompt =
        crate::core::bot::ws::message::load_system_prompt_for_channel(bot_name, channel);
    let system_prompt = if !active_switchers.is_empty() {
        let switcher_prompts = crate::basic::keywords::switcher::resolve_active_switchers(
            state.cache.as_ref(),
            &bot_uuid.to_string(),
            &session_id.to_string(),
            &active_switchers,
        );
        if switcher_prompts.is_empty() {
            base_system_prompt
        } else {
            format!("{base_system_prompt}\n\n{switcher_prompts}")
        }
    } else {
        base_system_prompt
    };
    let role = crate::security::user_role::resolve_user_role(&state.conn, user_id);
    let system_prompt = if channel == "web" {
        format!(
            "{system_prompt}\n\n{}\n\n{}",
            crate::core::bot::api_catalog::api_command_instructions(&role),
            crate::main_module::ui_plan::ui_automation_instructions(),
        )
    } else if channel == "whatsapp" {
        format!(
            "{system_prompt}\n\n---\nThis conversation is on WhatsApp.\n\n{}",
            crate::core::bot::api_catalog::api_command_instructions(&role),
        )
    } else {
        format!(
            "{system_prompt}\n\n{}",
            crate::core::bot::api_catalog::api_command_instructions(&role),
        )
    };

    let session_context = {
        let sm = state.session_manager.lock().await;
        sm.get_session_context_data(&session_id, &user_id)
            .ok()
            .unwrap_or_default()
    };

    let mut messages =
        vec![serde_json::json!({"role": "system", "content": system_prompt.clone()})];

    if !session_context.is_empty() {
        messages.push(serde_json::json!({
            "role": "system",
            "content": format!("Contexto da conversa:\n{session_context}")
        }));
    }

    if super::vibe_route::route_vibe_project(
        state, sink, project_context.as_ref(), &user_text,
        bot_uuid, session_id, user_id,
    ).await? {
        return Ok(());
    }
    if let Some(context) = project_context.as_ref() {
        let project_id = context.get("project_id").and_then(|v| v.as_str()).unwrap_or("");
        let project_name = context.get("project_name").and_then(|v| v.as_str()).unwrap_or("");
        if !project_id.is_empty() || !project_name.is_empty() {
            messages.push(serde_json::json!({
                "role": "system",
                "content": format!(
                    "The user referenced Vibe project '{project_name}' (id: {project_id}).\n\
                     For code changes to this project, use the vibe.project.change command with \
                     project_id='{project_id}', project_name='{project_name}', and the user's requested intent."
                ),
            }));
        }
    }

    super::history::append_conversation_history(state, bot_uuid, session_id, user_id, &mut messages).await;

    super::history::send_stream_start_marker(sink, bot_uuid, session_id, user_id).await;

    if tokio::time::timeout(
        std::time::Duration::from_secs(300),
        super::enriched_llm::build_and_stream(
            state, sink, rx, bot_uuid, session_id, user_id, bot_name,
            &messages, &user_text,
            #[cfg(feature = "integrations")]
            &mentions,
            is_switcher_replay,
        ),
    )
    .await
    .is_err()
    {
        log::warn!("stream_llm_response TIMEOUT after 300s for session {session_id}");
    }

    Ok(())
}
