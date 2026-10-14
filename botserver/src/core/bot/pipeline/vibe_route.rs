use std::sync::Arc;

use botcore::shared::state::AppState;
use uuid::Uuid;

use super::sink::ChannelSink;
use super::types::PipelineResult;

/// #1298 — a chat message that explicitly references a Vibe project (via the
/// mention picker or a typed `@project-name`) is a deterministic vibe agent
/// request: route it straight to the vibe run pipeline instead of relying on
/// the LLM to emit an `__api_call__` block. Returns `Ok(true)` when the
/// message was fully handled and the caller must stop.
pub(super) async fn route_vibe_project(
    state: &Arc<AppState>,
    sink: &dyn ChannelSink,
    project_context: Option<&serde_json::Value>,
    user_text: &str,
    bot_uuid: Uuid,
    session_id: Uuid,
    user_id: Uuid,
) -> PipelineResult<bool> {
    let project_context = project_context.cloned();
    // #1298 — a chat message that explicitly references a Vibe project via the
    // mention picker (`@project` / the vibe Chat toolbar) is a deterministic
    // vibe agent request, NOT a general chat question. Route it straight to the
    // vibe run pipeline instead of relying on the LLM to emit an `__api_call__`
    // block with `vibe.project.change` (smaller models like gpt-oss skip it and
    // answer with a generic clarification, so "@app paint it red" never reaches
    // the agent). The run executor is the same one `vibe.project.change` uses.
    let picked_project_id = project_context.as_ref().and_then(|ctx| {
        ctx.get("project_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    });
    let (routed_project_id, routed_project_name) = if let Some(pid) = picked_project_id {
        let name = project_context
            .as_ref()
            .and_then(|c| c.get("project_name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        (Some(pid), name)
    } else {
        // #mention — a typed "@project-name" in the raw text is deterministic
        // too: resolve it by name instead of trusting the LLM, which may route
        // the message to ui_plan automation (app 'designer') instead of the
        // vibe agent.
        match crate::core::bot::api_catalog::mentioned_project(state, &bot_uuid, user_text) {
            Ok(Some((pid, name))) => (Some(pid), name),
            _ => (None, String::new()),
        }
    };
    if let Some(pid) = routed_project_id {
        let project_name = routed_project_name;
        let mut run_params = serde_json::Map::new();
        run_params.insert("intent".to_string(), serde_json::Value::String(user_text.to_string()));
        run_params.insert("project_id".to_string(), serde_json::Value::String(pid.clone()));
        if !project_name.is_empty() {
            run_params.insert("project_name".to_string(), serde_json::Value::String(project_name.clone()));
        }
        run_params.insert(
            "use_case".to_string(),
            serde_json::Value::String("software_development".to_string()),
        );
        let result = crate::core::bot::api_exec::exec_endpoint(
            state,
            user_id,
            Some(bot_uuid),
            "POST",
            "/api/vibe/run",
            &run_params,
        )
        .await;
        match result {
            Ok(value) => {
                // #1286 — a lock conflict (another session is modifying the
                // same project) must be reported honestly, never as success:
                // the API returns success=false + lock_conflict=true.
                if value.get("lock_conflict").and_then(|v| v.as_bool()).unwrap_or(false) {
                    let detail = value
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("another session is modifying this project");
                    let resp = botlib::models::BotResponse::new(
                        bot_uuid.to_string(),
                        session_id.to_string(),
                        user_id.to_string(),
                        format!("⏳ {detail} — your change is queued and will run automatically."),
                        "web",
                    );
                    let _ = sink.send_bot_response(&resp).await;
                    return Ok(true);
                }
                // Surface a concise confirmation; the run itself streams its
                // progress through the vibe websocket/Run Dock, not the chat.
                let run_id = value.get("run_id").and_then(|v| v.as_str()).unwrap_or("");
                let note = if run_id.is_empty() {
                    "Vibe agent started for project '{project_name}'.".to_string()
                } else {
                    format!("Vibe agent started for project '{project_name}' (run #{run_id}).")
                };
                let resp = botlib::models::BotResponse::new(
                    bot_uuid.to_string(),
                    session_id.to_string(),
                    user_id.to_string(),
                    &note,
                    "web",
                );
                let _ = sink.send_bot_response(&resp).await;

                // #e2e — report the run's outcome back into this chat. The
                // "agent started" note above is the last thing the user sees
                // otherwise, so they never learn whether the edit applied or
                // the deploy published. A poller watches the run state via
                // the same loopback API and pushes one final BotResponse
                // through the live session channel (response_channels), so
                // the message lands in the open Chat window even after this
                // pipeline call returns.
                if !run_id.is_empty() {
                    let state = state.clone();
                    let run_id = run_id.to_string();
                    let project_name = project_name.clone();
                    let bot_uuid = bot_uuid;
                    let session_id = session_id;
                    let user_id = user_id;
                    tokio::spawn(async move {
                        // #vibe-verbose — narrate progress milestones into the
                        // open Chat window while the run executes. The user
                        // previously waited blind between "agent started" and
                        // the final verdict; these intermediate messages make
                        // the agent loop legible (tool calls, verifications,
                        // failures) without flooding: one message per change
                        // of milestone label.
                        let mut last_milestone = String::new();
                        let send_chat = |state: Arc<AppState>, text: String| async move {
                            let resp = botlib::models::BotResponse::new(
                                bot_uuid.to_string(),
                                session_id.to_string(),
                                user_id.to_string(),
                                &text,
                                "web",
                            );
                            let channels = state.response_channels.lock().await;
                            if let Some(tx) = channels.get(&session_id.to_string()) {
                                let _ = tx.try_send(resp);
                            }
                        };
                        let final_state = loop {
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                            let value = match crate::core::bot::api_exec::exec_endpoint(
                                &state,
                                user_id,
                                Some(bot_uuid),
                                "GET",
                                &format!("/api/vibe/run/{run_id}"),
                                &serde_json::Map::new(),
                            )
                            .await
                            {
                                Ok(v) => v,
                                Err(_) => return,
                            };
                            let s = value
                                .get("state")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            if matches!(s.as_str(), "completed" | "failed" | "cancelled")
                                || s.is_empty()
                            {
                                break s;
                            }
                            // Milestone narration: derive a short label from
                            // the run shape (tool-call count) and only speak
                            // when it changes.
                            let tools = value
                                .get("tool_call_count")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            // #1394 — name the tool being run so the user can
                            // tell WHAT the agent is doing, not just how much.
                            let last_tool = value
                                .get("last_tool_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let tool_word = if last_tool.is_empty() {
                                String::new()
                            } else {
                                format!(" ({last_tool})")
                            };
                            let milestone = match s.as_str() {
                                "awaiting_approval" => {
                                    format!("⏸ Waiting for your approval to continue ({} tool call{} so far).", tools, if tools == 1 { "" } else { "s" })
                                }
                                _ => {
                                    if tools > 0 {
                                        format!("⚙️ Working on '{project_name}'… {} tool call{} done{tool_word}.", tools, if tools == 1 { "" } else { "s" })
                                    } else {
                                        format!("🧠 Planning the changes for '{project_name}'…")
                                    }
                                }
                            };
                            if milestone != last_milestone {
                                last_milestone = milestone.clone();
                                send_chat(state.clone(), milestone).await;
                            }
                        };
                        let text = match final_state.as_str() {
                            "completed" => format!(
                                "✅ Vibe agent finished for '{project_name}' — the changes are ready to go."
                            ),
                            "failed" => format!(
                                "❌ Vibe run for '{project_name}' failed — check the Runner Log for details."
                            ),
                            "cancelled" => {
                                format!("⏹ Vibe run for '{project_name}' was cancelled.")
                            }
                            _ => return,
                        };
                        send_chat(state, text).await;
                    });
                }
            }
            Err(e) => {
                log::warn!("Vibe chat routing failed for project {pid}: {e}");
                let resp = botlib::models::BotResponse::new(
                    bot_uuid.to_string(),
                    session_id.to_string(),
                    user_id.to_string(),
                    format!("Vibe agent could not start for '{project_name}': {e}"),
                    "web",
                );
                let _ = sink.send_bot_response(&resp).await;
            }
        }
        return Ok(true);
    }
    Ok(false)
}
