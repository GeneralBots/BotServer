//! `agent_loop::llm_2` — split per #1443 (AGENTS.md 450-line rule).

use super::*;

impl AgentLoop {
    pub(crate) async fn run_loop(&self, run: &mut VibeRun, max_steps: u32, deadline: tokio::time::Instant) {
        let mut context =
            self.prompt_manager
                .build_context(run.use_case, &run.config.lang, &run.intent, &[]);
        context.run_id = run.run_id;
        if let Some(project) = run.config.project_name.as_deref() {
            context.system_prompt.push_str(&format!(
                "\\n\\nACTIVE VIBE PROJECT: {project}. For every file, shell, git, test, and log tool call, use this exact project value; never use a server path or another project."
            ));
        }

        // Unambiguous local Windows maintenance actions must not wait for
        // skill grounding or depend on a small model producing valid JSON.
        // They still pass through process_tool_call, which enforces project
        // scoping, approval, permissions, telemetry, and tool result checks.
        let local_tool_call = {
            let (_, _, api_url) = self.resolve_llm(run);
            Self::local_deterministic_tool_response(&api_url, run)
                .and_then(|response| self.parse_tool_calls(&response).into_iter().next())
        };
        if let Some(tool_call) = local_tool_call {
            self.broadcast_event(run, "thinking", "Preparing local project action", 5);
            match self
                .process_tool_call(run, &mut context, &tool_call, 0, max_steps)
                .await
            {
                ToolStep::Executed | ToolStep::Skipped => {
                    self.finish_truthfully(run, "Local project action completed")
                        .await;
                }
            }
            return;
        }

        let triggered = self.skills.auto_trigger(&run.intent).await;
        let grounding_refs: Vec<String> = triggered.iter().map(|s| s.content.clone()).collect();

        // #1446 G8 — the planner sidecar was a pure REST router the loop
        // never called; the loop now decomposes the intent planner-style
        // with the run's own LLM and injects the steps as guidance. On
        // planner failure (or timeout) the normal loop path continues
        // unchanged.
        self.inject_planner_guidance(run, &mut context).await;

        let mut empty_parse_rounds: u32 = 0;
        let mut verify_failures: u32 = 0;
        for step in 0..max_steps {
            if run.state == VibeRunState::Cancelled {
                info!("Vibe run {} cancelled at step {}", run.run_id, step);
                return;
            }

            if self.budget_exceeded(run).await {
                run.transition(VibeRunState::Failed);
                run.error = Some(format!(
                    "Budget cap exceeded ({} cents)",
                    run.config.budget_cents
                ));
                self.sync_active_run(run).await;
                self.telemetry
                    .record_run_completion(run, 0, None, 0.0)
                    .await;
                self.broadcast_event(run, "failed", "Budget cap exceeded", 100);
                return;
            }

            let progress = ((step as f64 / max_steps as f64) * 100.0) as u8;
            self.broadcast_event(
                run,
                "thinking",
                &format!("Step {}/{}", step + 1, max_steps),
                progress,
            );

            context.kb_references = {
                let mut refs = grounding_refs.clone();
                refs.extend(crate::grounding::sources_for_run(run));
                refs
            };

            let (llm_response, llm_usage) =
                match self.call_llm_with_retry(&context, run, &run.intent, deadline).await {
                    Ok((response, usage)) => (response, usage),
                    Err(e) => {
                        error!("LLM call failed at step {}: {}", step, e);
                        run.transition(VibeRunState::Failed);
                        run.error = Some(format!(
                            "LLM call failed at step {step} after {} attempts: {e}",
                            MAX_LLM_RETRIES + 1
                        ));
                        self.sync_active_run(run).await;
                        self.telemetry
                            .record_run_completion(run, 0, None, 0.0)
                            .await;
                        return;
                    }
                };

            // #923 — record real provider usage so the budget meter and cost
            // accounting reflect actual spend instead of a hardcoded 0.0.
            if let Some(usage) = llm_usage {
                let model = run.config.model.clone().unwrap_or_default();
                let cost = estimate_llm_cost(&model, usage.prompt_tokens, usage.completion_tokens);
                let mut meta = std::collections::HashMap::new();
                meta.insert("input_tokens".to_string(), usage.prompt_tokens.to_string());
                meta.insert("output_tokens".to_string(), usage.completion_tokens.to_string());
                self.telemetry
                    .record_tool_call(ToolCallRecord {
                        run_id: run.run_id,
                        use_case: run.use_case,
                        tool_name: "llm/chat".to_string(),
                        latency_ms: 0,
                        tokens: Some(usage.prompt_tokens.saturating_add(usage.completion_tokens)),
                        cost,
                        success: true,
                        error: None,
                        metadata: meta,
                    })
                    .await;
            }

            let tool_calls = self.parse_tool_calls(&llm_response);

            if tool_calls.is_empty() {
                // #923b — when the intent requires a mutation and none has
                // succeeded yet, a prose-only reply is NOT completion: the
                // model described the change instead of applying it (observed
                // with small models that narrate after an exploratory
                // file/list). Nudge it toward the tools, within the empty
                // parse budget, before accepting the text as final.
                let mutation_pending = Self::intent_requires_mutation(&run.intent)
                    && !run.tool_calls.iter().any(|call| {
                        matches!(
                            call.tool_name.as_str(),
                            "file/write"
                                | "file/replace"
                                | "file/delete"
                                | "file/set-title"
                                | "shell/run"
                                | "git/commit"
                                | "publish/project"
                        ) && call.result.as_ref().is_some_and(|result| result.success)
                    });
                if !looks_like_tool_intent(&llm_response)
                    && !mutation_pending
                    || empty_parse_rounds + 1 >= MAX_EMPTY_PARSE_RETRIES
                {
                    context.add_assistant_message(llm_response);
                    self.finish_truthfully(run, "Agent finished — no more tool calls")
                        .await;
                    return;
                }
                empty_parse_rounds += 1;
                context.add_assistant_message(llm_response);
                // The nudge must arrive as a USER message: appended as an
                // assistant turn, small models read it back as their own
                // narration and keep producing prose (observed on dev with
                // six consecutive prose rounds while the file was never
                // written). As a user turn it is an authoritative demand.
                context.add_user_message(
                    if mutation_pending {
                        "The requested change has NOT been applied yet. Reply with tool_calls \
                         now — for example {\"tool_calls\":[{\"tool_name\":\"file/replace\",\
                         \"arguments\":{\"project\":\"…\",\"path\":\"…\",\"old\":\"…\",\
                         \"new\":\"…\"}}]} — do not describe the change in prose.\
                         If the task is truly complete, answer plainly without any \
                         tool_calls key."
                        .to_string()
                    } else {
                        "Your previous response could not be parsed into tool calls. \
                         Return a JSON object exactly like: \
                         {\"tool_calls\": [{\"tool_name\": \"example_tool\", \"arguments\": {}}]} \
                         — or, if the task is complete, answer plainly without any tool_calls key."
                            .to_string()
                    },
                );
                continue;
            }
            empty_parse_rounds = 0;
            context.add_assistant_message(llm_response);

            let mut step_mutated = false;
            let mut cap_reached = false;
            for tc in &tool_calls {
                if (run.tool_calls.len() as u32) >= max_steps {
                    info!(
                        "Vibe run {} reached tool-call cap ({max_steps}) at step {}",
                        run.run_id, step
                    );
                    cap_reached = true;
                    break;
                }
                match self
                    .process_tool_call(run, &mut context, tc, step, max_steps)
                    .await
                {
                    // Self-verification only makes sense after a step that can
                    // change the workspace. Read-only tools (`file/list`,
                    // `file/read`, `file/exists`, searches) leave nothing to
                    // verify, and asking right after a read made competent
                    // models answer FAILED, which ended the run through
                    // MAX_VERIFY_FAILURES before the first write.
                    ToolStep::Executed => {
                        step_mutated = step_mutated
                            || crate::permissions::is_mutating_tool(&tc.tool_name);
                    }
                    ToolStep::Skipped => {}
                }
            }

            if cap_reached {
                // Tool-call budget exhausted: the work requested is bounded,
                // so finish the run instead of looping into the timeout.
                run.transition(VibeRunState::Completed);
                self.sync_active_run(run).await;
                self.telemetry
                    .record_run_completion(run, 0, None, 0.0)
                    .await;
                self.broadcast_event(
                    run,
                    "completed",
                    "Tool-call cap reached — run finished with the work produced so far",
                    100,
                );
                return;
            }

            if step_mutated {
                self.broadcast_event(run, "verifying", "Self-verifying latest changes", progress);
                let verified = self.verify_latest(&mut context, run).await;
                if !verified {
                    verify_failures += 1;
                    context.add_assistant_message(
                        "Self-verification failed for the latest change. Recheck and correct it before proceeding."
                            .to_string(),
                    );
                    if verify_failures >= MAX_VERIFY_FAILURES {
                        // vibe33 #814 — a repeated failed verification must
                        // not soft-complete with unverified work. Fail the
                        // run with a distinct verdict; the produced workspace
                        // artifacts are NOT deleted, only the run verdict is
                        // honest about them.
                        run.transition(VibeRunState::Failed);
                        run.error = Some(
                            "Self-verification failed repeatedly; run failed (produced artifacts remain in the workspace)."
                                .to_string(),
                        );
                        self.sync_active_run(run).await;
                        warn!(
                            "Vibe run {} self-verification failed repeatedly; run failed",
                            run.run_id
                        );
                        self.telemetry
                            .record_run_completion(run, 0, None, 0.0)
                            .await;
                        self.broadcast_event(
                            run,
                            "failed",
                            "Self-verification failed repeatedly; run failed",
                            100,
                        );
                        return;
                    }
                } else {
                    verify_failures = 0;
                }
            }

            tokio::task::yield_now().await;
        }

        if run.state == VibeRunState::Running {
            self.finish_truthfully(run, "Max steps reached — loop completed")
                .await;
        }
    }

    /// Resolves (model, api_key, api_url) for a run: explicit run overrides >
    /// the run's agent slot provider (Settings → Vibe, per-agent type) >
    /// per-bot config (Vault for secrets, Drive config.csv for the rest) >
    /// environment > built-in defaults.
    pub(crate) fn resolve_llm(&self, run: &VibeRun) -> (String, String, String) {
        let llm = run
            .config
            .agent
            .as_deref()
            .and_then(|agent| self.state.llm_config_for(&run.bot_id, agent))
            .or_else(|| self.state.llm_config(&run.bot_id));
        let model = run
            .config
            .model
            .clone()
            .or_else(|| llm.as_ref().map(|l| l.model.clone()))
            .or_else(|| std::env::var("LLM_MODEL").ok())
            .unwrap_or_else(|| "gpt-4o-mini".to_string());
        let api_key = run
            .config
            .llm_key
            .clone()
            .or_else(|| llm.as_ref().map(|l| l.key.clone()))
            .or_else(|| std::env::var("LLM_KEY").ok())
            .unwrap_or_default();
        let api_url = run
            .config
            .llm_url
            .clone()
            .or_else(|| llm.as_ref().map(|l| l.url.clone()))
            .or_else(|| std::env::var("LLM_URL").ok())
            .unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".to_string());
        (model, api_key, api_url)
    }

    /// #1446 G8 — planner decomposition before the loop: decomposes the
    /// intent into a short numbered plan with the run's own LLM and injects
    /// the steps as guidance into the system prompt. Resilient by design:
    /// any failure (no LLM, timeout, unparsable JSON) is logged and the
    /// normal loop path continues unchanged.
    pub(crate) async fn inject_planner_guidance(&self, run: &VibeRun, context: &mut crate::types::VibeContext) {
        let (model, api_key, api_url) = self.resolve_llm(run);
        let settings = crate::llm_client::LlmSettings {
            url: api_url,
            model,
            key: api_key,
        };
        let decompose = crate::llm_client::chat_completion(
            &settings,
            "You decompose an intent into a short numbered execution plan. Reply with ONLY a JSON \
             array of step strings, no prose, no markdown fences.",
            &run.intent,
        );
        let parsed = match timeout(Duration::from_secs(20), decompose).await {
            Ok(Ok(raw)) => {
                let json = crate::llm_client::extract_json(&raw);
                serde_json::from_str::<Vec<String>>(&json).ok()
            }
            Ok(Err(e)) => {
                log::debug!("Vibe run {}: planner decomposition skipped: {e}", run.run_id);
                None
            }
            Err(_) => {
                log::debug!("Vibe run {}: planner decomposition timed out", run.run_id);
                None
            }
        };
        let Some(plan) = parsed else {
            return;
        };
        if plan.is_empty() {
            return;
        }
        let mut guidance =
            String::from("\n\nPLANNED STEPS (follow in order; adapt when verification fails):\n");
        for (i, step) in plan.iter().enumerate().take(10) {
            guidance.push_str(&format!("{}. {step}\n", i + 1));
        }
        context.system_prompt.push_str(&guidance);
        self.broadcast_event(
            run,
            "planning",
            &format!("Plan: {} steps", plan.len()),
            15,
        );
    }
}
