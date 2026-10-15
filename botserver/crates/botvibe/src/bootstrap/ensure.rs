use super::*;

/// Ensure the two-environment bot pair for a bot-kind project (#1504):
/// PROD `{bot}` (adopt the branch's existing bot when present, otherwise
/// create) and TEST `{bot}-test` (always vibe-owned, `_dev` database).
/// `payload.bot_id` records the adoption so downstream flows reuse it.
pub fn ensure_bot_rows(
    pool: &botcore::shared::utils::DbPool,
    bot: &BotRowRef,
    description: Option<&str>,
) {
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("vibe bootstrap: pool unavailable: {e}");
            return;
        }
    };
    if let Err(e) = ensure_bot_rows_conn(&mut conn, bot, description) {
        log::error!("vibe bootstrap: bot pair for project {}: {e}", bot.id);
    }
}

pub(crate) fn ensure_bot_rows_conn(
    conn: &mut PgConnection,
    bot: &BotRowRef,
    description: Option<&str>,
) -> Result<(), String> {
    let prod_slug = bot_slug(&bot.name);
    let test_slug = format!("{prod_slug}-test");
    let desc = description
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .unwrap_or("Vibe bot project");

    // PROD: adopt (preferred) or create.
    match find_branch_bot(conn, bot.branch_id, &prod_slug) {
        Some(row) => {
            let payload_patch = serde_json::json!({ "bot_id": row.id.to_string() });
            diesel::sql_query(
                "UPDATE vibe_projects SET payload = payload || $2::jsonb, updated_at = NOW() \
                 WHERE id = $1",
            )
            .bind::<diesel::sql_types::Uuid, _>(bot.id)
            .bind::<diesel::sql_types::Jsonb, _>(&payload_patch)
            .execute(conn)
            .map_err(|e| format!("link project {} to bot: {e}", bot.id))?;
            log::info!(
                "vibe bootstrap: project {} adopted bot {} (branch {})",
                bot.id,
                row.id,
                bot.branch_id
            );
        }
        None => {
            let id = Uuid::new_v4();
            let database_name =
                crate::project_db::project_database_name(bot.branch_id, &bot.name, "production");
            insert_bot_row(NewBotRow {
                conn,
                id,
                name: &bot.name,
                slug: &prod_slug,
                description: desc,
                org_id: bot.org_id,
                branch_id: bot.branch_id,
                database_name: &database_name,
            })?;
            log::info!(
                "vibe bootstrap: created PROD bot {prod_slug} (branch {})",
                bot.branch_id
            );
        }
    }

    // TEST twin: always fresh, never adopted, `_dev` database (#1386).
    if find_branch_bot(conn, bot.branch_id, &test_slug).is_none() {
        let id = Uuid::new_v4();
        let database_name =
            crate::project_db::project_database_name(bot.branch_id, &bot.name, "test");
        insert_bot_row(NewBotRow {
            conn,
            id,
            name: &format!("{}-test", bot.name),
            slug: &test_slug,
            description: &format!("Vibe TEST bot for {}", bot.name),
            org_id: bot.org_id,
            branch_id: bot.branch_id,
            database_name: &database_name,
        })?;
        log::info!(
            "vibe bootstrap: created TEST bot {test_slug} (branch {})",
            bot.branch_id
        );
    }
    Ok(())
}
