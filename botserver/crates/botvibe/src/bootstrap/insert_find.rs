use super::*;

/// Insert a `bots` row. `ON CONFLICT (slug) DO NOTHING` keeps another
/// branch's bot untouchable.
/// The `bots` row to create.
pub(super) struct NewBotRow<'a> {
    pub conn: &'a mut PgConnection,
    pub id: Uuid,
    pub name: &'a str,
    pub slug: &'a str,
    pub description: &'a str,
    pub org_id: Uuid,
    pub branch_id: Uuid,
    pub database_name: &'a str,
}

pub(super) fn insert_bot_row(row: NewBotRow<'_>) -> Result<(), String> {
    let NewBotRow {
        conn,
        id,
        name,
        slug,
        description,
        org_id,
        branch_id,
        database_name,
    } = row;
    diesel::sql_query(
        "INSERT INTO bots (id, name, slug, description, org_id, branch_id, \
             database_name, llm_provider, llm_config, context_provider, context_config, \
             is_active, is_public, origin, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'openai', '{}'::jsonb, 'openai', '{}'::jsonb, \
             true, true, 'vibe', NOW(), NOW()) \
         ON CONFLICT (slug) DO NOTHING",
    )
    .bind::<diesel::sql_types::Uuid, _>(id)
    .bind::<diesel::sql_types::Text, _>(name)
    .bind::<diesel::sql_types::Text, _>(slug)
    .bind::<diesel::sql_types::Text, _>(description)
    .bind::<diesel::sql_types::Uuid, _>(org_id)
    .bind::<diesel::sql_types::Uuid, _>(branch_id)
    .bind::<diesel::sql_types::Text, _>(database_name)
    .execute(conn)
    .map_err(|e| format!("insert bots row {slug}: {e}"))?;
    Ok(())
}

/// Look up the branch's bot for `slug-or-name` (returns its id).
///
/// The match is case-insensitive: `bot_slug` lowercases the project name while
/// Drive-discovered bots keep the case of their bucket name (`PragmatismoGB`).
/// A case-sensitive comparison missed those rows and made `ensure_bot_rows`
/// create a duplicate Vibe bot instead of adopting the branch's real one.
pub(super) fn find_branch_bot(
    conn: &mut PgConnection,
    branch_id: Uuid,
    slug_or_name: &str,
) -> Option<BotIdRow> {
    diesel::sql_query(
        "SELECT id FROM bots WHERE branch_id = $1 \
         AND (lower(slug) = lower($2) OR lower(name) = lower($2)) \
         ORDER BY created_at ASC LIMIT 1",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch_id)
    .bind::<diesel::sql_types::Text, _>(slug_or_name)
    .get_result::<BotIdRow>(conn)
    .optional()
    .ok()
    .flatten()
}
