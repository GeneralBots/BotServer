use super::*;

/// Resolve the bot identity so generated scripts land on the right Drive
/// bucket (`{bot}.gbai/{bot}.gbdialog/`).
pub(crate) fn resolve_bot_info(pool: &DbPool, bot_id: Uuid) -> Result<Option<BotInfo>, String> {
    let mut conn = pool.get().map_err(|e| format!("db pool: {e}"))?;
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
    Ok(bot.map(|b| BotInfo {
        id: bot_id,
        name: b.name,
        drive: botbasic_core::utils::bot_drive_location_for(&mut conn, bot_id),
    }))
}
/// Parse an optional `bot_id` string from a request body; defaults to nil.
pub(crate) fn canonical_bot_id(bot_id: Option<String>) -> Uuid {
    bot_id
        .and_then(|s| Uuid::parse_str(&s).ok())
        .unwrap_or_else(Uuid::nil)
}
/// Resolves a nil bot id to the default bot so chat-driven requests without
/// an explicit bot still persist to a real bucket (vibe chat, autotask).
pub(super) fn resolve_effective_bot_id(pool: &DbPool, bot_id: Uuid) -> Uuid {
    if bot_id != Uuid::nil() {
        return bot_id;
    }
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return bot_id,
    };
    sql_query("SELECT id FROM bots WHERE name = 'default' AND is_active = true LIMIT 1")
        .get_result::<BotIdRow>(&mut conn)
        .optional()
        .ok()
        .flatten()
        .map(|r| r.id)
        .unwrap_or(bot_id)
}
