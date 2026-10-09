use super::*;

pub(super) fn delegate_to_bot(state: &dyn BasicRuntime, session_id: Uuid, bot_name: &str) -> Result<String, String> {
    let mut conn = state.db_pool().get().map_err(|e| format!("DB error: {e}"))?;

    let bot_config: Option<BotConfigRow> = diesel::sql_query(
        "SELECT id, name, system_prompt, model_config FROM bots WHERE name = $1 AND is_active = true",
    )
    .bind::<diesel::sql_types::Text, _>(bot_name)
    .get_result(&mut *conn)
    .ok();

    let Some(config) = bot_config else {
        return Err(format!("Bot '{bot_name}' not found"));
    };

    trace!(
        "Delegating to bot: id={}, name={}, has_system_prompt={}, has_model_config={}",
        config.id,
        config.name,
        config.system_prompt.is_some(),
        config.model_config.is_some()
    );

    diesel::sql_query("UPDATE sessions SET delegated_to = $1, delegated_at = NOW() WHERE id = $2")
        .bind::<diesel::sql_types::Text, _>(&config.id)
        .bind::<diesel::sql_types::Text, _>(session_id.to_string())
        .execute(&mut *conn)
        .map_err(|e| format!("Failed to delegate: {e}"))?;

    let response = config.system_prompt.as_ref().map_or_else(
        || format!("Conversation delegated to '{}'", config.name),
        |prompt| {
            format!(
                "Conversation delegated to '{}' (specialized: {})",
                config.name,
                prompt.chars().take(50).collect::<String>()
            )
        },
    );

    Ok(response)
}
#[must_use]
pub fn match_bot_triggers(message: &str, bots: &[SessionBot]) -> Vec<SessionBot> {
    let message_lower = message.to_lowercase();
    let mut matching_bots = Vec::new();

    for bot in bots {
        if !bot.is_active {
            continue;
        }

        let matches = match bot.trigger.trigger_type {
            TriggerType::Keyword => bot.trigger.keywords.as_ref().is_some_and(|keywords| {
                keywords
                    .iter()
                    .any(|kw| message_lower.contains(&kw.to_lowercase()))
            }),
            TriggerType::Tool | TriggerType::Schedule | TriggerType::Event => false,
            TriggerType::Always => true,
        };

        if matches {
            matching_bots.push(bot.clone());
        }
    }

    matching_bots.sort_by_key(|b| std::cmp::Reverse(b.priority));
    matching_bots
}
#[must_use]
pub fn match_tool_triggers(tool_name: &str, bots: &[SessionBot]) -> Vec<SessionBot> {
    let tool_upper = tool_name.to_uppercase();
    let mut matching_bots = Vec::new();

    for bot in bots {
        if !bot.is_active {
            continue;
        }

        if bot.trigger.trigger_type == TriggerType::Tool {
            if let Some(tools) = &bot.trigger.tools {
                if tools.iter().any(|t| t.to_uppercase() == tool_upper) {
                    matching_bots.push(bot.clone());
                }
            }
        }
    }

    matching_bots.sort_by_key(|b| std::cmp::Reverse(b.priority));
    matching_bots
}
