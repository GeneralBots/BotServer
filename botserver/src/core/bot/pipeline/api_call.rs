use std::sync::Arc;
use botcore::shared::state::AppState;
use uuid::Uuid;

use super::sink::ChannelSink;

/// Collects every `app://...` deep-link string present anywhere in a command
/// result so the final answer can surface the most relevant one even when the
/// compose LLM call fails.
fn extract_deep_links(value: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![value.clone()];
    while let Some(v) = stack.pop() {
        match v {
            serde_json::Value::String(s) => {
                if s.starts_with("app://") {
                    out.push(s.clone());
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    stack.push(item.clone());
                }
            }
            serde_json::Value::Object(map) => {
                for val in map.values() {
                    stack.push(val.clone());
                }
            }
            _ => {}
        }
    }
    out
}

/// Injects a `value` param for tax commands when the model omitted it but the
/// user's message contains a monetary amount ("R$ 10.000", "10000 reais").
fn inject_tax_value_from_user(params: &serde_json::Value, user_text: &str) -> serde_json::Value {
    let has_value = params
        .get("value")
        .and_then(|v| v.as_str())
        .is_some_and(|v| !v.trim().is_empty());
    if has_value {
        return params.clone();
    }
    let amount = extract_money_amount(user_text);
    match amount {
        Some(value) => {
            let mut params = params.clone();
            if let serde_json::Value::Object(map) = &mut params {
                map.insert("value".to_string(), serde_json::Value::String(value));
            }
            params
        }
        None => params.clone(),
    }
}

/// Extracts a BRL amount from free text, e.g. "R$ 10.000,50" or "10.000 reais".
/// Returns a normalized decimal string ("10000.50").
fn extract_money_amount(text: &str) -> Option<String> {
    let re = regex::Regex::new(r"(?i)R\$\s*([\d.,]+)|\b([\d.,]+)\s*reais").ok()?;
    let caps = re.captures(text)?;
    let raw = caps.get(1).or_else(|| caps.get(2))?.as_str();
    let cleaned = raw.replace('.', "");
    let normalized = if cleaned.contains(',') {
        cleaned.replace(',', ".")
    } else {
        cleaned
    };
    if normalized.parse::<f64>().is_ok() {
        Some(normalized)
    } else {
        None
    }
}

/// Extracts the first `{"__api_call__": {...}}` object from a response,
/// returning its payload (the object's `__api_call__` value, or the whole
/// object when it is not nested).
fn extract_api_call_payload(full_response: &str) -> Option<serde_json::Value> {
    use crate::core::bot::api_catalog::find_api_call;
    let pos = find_api_call(full_response)?;
    let obj_start = full_response[..pos].rfind('{')?;
    let bytes = full_response.as_bytes();
    let mut depth = 0i32;
    let mut in_string = false;
    for (i, &b) in bytes[obj_start..].iter().enumerate() {
        match b {
            b'"' => in_string = !in_string,
            b'{' if !in_string => depth += 1,
            b'}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    let end = obj_start + i + 1;
                    let parsed: serde_json::Value = match serde_json::from_str(&full_response[obj_start..end]) {
                        Ok(v) => v,
                        Err(_) => return None,
                    };
                    let inner = parsed.get("__api_call__").or_else(|| parsed.get("api_call")).or_else(|| parsed.get("_api_call_")).cloned().unwrap_or(parsed);
                    return Some(inner);
                }
            }
            _ => {}
        }
    }
    None
}

/// Removes the `{"__api_call__": ...}` JSON block (the model's tool-call
/// preamble) from a response, leaving just the user-facing prose before and
/// after it. The block is matched with brace balancing so it is stripped in
/// full no matter which key variant the model emitted.
fn strip_api_call_block(text: &str) -> String {
    use crate::core::bot::api_catalog::find_api_call;
    let Some(pos) = find_api_call(text) else {
        return text.to_string();
    };
    let Some(obj_start) = text[..pos].rfind('{') else {
        return text.to_string();
    };
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut end = None;
    for (i, &b) in bytes[obj_start..].iter().enumerate() {
        match b {
            b'"' => in_string = !in_string,
            b'{' if !in_string => depth += 1,
            b'}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    end = Some(obj_start + i + 1);
                    break;
                }
            }
            _ => {}
        }
    }
    match end {
        Some(e) => {
            let mut out = String::with_capacity(text.len());
            out.push_str(&text[..obj_start]);
            out.push_str(&text[e..]);
            out
        }
        None => text.to_string(),
    }
}
/// Executes an `{"__api_call__": {"name", "params", "compose"}}` block found
/// in the LLM reply. Returns true when the call was handled (even on error),
/// so the caller skips the regular rendering path.
pub(super) async fn handle_api_call(
    sink: &dyn ChannelSink,
    state: &Arc<AppState>,
    llm: Option<&Arc<dyn botlib::traits::LLMProvider>>,
    model: &str,
    key: &str,
    bot_uuid: Uuid,
    session_id: Uuid,
    user_id: Uuid,
    bot_name: &str,
    full_response: &str,
    user_text: &str,
    rx: &mut tokio::sync::mpsc::Receiver<botlib::models::BotResponse>,
) -> bool {
    use crate::core::bot::api_catalog;
    let payload = match extract_api_call_payload(full_response) {
        Some(p) => p,
        None => return false,
    };
    let name = payload.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if name.is_empty() {
        return false;
    }
    let params = payload.get("params").cloned().unwrap_or(serde_json::Value::Null);
    let compose = payload.get("compose").and_then(|v| v.as_bool()).unwrap_or(false);
    let channel = sink.channel_type().to_string();

    // Session tools (USE TOOL / AutoTask, e.g. classify_media) are NOT part
    // of the declarative api-catalog; models sometimes emit them wrapped in
    // an __api_call__ block, which used to fail with "unknown command".
    // When the requested name belongs to the session's tool set, reroute to
    // the regular MCP tool-call executor with equivalent arguments.
    if crate::core::bot::tool_context::is_tool_associated_with_session(
        &state.conn, &session_id, &name,
    ) {
        log::info!("api_call '{name}' is a session tool; rerouting to tool executor");
        // run_llm_tool_call parses a FLAT payload: name + arguments-as-string
        // on the same level as the __tool_call__ marker.
        let args_str = serde_json::to_string(&params).unwrap_or_else(|_| "{}".to_string());
        let args_escaped = serde_json::to_string(&args_str).unwrap_or_else(|_| "\"{}\"".to_string());
        let synthetic = format!(
            "{{\"__tool_call__\": true, \"name\": \"{name}\", \"arguments\": {args_escaped}}}"
        );
        super::tool_exec::run_llm_tool_call(
            sink, state, bot_uuid, session_id, user_id, bot_name,
            &synthetic, rx, user_text,
        ).await;
        return true;
    }

    // The LLM may have spoken a short "working on it..." line before the
    // __api_call__ block. Surface it to the user so the tool call feels
    // natural, then run the command. The JSON block itself is always
    // stripped out of the leading text (never shown to the user).
    let stripped = strip_api_call_block(full_response);
    let leading = stripped.trim();
    if !leading.is_empty() {
        let progress = botlib::models::BotResponse::new(
            bot_uuid.to_string(), session_id.to_string(), user_id.to_string(),
            leading, &channel,
        );
        let _ = sink.send_bot_response(&progress).await;
    }

    // Tax commands: models frequently omit the `value` param even when the
    // user stated an amount. Recover it from the user's message (issue #722).
    let params = if matches!(name.as_str(), "service.tax" | "tax.calculate") {
        inject_tax_value_from_user(&params, user_text)
    } else {
        params
    };

    #[cfg(feature = "consent")]
    {
        let app_id = crate::apps::commands::command_by_name(&name)
            .map(|c| c.app.to_string())
            .unwrap_or_else(|| name.clone());
        let gate = crate::core::bot::consent_gate::check(
            user_id,
            &app_id,
            "update",
            serde_json::json!({ "command": name, "params": params }),
        )
        .await;
        if !gate.allowed {
            let message = match gate.pending_request {
                Some(ref request) => {
                    crate::core::bot::consent_gate::marker_content(request)
                }
                None => crate::core::bot::consent_gate::deny_message(&app_id),
            };
            let resp = botlib::models::BotResponse::new(
                bot_uuid.to_string(), session_id.to_string(),
                user_id.to_string(), &message, &channel,
            );
            let _ = sink.send_bot_response(&resp).await;
            return true;
        }
    }

    match api_catalog::execute_command(state, bot_uuid, user_id, &name, &params).await {
        Ok(result) => {
            if compose {
                let deep_links = extract_deep_links(&result);
                // Grounding: the model once answered "I created the automation"
                // after a discovery-only command returned nothing but a match
                // list, and nothing was ever created. Spell out what the data
                // does and does not support.
                let created_something = result
                    .get("created_resources")
                    .and_then(|c| c.as_array())
                    .is_some_and(|a| !a.is_empty());
                let performed_action =
                    result.get("success").and_then(|s| s.as_bool()).unwrap_or(false)
                        || created_something;
                let grounding = if performed_action {
                    "The data confirms the action was carried out: report it as done.\n"
                } else {
                    "The data does NOT show that any action was carried out — it is a lookup, a search \
                     or an empty result. Never tell the user that something was created, saved, sent or \
                     scheduled. If they asked for an action, say you could not perform it yet and what \
                     you would need.\n"
                };
                let prompt = format!(
                    "You are the assistant of bot '{bot_name}'. The user asked: \"{user_text}\".\n\
                     You ran the command '{name}' and received this data:\n{json}\n\n\
                     {grounding}\
                     Write a concise, friendly answer for the user in the language of the user's message, \
                     using the data. Never mention JSON, commands or internal details.\n\
                     {deep_link_instruction}",
                    json = serde_json::to_string_pretty(&result).unwrap_or_default(),
                    grounding = grounding,
                    deep_link_instruction = if deep_links.is_empty() {
                        String::new()
                    } else {
                        format!(
                            "The result references specific records. End your answer with a clickable \
                             link to the most relevant record, in the exact form provided (it starts \
                             with app://), e.g. '{first}'.",
                            first = deep_links[0],
                        )
                    },
                );
                // A rerouted tool call may arrive before a provider is resolved
                // for this bot; the command result is then reported raw.
                let composed = match llm {
                    Some(llm) => llm.generate(&prompt, &serde_json::json!({}), model, key).await.ok(),
                    None => None,
                };
                match composed {
                    Some(text) => {
                        let resp = botlib::models::BotResponse::new(
                            bot_uuid.to_string(), session_id.to_string(), user_id.to_string(),
                            &text, &channel,
                        );
                        let _ = sink.send_bot_response(&resp).await;
                    }
                    None => {
                        // Never lose the result to a composition failure: the
                        // command's own message is the most useful thing we
                        // have, and a bare apology told the user nothing.
                        let command_message = result
                            .get("message")
                            .and_then(|m| m.as_str())
                            .map(str::trim)
                            .filter(|m| !m.is_empty())
                            .map(str::to_string);
                        let fallback = match (command_message, deep_links.first()) {
                            (Some(message), _) => message,
                            (None, Some(link)) => format!(
                                "Here is the record you asked about: {link}"
                            ),
                            (None, None) => {
                                "Dados obtidos, mas falhei ao redigir a resposta.".to_string()
                            }
                        };
                        let resp = botlib::models::BotResponse::new(
                            bot_uuid.to_string(), session_id.to_string(), user_id.to_string(),
                            &fallback, &channel,
                        );
                        let _ = sink.send_bot_response(&resp).await;
                    }
                }
            } else {
                let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                // The result can contain any language: cut on a char boundary
                // rather than aborting the process mid-conversation.
                let text = botlib::text::truncate_with_suffix(&text, 2000, "...");
                let resp = botlib::models::BotResponse::new(
                    bot_uuid.to_string(), session_id.to_string(), user_id.to_string(),
                    &text, &channel,
                );
                let _ = sink.send_bot_response(&resp).await;
            }
            true
        }
        Err(e) => {
            log::warn!("api_call '{name}' failed: {e}");
            let resp = botlib::models::BotResponse::new(
                bot_uuid.to_string(), session_id.to_string(), user_id.to_string(),
                &format!("Falha ao executar {name}: {e}"), &channel,
            );
            let _ = sink.send_bot_response(&resp).await;
            true
        }
    }
}
