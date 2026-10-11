use std::sync::Arc;

use botcore::shared::state::AppState;
use uuid::Uuid;

use super::sink::ChannelSink;
use super::types::PipelineResult;

/// Enriches the LLM prompt with KB context, memory recall and integration
/// mention blocks, saves the user turn, and streams the LLM response.
pub(super) async fn build_and_stream(
    state: &Arc<AppState>,
    sink: &dyn ChannelSink,
    rx: &mut tokio::sync::mpsc::Receiver<botlib::models::BotResponse>,
    bot_uuid: Uuid,
    session_id: Uuid,
    user_id: Uuid,
    bot_name: &str,
    messages: &[serde_json::Value],
    user_text: &str,
    #[cfg(feature = "integrations")] mentions: &[botlib::models::MentionRef],
    is_switcher_replay: bool,
) -> PipelineResult<()> {
    let user_query = user_text.to_string();
    let mut messages_val = serde_json::Value::Array(messages.to_vec());
    if tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::kb::inject_kb(
            state,
            bot_uuid,
            session_id,
            user_id,
            bot_name,
            &user_query,
            &mut messages_val,
        ),
    )
    .await
    .is_err()
    {
        log::warn!("ws_handler: inject_kb_context TIMEOUT after 30s for session {session_id}");
    }

    #[cfg(feature = "memory-os")]
    crate::core::bot::memory_hook::inject_recall(
        state,
        user_id,
        &user_query,
        &mut messages_val,
    );

    // Mention system blocks land right before the user turn so the LLM sees
    // the advertised integration surface adjacent to the request it enables.
    #[cfg(feature = "integrations")]
    if !mentions.is_empty() {
        super::mentions::append_integration_mention_blocks(
            state,
            bot_uuid,
            user_id,
            mentions,
            &mut messages_val,
        )
        .await;
    }

    if !is_switcher_replay {
        let mut sm = state.session_manager.lock().await;
        let _ = sm.save_message(session_id, user_id, 1, &user_text, 1);
    }

    if let Some(arr) = messages_val.as_array_mut() {
        arr.push(serde_json::json!({
            "role": "user",
            "content": user_text
        }));
    }

    if tokio::time::timeout(
        std::time::Duration::from_secs(300),
        super::llm::stream_llm_response(
            sink,
            rx,
            state,
            bot_uuid,
            session_id,
            user_id,
            bot_name,
            &messages_val,
            &user_query,
        ),
    )
    .await
    .is_err()
    {
        log::warn!("stream_llm_response TIMEOUT after 300s for session {session_id}");
    }

    Ok(())
}

/// Resolves the per-bot LLM provider (Vault config → env override), logging
/// the host/model pair. Returns None when llm-url is empty (global fallback).
pub(super) fn resolve_bot_llm_provider(
    state: &Arc<AppState>,
    bot_uuid: Uuid,
) -> Option<(Arc<dyn botlib::traits::LLMProvider>, String, String)> {
    let bot_llm_provider: Option<(Arc<dyn botlib::traits::LLMProvider>, String, String)> = {
        use botcore::config::ConfigManager;
        let cfg = ConfigManager::new(state.conn.clone());
        let mut llm_url = cfg.get_config(&bot_uuid, "llm-url", Some("")).unwrap_or_default();
        let mut llm_key = cfg.get_config(&bot_uuid, "llm-key", Some("")).unwrap_or_default();
        let mut llm_model = cfg.get_config(&bot_uuid, "llm-model", Some("")).unwrap_or_default();
        let llm_provider = cfg.get_config(&bot_uuid, "llm-provider", Some("")).unwrap_or_default();
        if let Ok(val) = std::env::var("LLM_URL") { if !val.is_empty() { llm_url = val; } }
        if let Ok(val) = std::env::var("LLM_KEY") { if !val.is_empty() { llm_key = val; } }
        if let Ok(val) = std::env::var("LLM_MODEL") { if !val.is_empty() { llm_model = val; } }
        let llm_host = url::Url::parse(&llm_url).ok()
            .and_then(|u| u.host_str().map(|h| h.to_string()))
            .unwrap_or_else(|| "<unparseable>".to_string());
        log::info!("BOT {bot_uuid} LLM CONFIG: host=[{llm_host}] key_len=[{}] model=[{llm_model}] provider=[{llm_provider}]", llm_key.len());
        if !llm_url.is_empty() {
            let explicit = if llm_provider.is_empty() { None } else { crate::llm::llm_provider_type_from_name(&llm_provider) };
            let provider = crate::llm::create_llm_provider_from_url(
                &llm_url,
                if llm_model.is_empty() { None } else { Some(llm_model.clone()) },
                None, explicit,
            );
            Some((
                Arc::new(crate::llm::BotlibLLMProviderWrapper::new(
                    provider, llm_model.clone(), llm_key.clone(),
                )) as Arc<dyn botlib::traits::LLMProvider>,
                llm_key, llm_model,
            ))
        } else {
            log::warn!("CRISTO BOT: llm-url is EMPTY, falling back to global provider");
            None
        }
    };

    bot_llm_provider
}

/// When the answer mode is Data/Chart, generates and sends the specialized
/// response; returns true when the pipeline has already replied.
pub(super) async fn respond_in_answer_mode(
    sink: &dyn ChannelSink,
    state: &Arc<AppState>,
    user_text: &str,
    bot_uuid: Uuid,
    bot_name: &str,
    session_id: Uuid,
    user_id: Uuid,
) -> PipelineResult<bool> {
    #[cfg(feature = "chat")]
    let answer_mode = crate::core::bot::answer_mode::get_answer_mode(state, &session_id).await;
    #[cfg(not(feature = "chat"))]
    let answer_mode = crate::core::bot::answer_mode::AnswerMode::Default;
    if answer_mode == crate::core::bot::answer_mode::AnswerMode::Default {
        return Ok(false);
    }
    let mode_response = match answer_mode {
        crate::core::bot::answer_mode::AnswerMode::Data => crate::core::bot::answer_mode::generate_data_response(
            state, user_text, bot_uuid, bot_name, session_id, user_id,
        ).await,
        crate::core::bot::answer_mode::AnswerMode::Chart => crate::core::bot::answer_mode::generate_chart_response(
            state, user_text, bot_uuid, bot_name, session_id, user_id,
        ).await,
        _ => return Ok(true),
    };
    match mode_response {
        Ok(resp) => {
            let _ = sink.send_bot_response(&resp).await;
            let mut sm = state.session_manager.lock().await;
            let _ = sm.save_message(session_id, user_id, 2, &resp.content, 2);
        }
        Err(e) => {
            let err_resp = botlib::models::BotResponse::new(
                bot_uuid.to_string(), session_id.to_string(),
                user_id.to_string(),
                format!("<p>Error: {e}</p>"), "web",
            );
            let _ = sink.send_bot_response(&err_resp).await;
        }
    }
    Ok(true)
}
