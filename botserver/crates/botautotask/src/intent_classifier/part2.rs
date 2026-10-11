use super::*;

impl IntentClassifier {
    pub(super) fn handle_monitor(
        &self, classification: &ClassifiedIntent, session: &UserSession,
    ) -> Result<IntentResult, Box<dyn std::error::Error + Send + Sync>> {
        info!("Handling MONITOR intent");
        let subject = classification.entities.subject.clone().unwrap_or_else(|| "data".to_string());
        let condition = classification.entities.condition.clone().unwrap_or_else(|| "changes".to_string());
        let handler_name = format!("monitor_{}.bas", subject.to_lowercase().replace(' ', "_"));
        // `ON CHANGE` / `END ON` are unregistered (#1359), and `condition` is
        // free text rather than a Rhai expression, so the old body failed to
        // compile twice over. Emit the supported subset until the event
        // subsystem ships.
        let basic_code = format!(
            "' Monitor: {subject}\n' Condition: {condition}\n' Created: {}\n\nTALK \"Monitor active for {subject} ({condition})\"\n",
            Utc::now().format("%Y-%m-%d %H:%M")
        );
        let event_path = format!("events/{handler_name}");
        self.save_basic_file(session.bot_id, &event_path, &basic_code)?;
        Ok(IntentResult {
            success: true, intent_type: IntentType::Monitor,
            message: format!("Monitor created for: {subject}"),
            created_resources: vec![CreatedResource { resource_type: "event".to_string(), name: handler_name, path: Some(event_path) }],
            app_url: None, task_id: None, schedule_id: None, tool_triggers: Vec::new(),
            next_steps: vec![format!("You'll be notified when {subject} {condition}")], error: None,
        })
    }

    pub(super) fn handle_action_placeholder(
        &self, classification: &ClassifiedIntent, _session: &UserSession,
    ) -> Result<IntentResult, Box<dyn std::error::Error + Send + Sync>> {
        info!("Handling ACTION intent (placeholder — IntentCompiler integration needed)");
        Ok(IntentResult {
            success: true, intent_type: IntentType::Action,
            message: format!("Action noted: {}", classification.original_text),
            created_resources: Vec::new(), app_url: None, task_id: None,
            schedule_id: None, tool_triggers: Vec::new(),
            next_steps: vec!["Use IntentCompiler for execution planning".to_string()], error: None,
        })
    }

    pub(super) fn handle_schedule(
        &self, classification: &ClassifiedIntent, session: &UserSession,
    ) -> Result<IntentResult, Box<dyn std::error::Error + Send + Sync>> {
        info!("Handling SCHEDULE intent");
        let schedule_name = classification.suggested_name.clone().unwrap_or_else(|| "scheduled-task".to_string()).to_lowercase().replace(' ', "-");
        let time_spec = classification.entities.time_spec.as_ref().map(|ts| {
            format!("{} at {}", match ts.schedule_type {
                ScheduleType::Daily => "Every day", ScheduleType::Weekly => "Every week",
                ScheduleType::Monthly => "Every month", _ => "Once",
            }, ts.time.as_deref().unwrap_or("9:00 AM"))
        }).unwrap_or_else(|| "Every day at 9:00 AM".to_string());
        let scheduler_file = format!("{schedule_name}.bas");
        // `SET SCHEDULE` is a compiler directive consumed by the preprocessor
        // (which registers the cron), so it must not be paired with an
        // `END SCHEDULE` block: that keyword is unregistered and the script
        // failed to compile (#1359).
        let basic_code = format!(
            "' Scheduler: {schedule_name}\n' Schedule: {time_spec}\n' Created: {}\n\nSET SCHEDULE \"{time_spec}\"\nTALK \"Running scheduled task: {schedule_name}\"\n",
            Utc::now().format("%Y-%m-%d %H:%M")
        );
        let scheduler_path = format!("schedulers/{scheduler_file}");
        self.save_basic_file(session.bot_id, &scheduler_path, &basic_code)?;
        let schedule_id = Uuid::new_v4();
        Ok(IntentResult {
            success: true, intent_type: IntentType::Schedule,
            message: format!("Scheduler created: {scheduler_file}\nSchedule: {time_spec}"),
            created_resources: vec![CreatedResource { resource_type: "scheduler".to_string(), name: scheduler_file, path: Some(scheduler_path) }],
            app_url: None, task_id: None, schedule_id: Some(schedule_id.to_string()), tool_triggers: Vec::new(),
            next_steps: vec![format!("The task will run {time_spec}")], error: None,
        })
    }

    pub(super) fn handle_goal(
        &self, classification: &ClassifiedIntent, session: &UserSession,
    ) -> Result<IntentResult, Box<dyn std::error::Error + Send + Sync>> {
        info!("Handling GOAL intent");
        let goal_name = classification.suggested_name.clone().unwrap_or_else(|| "goal".to_string());
        let target = classification.entities.target_value.clone().unwrap_or_else(|| "unspecified".to_string());
        // `SET GOAL` / `GET_METRIC` / `END GOAL` are not registered in the Rhai
        // engine (#1359); emit the supported subset so the script compiles.
        let basic_code = format!(
            "' Goal: {goal_name}\n' Target: {target}\n' Created: {}\n\nTALK \"Goal {goal_name} registered with target: {target}\"\n",
            Utc::now().format("%Y-%m-%d %H:%M")
        );
        let goal_file = format!("{}.bas", goal_name.to_lowercase().replace(' ', "-"));
        let goal_path = format!("goals/{goal_file}");
        self.save_basic_file(session.bot_id, &goal_path, &basic_code)?;
        Ok(IntentResult {
            success: true, intent_type: IntentType::Goal,
            message: format!("Goal created: {goal_name}\nTarget: {target}"),
            created_resources: vec![CreatedResource { resource_type: "goal".to_string(), name: goal_name, path: Some(goal_path) }],
            app_url: None, task_id: None, schedule_id: None, tool_triggers: Vec::new(),
            next_steps: vec!["The system will work toward this goal autonomously".to_string()], error: None,
        })
    }

    pub(super) fn handle_tool(
        &self, classification: &ClassifiedIntent, session: &UserSession,
    ) -> Result<IntentResult, Box<dyn std::error::Error + Send + Sync>> {
        info!("Handling TOOL intent");
        let tool_name = classification.suggested_name.clone().unwrap_or_else(|| "custom-command".to_string()).to_lowercase().replace(' ', "-");
        let triggers = if classification.entities.trigger_phrases.is_empty() {
            vec![tool_name.clone()]
        } else { classification.entities.trigger_phrases.clone() };
        let triggers_str = triggers.iter().map(|t| format!("\"{}\"", t)).collect::<Vec<_>>().join(", ");
        let tool_file = format!("{tool_name}.bas");
        // `TRIGGER` / `END TRIGGER` are not registered in the Rhai engine
        // (#1359); the triggers are recorded as comments and the tool is
        // associated by its filename, so the script compiles today.
        let basic_code = format!(
            "' Tool: {tool_name}\n' Triggers: {triggers_str}\n' Created: {}\n\nTALK \"Running command: {tool_name}\"\n",
            Utc::now().format("%Y-%m-%d %H:%M")
        );
        let tool_path = format!("tools/{tool_file}");
        self.save_basic_file(session.bot_id, &tool_path, &basic_code)?;
        Ok(IntentResult {
            success: true, intent_type: IntentType::Tool,
            message: format!("Command created: {tool_file}\nTriggers: {}", triggers.join(", ")),
            created_resources: vec![CreatedResource { resource_type: "tool".to_string(), name: tool_file, path: Some(tool_path) }],
            app_url: None, task_id: None, schedule_id: None, tool_triggers: triggers,
            next_steps: vec!["Say any of the trigger phrases to use the command".to_string()], error: None,
        })
    }

    pub(super) fn handle_unknown(classification: &ClassifiedIntent) -> Result<IntentResult, Box<dyn std::error::Error + Send + Sync>> {
        info!("Handling UNKNOWN intent");
        let suggestions = if classification.alternative_types.is_empty() {
            "- Create an app\n- Add a task\n- Set up monitoring\n- Schedule automation".to_string()
        } else {
            classification.alternative_types.iter().map(|a| format!("- {}: {}", a.intent_type, a.reason)).collect::<Vec<_>>().join("\n")
        };
        Ok(IntentResult {
            success: false, intent_type: IntentType::Unknown,
            message: format!("I'm not sure what you'd like me to do. Could you clarify?\n\nPossible interpretations:\n{suggestions}"),
            created_resources: Vec::new(), app_url: None, task_id: None,
            schedule_id: None, tool_triggers: Vec::new(),
            next_steps: vec!["Provide more details about what you want".to_string()], error: None,
        })
    }

    pub(crate) async fn call_llm(&self, _prompt: &str, _bot_id: Uuid) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        #[cfg(feature = "llm")]
        {
            // A present-but-empty value is as good as an error: chaining on
            // `Err` alone sent the gateway an empty model name and every
            // classification died with 404 model_not_found. Scope order is
            // bot, then global; an empty result is an explicit error, never a
            // hardcoded vendor guess.
            let mut model = String::new();
            let mut key = String::new();
            for id in [_bot_id, Uuid::nil()] {
                if model.is_empty() {
                    if let Ok(value) = self.config_ops.get_config(&id, "llm-model", None) {
                        model = value.trim().to_string();
                    }
                }
                if key.is_empty() {
                    if let Ok(value) = self.config_ops.get_config(&id, "llm-key", None) {
                        key = value.trim().to_string();
                    }
                }
            }
            if model.is_empty() {
                return Err(format!(
                    "no llm-model configured for bot {_bot_id} (checked the bot scope, then the global one)"
                )
                .into());
            }
            let llm_config = serde_json::json!({"temperature": 0.3, "max_tokens": 1000});
            // Bounded: the API path already ran the heuristic classifier first,
            // so a stalled model degrades to a reported failure.
            let result = crate::types::collect_llm_stream(
                self.llm_ops.as_ref(), _prompt, &llm_config, &model, &key, None,
            )
            .await;
            if result.is_err() {
                warn!("Intent classification via model '{model}' failed or timed out");
            }
            result
        }
        #[cfg(not(feature = "llm"))]
        { warn!("LLM feature not enabled, using heuristic classification"); Ok("{}".to_string()) }
    }

    pub(super) fn save_basic_file(&self, bot_id: Uuid, path: &str, content: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.state.broadcast_task_progress(
            TaskProgressEvent::new(&bot_id.to_string(), "save_basic_file", "Persisting BASIC to Drive")
                .with_details(path.to_string()),
        );
        let mut conn = self.pool.get().map_err(|e| format!("db pool: {e}"))?;
        #[derive(diesel::QueryableByName)]
        struct BotNameRow {
            #[diesel(sql_type = Text)]
            name: String,
        }
        let bot = sql_query("SELECT name FROM bots WHERE id = $1")
            .bind::<DieselUuid, _>(bot_id)
            .get_result::<BotNameRow>(&mut conn)
            .optional()
            .map_err(|e| format!("resolve bot: {e}"))?;
        let Some(bot) = bot else {
            return Err(format!("Bot not found for id {bot_id}").into());
        };
        // Reform #1505 — the bot's repository is the canonical `.gbdialog`, so a
        // generated command is committed there (Drive only for a bot without a
        // git project, where nothing else would compile it).
        let relative = path.trim_start_matches('/');
        if let Some(sources) = self.state.source_ops() {
            let files = vec![(relative.to_string(), content.to_string())];
            sources
                .write_sources(bot_id, &files, &format!("autotask: add {relative}"))
                .map_err(|e| format!("git source write failed: {e}"))?;
            info!("Committed generated BASIC to ALM: {}/{}", bot.name, relative);
            return Ok(());
        }
        let Some(ops) = self.state.file_ops() else {
            return Err("Drive ops not available — cannot persist generated BASIC".into());
        };
        let location = botbasic_core::utils::bot_drive_location_for(&mut conn, bot_id);
        let bucket = location.bucket;
        let key = format!("{}{}.gbdialog/{relative}", location.bot_prefix, bot.name);
        ops.put_object(&bucket, &key, content.as_bytes().to_vec(), "text/plain")?;
        info!("Saved BASIC file to Drive: {bucket}/{key}");
        Ok(())
    }

    pub(super) fn store_classification(&self, classification: &ClassifiedIntent, session: &UserSession) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().ok();
        if let Some(ref mut conn) = conn {
            sql_query("INSERT INTO intent_classifications (id, bot_id, session_id, original_text, intent_type, confidence, entities, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, NOW()) ON CONFLICT DO NOTHING")
                .bind::<DieselUuid, _>(Uuid::parse_str(&classification.id).unwrap_or(Uuid::nil()))
                .bind::<DieselUuid, _>(session.bot_id)
                .bind::<DieselUuid, _>(session.id)
                .bind::<Text, _>(&classification.original_text)
                .bind::<Text, _>(&classification.intent_type.to_string())
                .bind::<Float8, _>(classification.confidence)
                .bind::<Text, _>(serde_json::to_string(&classification.entities).unwrap_or_default())
                .execute(conn).ok();
        }
        Ok(())
    }
}
