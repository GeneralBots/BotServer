use super::*;

pub(super) fn resolve_from_scope_row(row: BotDriveScopeRow) -> BotDriveLocation {
    resolve_bot_drive_location(
        &row.name,
        row.branch_slug.as_deref(),
        row.org_slug.as_deref(),
    )
}
/// Looks up a bot's name, branch and org slug, then resolves its Drive location.
///
/// This is the single replacement for the `format!("{bot_name}.gbai")` pattern
/// that every file keyword used: neither the bucket nor the workspace prefix
/// can be derived from the bot name alone, because an org-hosted bot lives in
/// `{slug}.gborg` under its branch's `{branch}.gbai/` workspace. Falls back to
/// the standalone layout when the bot has no org or the lookup fails, so a
/// transient DB error degrades to the legacy path instead of writing nowhere.
pub fn bot_drive_location_for(
    conn: &mut diesel::r2d2::PooledConnection<
        diesel::r2d2::ConnectionManager<diesel::PgConnection>,
    >,
    bot_id: uuid::Uuid,
) -> BotDriveLocation {
    use diesel::prelude::*;

    let row = diesel::sql_query(format!("{BOT_DRIVE_SCOPE_SELECT} WHERE b.id = $1 LIMIT 1"))
        .bind::<diesel::sql_types::Uuid, _>(bot_id)
        .get_result::<BotDriveScopeRow>(conn);

    match row {
        Ok(row) => resolve_from_scope_row(row),
        Err(e) => {
            // Without the bot name no correct key can be built, so surface it:
            // silently writing to a wrong key is how files become unreachable.
            log::error!("bot_drive_location_for: failed to resolve bot {bot_id}: {e}");
            resolve_bot_drive_location("", None, None)
        }
    }
}
/// Same as [`bot_drive_location_for`], for callers that only know the bot
/// *name* (URL path segments such as the media player's `/{bot}/{path}`).
///
/// The name is resolved to the bot row first, so the org slug — and therefore
/// the `{slug}.gborg` bucket and `{branch}.gbai/` workspace — is still honoured.
pub fn bot_drive_location_for_name(
    conn: &mut diesel::r2d2::PooledConnection<
        diesel::r2d2::ConnectionManager<diesel::PgConnection>,
    >,
    bot_name: &str,
) -> BotDriveLocation {
    use diesel::prelude::*;

    let row = diesel::sql_query(format!("{BOT_DRIVE_SCOPE_SELECT} WHERE b.name = $1 LIMIT 1"))
        .bind::<diesel::sql_types::Text, _>(bot_name)
        .get_result::<BotDriveScopeRow>(conn);

    match row {
        Ok(row) => resolve_from_scope_row(row),
        Err(e) => {
            log::error!("bot_drive_location_for_name: failed to resolve bot '{bot_name}': {e}");
            resolve_bot_drive_location(bot_name, None, None)
        }
    }
}
/// Build an absolute bot path with org isolation.
/// Returns: "{work_root}/{org_id}.gborg/{bot_bucket}.gbai/{sub_path}"
pub fn build_absolute_bot_path(
    work_root: &str,
    org_id: impl std::fmt::Display,
    bot_bucket: &str,
    sub_path: &str,
) -> String {
    format!("{work_root}/{org_id}.gborg/{bot_bucket}.gbai/{sub_path}")
}
/// Get work path with org isolation suffix.
/// Returns: "{work_path}/{org_id}.gborg/"
pub fn get_org_work_path(org_id: impl std::fmt::Display) -> String {
    format!("{}/{org_id}.gborg/", get_work_path())
}
pub fn get_content_type(path: &str) -> String {
    let p = PathBuf::from(path);
    match p.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "txt" => "text/plain",
        "csv" => "text/csv",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }.to_string()
}
pub fn get_default_bot(state: &std::sync::Arc<dyn botbasic_types::BasicRuntime>) -> Result<serde_json::Value, String> {
    let mut conn = state.db_pool().get().map_err(|e| e.to_string())?;
    use diesel::prelude::*;
    use diesel::sql_query;
    use diesel::sql_types::Text;
    #[derive(diesel::QueryableByName)]
    struct BotRow {
        #[diesel(sql_type = Text)]
        bot_id: String,
    }
    let rows = sql_query("SELECT bot_id FROM bots LIMIT 1")
        .load::<BotRow>(&mut conn)
        .map_err(|e| e.to_string())?;
    rows.into_iter()
        .next()
        .map(|r| serde_json::json!({"bot_id": r.bot_id}))
        .ok_or_else(|| "No bots found".to_string())
}
pub fn parse_filter(filter_str: &str) -> Result<(String, Vec<String>), Box<dyn std::error::Error + Send + Sync>> {
    let trimmed = filter_str.trim();
    if trimmed == "1=1" {
        return Ok(("TRUE".to_string(), vec![]));
    }
    let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err("Invalid filter format. Expected 'KEY=VALUE'".into());
    }
    let column = parts[0].trim();
    let value = parts[1].trim();
    if !column.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("Invalid column name in filter".into());
    }
    Ok((format!("{} = $1", column), vec![value.to_string()]))
}
