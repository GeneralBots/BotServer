//! `agent_loop::llm_3` — split per #1443 (AGENTS.md 450-line rule).

use super::*;

/// The provider call `call_llm_nonstream` issues: endpoint, credentials, model
/// and the composed conversation, grouped so the signature stays short.
pub(crate) struct LlmRequest<'a> {
    api_url: &'a str,
    api_key: &'a str,
    model: &'a str,
    system: &'a str,
    prompt: &'a str,
    tools: &'a [serde_json::Value],
    tool_choice: &'a str,
}

impl AgentLoop {
    /// vibe33 #813 — retries the LLM call with short backoff so transient
    /// provider failures do not kill the whole run. Deterministic client
    /// errors (bad key, wrong model, malformed request) fail fast on the
    /// first attempt instead of wasting the retry budget (#932).
    pub(crate) async fn call_llm_with_retry(
        &self,
        context: &crate::types::VibeContext,
        run: &VibeRun,
        user_message: &str,
        deadline: tokio::time::Instant,
    ) -> Result<(String, Option<LlmUsage>), String> {
        let mut last_error = String::new();
        for attempt in 0..=MAX_LLM_RETRIES {
            // #1270 — budget-aware: when the remaining run time can no
            // longer fit one more attempt (request timeout + short backoff),
            // stop here with the real cause instead of letting the outer
            // run timeout kill the loop mid-flight with a generic
            // "Agent loop timed out".
            let remaining = deadline
                .checked_duration_since(tokio::time::Instant::now())
                .unwrap_or_default();
            if remaining <= Duration::from_secs(LLM_REQUEST_TIMEOUT_SECS) {
                break;
            }
            match self.call_llm(context, run, user_message).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = e.clone();
                    if Self::is_non_retryable_llm_error(&e) {
                        warn!(
                            "Vibe run {} LLM call rejected (non-retryable): {e}",
                            run.run_id
                        );
                        break;
                    }
                    if attempt == MAX_LLM_RETRIES {
                        break;
                    }
                    let backoff = LLM_RETRY_BACKOFF_SECS
                        .get(attempt as usize)
                        .copied()
                        .unwrap_or(3);
                    warn!(
                        "Vibe run {} LLM call failed (attempt {}), retrying in {backoff}s: {e}",
                        run.run_id,
                        attempt + 1
                    );
                    tokio::time::sleep(Duration::from_secs(backoff)).await;
                }
            }
        }
        Err(last_error)
    }

    pub(crate) async fn call_llm(
        &self,
        context: &crate::types::VibeContext,
        run: &VibeRun,
        user_message: &str,
    ) -> Result<(String, Option<LlmUsage>), String> {
        let mut prompt = self.prompt_manager.compose_prompt(context, user_message);
        // #1446 G7 — capabilities reach the LLM as text too: the tool
        // schemas below are already filtered by use case, and the prompt
        // now names the same allowed set so it never promises a tool the
        // executor would reject.
        let allowed_tools = self.tool_schemas_for(run.use_case).await;
        if !allowed_tools.is_empty() {
            let mut section = String::from(
                "\n\nAvailable capabilities for this use case (only these tools may be called):\n",
            );
            for tool in allowed_tools.iter().take(40) {
                if let (Some(name), Some(desc)) = (
                    tool["function"]["name"].as_str(),
                    tool["function"]["description"].as_str(),
                ) {
                    section.push_str(&format!("- {name}: {desc}\n"));
                }
            }
            prompt.push_str(&section);
        }
        let system = self
            .prompt_manager
            .system_prompt_for(run.use_case, &run.config.lang);
        // Per-bot config (Issue #795): explicit run overrides > config
        // (Vault for secrets, Drive config.csv for the rest) via the state
        // > environment > built-in defaults.
        let (model, api_key, api_url) = self.resolve_llm(run);

        // The bundled Windows model is intentionally small and can emit
        // malformed function-call JSON even when a single tool is required.
        // For unambiguous local maintenance intents, build the registered
        // tool call deterministically; execution still goes through normal
        // permissions, approval, telemetry, and project scoping.
        if let Some(response) = Self::local_deterministic_tool_response(&api_url, run) {
            return Ok((response, None));
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(LLM_REQUEST_TIMEOUT_SECS))
            .user_agent(LLM_USER_AGENT)
            .build()
            .map_err(|e| format!("http client: {e}"))?;
        let mut tools = self.tool_schemas_for(run.use_case).await;
        let forced_tool = Self::local_forced_tool(&api_url, run);
        if let Some(name) = forced_tool {
            // Schema names are LLM-sanitized (#1276): compare the forced
            // canonical name through the same encoding.
            let wire_name = sanitize_tool_name_for_llm(name);
            tools.retain(|tool| tool["function"]["name"].as_str() == Some(wire_name.as_str()));
        }
        // Kiro provider (secret/gbo/llm with provider=kiro): the ksk_ key
        // speaks the CodeWhisperer protocol, not OpenAI SSE. Translate the
        // same system/prompt/tools and map the response back to the canonical
        // tool-call envelope.
        if crate::kiro_bridge::is_kiro(&api_url) {
            return crate::kiro_bridge::call_kiro(
                &api_url,
                &api_key,
                &model,
                &system,
                &prompt,
                &tools,
                MAX_LLM_RETRIES as usize,
            )
            .await;
        }
        // Force a native tool call while the request still needs work:
        // reasoning models (e.g. NVIDIA gpt-oss) that are simultaneously told
        // to "respond in the JSON envelope" stream their whole plan as content
        // text instead of emitting `tool_calls` deltas, so an unforced parse
        // yields nothing and the loop retries until the run timeout (fix
        // #1294). With `tool_choice: "required"` the provider emits native tool
        // calls reliably. Keep it required until a mutation tool has actually
        // succeeded (writes/replace/delete/shell/publish), not merely until the
        // first read ran — otherwise the model inspects the workspace, then
        // flips to "auto" and answers in prose without applying the edit.
        let mutation_done = run.tool_calls.iter().any(|call| {
            matches!(
                call.tool_name.as_str(),
                "file/write"
                    | "file/replace"
                    | "file/delete"
                    | "file/set-title"
                    | "shell/run"
                    | "test/run"
                    | "git/commit"
                    | "publish/project"
            ) && call.result.as_ref().is_some_and(|result| result.success)
        });
        let tool_choice = if forced_tool.is_some() || (!tools.is_empty() && !mutation_done) {
            "required"
        } else {
            "auto"
        };
        let body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.3,
            "max_tokens": 4096,
            "tools": tools,
            "tool_choice": tool_choice,
            "stream": true,
            "stream_options": {"include_usage": true},
        });

        // Native streaming with SSE accumulation (Issue #794); the streamed
        // content and tool-call deltas are reassembled into the canonical
        // form `parse_tool_calls` already understands.
        let send_result = client
            .post(&api_url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await;

        let resp = match send_result {
            Ok(r) => r,
            Err(e) => return Err(format!("HTTP request failed: {e}")),
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            // Some providers reject the `tools` field; retry without it so
            // the plain chat path keeps working (fallback per issue #794).
            if text.contains("tool") || text.contains("function") {
                return self
                    .call_llm_plain(&api_url, &api_key, &model, &system, &prompt)
                    .await;
            }
            return Err(format!("LLM returned status {status}: {text}"));
        }

        let payload = match self.read_body(resp).await {
            Ok(payload) => payload,
            Err(e) => {
                // Streaming failed (e.g. truncated body on a large context) —
                // retry non-streaming with the same tool set so function
                // calling still works instead of failing the run.
                warn!("LLM streaming read failed ({e}); retrying non-streaming");
                return self
                    .call_llm_nonstream(LlmRequest {
                        api_url: &api_url,
                        api_key: &api_key,
                        model: &model,
                        system: &system,
                        prompt: &prompt,
                        tools: &tools,
                        tool_choice,
                    })
                    .await;
            }
        };
        let usage = usage_from_payload(&payload);
        match Self::content_from_payload(&payload) {
            Ok(content) => Ok((content, usage)),
            Err(e) if e.contains("truncated") => {
                // The stream ended mid-tool-argument (e.g. the model hit the
                // output token cap while writing several large files). Retry
                // non-streaming so a complete arguments document is returned
                // instead of silently writing an empty/corrupt file.
                warn!("LLM streaming truncated tool-call arguments ({e}); retrying non-streaming");
                self.call_llm_nonstream(LlmRequest {
                    api_url: &api_url,
                    api_key: &api_key,
                    model: &model,
                    system: &system,
                    prompt: &prompt,
                    tools: &tools,
                    tool_choice,
                })
                .await
            }
            Err(e) if e.contains("empty content") => {
                // #1270 — providers behind flaky gateways (NVIDIA, CodeBuddy)
                // intermittently complete the stream with ZERO bytes and a
                // 2xx. That is a transient provider fault, not a valid empty
                // turn: surface it as an error so call_llm_with_retry retries
                // instead of the loop burning an empty-parse round or
                // accepting the turn as final.
                warn!("LLM stream completed empty ({e}); retrying non-streaming");
                self.call_llm_nonstream(LlmRequest {
                    api_url: &api_url,
                    api_key: &api_key,
                    model: &model,
                    system: &system,
                    prompt: &prompt,
                    tools: &tools,
                    tool_choice,
                })
                .await
            }
            Err(e) => Err(e),
        }
    }

    /// Non-streaming fallback: identical request (tools + system prompt) but
    /// `stream: false`, so the response is a single JSON document instead of
    /// an SSE stream — more robust against truncated/decoded bodies.
    pub(crate) async fn call_llm_nonstream(
        &self,
        request: LlmRequest<'_>,
    ) -> Result<(String, Option<LlmUsage>), String> {
        let LlmRequest {
            api_url,
            api_key,
            model,
            system,
            prompt,
            tools,
            tool_choice,
        } = request;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(LLM_REQUEST_TIMEOUT_SECS))
            .user_agent(LLM_USER_AGENT)
            .build()
            .map_err(|e| format!("http client: {e}"))?;
        let body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.3,
            "max_tokens": 4096,
            "tools": tools,
            "tool_choice": tool_choice,
            "stream": false,
        });
        let resp = client
            .post(api_url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("LLM returned status {status}: {text}"));
        }
        let payload = self.read_body(resp).await?;
        let usage = usage_from_payload(&payload);
        Self::content_from_payload(&payload).map(|content| (content, usage))
    }

    /// OpenAI-style `tools` array derived from the live tool registry.
    pub(crate) async fn tool_schemas_for(&self, use_case: VibeUseCase) -> Vec<serde_json::Value> {
        self.tool_executor
            .registry()
            .list_tools_for_use_case(use_case)
            .await
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": sanitize_tool_name_for_llm(&t.schema.name),
                        "description": t.schema.description,
                        "parameters": t.schema.parameters,
                    }
                })
            })
            .collect()
    }

    /// Extracts either native tool calls (canonical envelope) or plain text
    /// content from an OpenAI-style completion payload. Returns an error when
    /// a tool call's `arguments` is non-empty but not valid JSON — that means
    /// the stream was truncated mid-argument, and silently substituting `{}`
    /// would write an empty/corrupt file.
    pub(crate) fn content_from_payload(payload: &serde_json::Value) -> Result<String, String> {
        if let Some(calls) = payload["choices"][0]["message"]["tool_calls"].as_array() {
            let mut has_call = false;
            for tc in calls {
                let name = tc["function"]["name"].as_str().unwrap_or_default();
                let args_raw = tc["function"]["arguments"].as_str().unwrap_or_default();
                if name.is_empty() && args_raw.is_empty() {
                    continue;
                }
                has_call = true;
                if !args_raw.is_empty()
                    && serde_json::from_str::<serde_json::Value>(args_raw).is_err()
                {
                    return Err("LLM tool-call arguments were truncated (invalid JSON)".to_string());
                }
            }
            if has_call {
                if let Some(calls) = native_tool_calls_from_value(payload) {
                    return Ok(canonical_tool_calls_json(&calls));
                }
            }
        }
        let content = payload["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        if content.is_empty() {
            return Err("LLM returned empty content".to_string());
        }
        Ok(content)
    }

    /// Reads the entire HTTP body, classifying it as either an SSE stream
    /// (`data: ...` lines, Issue #794) or a plain JSON completion payload.
    /// Bounded by `read_body_timeout()` so stalled provider streams fail
    /// fast instead of hanging the whole run until the run-level timeout.
    pub(crate) async fn read_body(&self, mut resp: reqwest::Response) -> Result<serde_json::Value, String> {
        let read_timeout = Duration::from_secs(120);
        let result = timeout(read_timeout, Self::read_body_inner(&mut resp))
            .await
            .map_err(|_| "LLM response body read timed out".to_string());
        match result {
            Ok(inner) => inner,
            Err(e) => {
                log::warn!("read_body: {e}");
                Err(e)
            }
        }
    }
}
