use super::*;

impl BasicCompiler {
    pub(super) fn preprocess_basic(
        &mut self,
        source: &str,
        source_path: &str,
        bot_id: Uuid,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        use botcore::shared::models::TriggerKind;
        use diesel::ExpressionMethods;
        use diesel::QueryDsl;
        use diesel::RunQueryDsl;
        use log::trace;
        use regex::Regex;

        let bot_uuid = bot_id;
        let mut result = String::new();

        let source = if goto_transform::has_goto_constructs(source) {
            trace!("GOTO constructs detected, transforming to state machine");
            goto_transform::transform_goto(source)
        } else {
            source.to_string()
        };
        let source = source.as_str();

        let source = syntax_transforms::preprocess_llm_keyword(source);
        let mut found_directives = std::collections::HashSet::new();
        let mut has_schedule = false;
        let script_name = std::path::Path::new(source_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        {
            let mut conn = self
                .state
                .conn
                .get()
                .map_err(|e| format!("Failed to get database connection: {e}"))?;
            use botcore::shared::schema::system_automations::dsl::*;
            diesel::delete(
                system_automations
                    .filter(bot_id.eq(&bot_uuid))
                    .filter(kind.eq(TriggerKind::Scheduled as i32))
                    .filter(param.eq(&script_name)),
            )
            .execute(&mut conn)
            .ok();
        }

        let website_regex =
            Regex::new(r#"(?i)USE\s+WEBSITE\s+"([^"]+)"(?:\s+REFRESH\s+"([^"]+)")?"#)?;

        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty()
                || trimmed.starts_with('\'')
                || trimmed.starts_with("//")
                || trimmed.starts_with("REM")
            {
                continue;
            }

            let normalized = trimmed
                .replace("FOR EACH", "FOR_EACH")
                .replace("EXIT FOR", "EXIT_FOR")
                .replace("GROUP BY", "GROUP_BY")
                .replace("ADD SUGGESTION TOOL", "ADD_SUGGESTION_TOOL")
                .replace("ADD SUGGESTION TEXT", "ADD_SUGGESTION_TEXT")
                .replace("ADD SUGGESTION", "ADD_SUGGESTION")
                .replace("ADD SWITCHER", "ADD_SWITCHER");
            if normalized.starts_with("SET SCHEDULE") || trimmed.starts_with("SET SCHEDULE") {
                has_schedule = true;
                found_directives.insert(basic_errors::Directive::SetSchedule);
                let parts: Vec<&str> = normalized.split('"').collect();
                if parts.len() >= 3 {
                    if let Some(ref cb) = self.callbacks.execute_set_schedule {
                        let cron = parts[1];
                        let mut conn = self
                            .state
                            .conn
                            .get()
                            .map_err(|e| format!("Failed to get database connection: {e}"))?;
                        if let Err(e) = (cb)(&mut conn, cron, &script_name, bot_id) {
                            log::error!(
                                "Failed to schedule SET SCHEDULE during preprocessing: {}",
                                e
                            );
                        }
                    }
                } else {
                    log::warn!("Malformed SET SCHEDULE line ignored: {}", trimmed);
                }
                continue;
            }

            // `ON EVENT "<event>"` is a design-time declaration, like WEBHOOK
            // and SET SCHEDULE: the tool being compiled IS the tool to run, so
            // the line only names the event and is consumed here.
            if normalized.starts_with("ON EVENT") {
                found_directives.insert(basic_errors::Directive::OnEvent);
                let parts: Vec<&str> = normalized.split('"').collect();
                if parts.len() >= 2 {
                    let event = parts[1].trim();
                    match self.callbacks.execute_on_event {
                        Some(ref cb) => {
                            let mut conn = self
                                .state
                                .conn
                                .get()
                                .map_err(|e| format!("Failed to get database connection: {e}"))?;
                            if let Err(e) = (cb)(&mut conn, event, &script_name, bot_id) {
                                log::error!(
                                    "Failed to subscribe script {script_name} to event {event}: {e}"
                                );
                            } else {
                                log::info!(
                                    "Script {script_name} subscribed to channel event {event}"
                                );
                            }
                        }
                        None => log::warn!(
                            "ON EVENT \"{event}\" in {script_name} ignored: no registration hook"
                        ),
                    }
                } else {
                    log::warn!("Malformed ON EVENT line ignored: {normalized}");
                }
                continue;
            }

            if normalized.starts_with("WEBHOOK") {
                found_directives.insert(basic_errors::Directive::Webhook);
                let parts: Vec<&str> = normalized.split('"').collect();
                if parts.len() >= 2 {
                    if let Some(ref cb) = self.callbacks.execute_webhook {
                        let endpoint = parts[1];
                        let mut conn = self
                            .state
                            .conn
                            .get()
                            .map_err(|e| format!("Failed to get database connection: {}", e))?;
                        if let Err(e) = (cb)(&mut conn, endpoint, &script_name, bot_id) {
                            log::error!("Failed to register WEBHOOK during preprocessing: {}", e);
                        } else {
                            log::trace!(
                                "Registered webhook endpoint {} for script {} during preprocessing",
                                endpoint,
                                script_name
                            );
                        }
                    }
                } else {
                    log::warn!("Malformed WEBHOOK line ignored: {}", normalized);
                }
                continue;
            }

            // Support: ON UPDATE OF "table" (uses current script as param)
            // Support: ON UPDATE OF "table" DO CALL "script" (uses DO CALL script as param)
            let on_update_base = r#"(?i)^ON\s+UPDATE\s+OF\s+"([^"]+)"\s*$"#;
            let on_update_do_call = r#"(?i)^ON\s+UPDATE\s+OF\s+"([^"]+)"\s+DO\s+CALL\s+"([^"]+)"\s*$"#;
            let do_call_re = Regex::new(on_update_do_call).ok();
            let base_re = Regex::new(on_update_base).ok();

            if do_call_re.as_ref().is_some_and(|re| re.is_match(&normalized)) {
                found_directives.insert(basic_errors::Directive::OnUpdateOf);
                if let Some(re) = do_call_re.as_ref().and_then(|r| r.captures(&normalized)) {
                    let table_name = re.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
                    let trigger_script = re.get(2).map(|m| m.as_str().to_string()).unwrap_or_else(|| script_name.clone());
                    if let Some(ref cb) = self.callbacks.execute_on_update {
                        let mut conn = self.state.conn.get().map_err(|e| format!("DB error: {e}"))?;
                        use botcore::shared::schema::system_automations;
                        if let Err(e) = diesel::delete(
                            system_automations::table
                                .filter(system_automations::bot_id.eq(&bot_uuid))
                                .filter(system_automations::target.eq(&table_name))
                                .filter(system_automations::kind.eq_any(vec![1i32, 2i32, 3i32])),
                        )
                        .execute(&mut conn)
                        {
                            log::error!("Failed to clean old ON UPDATE OF triggers: {e}");
                        }
                        let call_bot_id = bot_uuid;
                        if let Err(e) = (cb)(&mut conn, &table_name, &trigger_script, call_bot_id, 2) {
                            log::error!("Failed to register ON UPDATE OF DO CALL (INSERT) trigger: {e}");
                        }
                        if let Err(e) = (cb)(&mut conn, &table_name, &trigger_script, call_bot_id, 1) {
                            log::error!("Failed to register ON UPDATE OF DO CALL (UPDATE) trigger: {e}");
                        }
                        if let Err(e) = (cb)(&mut conn, &table_name, &trigger_script, call_bot_id, 3) {
                            log::error!("Failed to register ON UPDATE OF DO CALL (DELETE) trigger: {e}");
                        }
                    }
                }
                continue;
            }

            if base_re.as_ref().is_some_and(|re| re.is_match(&normalized)) {
                found_directives.insert(basic_errors::Directive::OnUpdateOf);
                let table_name = normalized
                    .split('"')
                    .nth(1)
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                if let Some(ref cb) = self.callbacks.execute_on_update {
                    let mut conn = self.state.conn.get().map_err(|e| format!("DB error: {e}"))?;
                    use botcore::shared::schema::system_automations;
                    if let Err(e) = diesel::delete(
                        system_automations::table
                            .filter(system_automations::bot_id.eq(&bot_uuid))
                            .filter(system_automations::target.eq(&table_name))
                            .filter(system_automations::kind.eq_any(vec![1i32, 2i32, 3i32])),
                    )
                    .execute(&mut conn)
                    {
                        log::error!("Failed to clean old ON UPDATE OF triggers: {e}");
                    }
                    let call_bot_id = bot_uuid;
                    if let Err(e) = (cb)(&mut conn, &table_name, &script_name, call_bot_id, 2) {
                        log::error!("Failed to register ON UPDATE OF (INSERT) trigger: {e}");
                    }
                    if let Err(e) = (cb)(&mut conn, &table_name, &script_name, call_bot_id, 1) {
                        log::error!("Failed to register ON UPDATE OF (UPDATE) trigger: {e}");
                    }
                    if let Err(e) = (cb)(&mut conn, &table_name, &script_name, call_bot_id, 3) {
                        log::error!("Failed to register ON UPDATE OF (DELETE) trigger: {e}");
                    }
                }
                continue;
            }

            if trimmed.to_uppercase().starts_with("USE WEBSITE") {
                found_directives.insert(basic_errors::Directive::UseWebsite);
                if let Some(caps) = website_regex.captures(&normalized) {
                    if let Some(url_match) = caps.get(1) {
                        let url = url_match.as_str();
                        let refresh = caps.get(2).map(|m| m.as_str()).unwrap_or("1m");
                        if let Some(ref cb) = self.callbacks.execute_use_website {
                            let mut conn = self
                                .state
                                .conn
                                .get()
                                .map_err(|e| format!("Failed to get database connection: {}", e))?;
                            if let Err(e) = (cb)(&mut conn, url, bot_id, refresh) {
                                log::error!("Failed to register USE_WEBSITE during preprocessing: {}", e);
                            } else {
                                log::trace!(
                                    "Registered website {} for crawling during preprocessing (refresh: {})",
                                    url,
                                    refresh
                                );
                            }
                        }
                        result.push_str(&format!("USE_WEBSITE(\"{}\", \"{}\");\n", url, refresh));
                        continue;
                    }
                } else {
                    log::warn!("Malformed USE_WEBSITE line ignored: {}", normalized);
                }
                continue;
            }
            if normalized.starts_with("PARAM ") || normalized.starts_with("DESCRIPTION ") {
                continue;
            }
            result.push_str(&normalized);
            result.push('\n');
        }
        if self.previous_schedules.contains(&script_name) && !has_schedule {
            let mut conn = self
                .state
                .conn
                .get()
                .map_err(|e| format!("Failed to get database connection: {}", e))?;
            use botcore::shared::schema::system_automations::dsl::*;
            diesel::delete(
                system_automations
                    .filter(bot_id.eq(&bot_uuid))
                    .filter(kind.eq(TriggerKind::Scheduled as i32))
                    .filter(param.eq(&script_name)),
            )
            .execute(&mut conn)
            .map_err(|e| log::error!("Failed to remove schedule for {}: {}", script_name, e))
            .ok();
        }
        if has_schedule {
            self.previous_schedules.insert(script_name.clone());
        } else {
            self.previous_schedules.remove(&script_name);
        }

        // Validate directive conflicts (e.g., SET SCHEDULE + ON UPDATE OF)
        if let Err(e) = basic_errors::validate_directives(&found_directives) {
            log::warn!("Conflict in '{}': {}", script_name, e);
        }

        let result = match self.convert_save_statements(&result, bot_id) {
            Ok(r) => r,
            Err(e) => {
                log::warn!("SAVE conversion failed: {}, using original code", e);
                result
            }
        };
        let result = blocks::convert_begin_blocks(&result);
        let result = syntax_transforms::convert_multiword_keywords(&result);
        let result = syntax_transforms::convert_while_wend_syntax(&result);
        let result = syntax_transforms::predeclare_variables(&result);
        let result = syntax_transforms::convert_if_then_syntax(&result);
        let result = syntax_transforms::convert_select_case_syntax(&result);
        let result = syntax_transforms::convert_keywords_to_lowercase(&result);

        Ok(result)
    }
}
