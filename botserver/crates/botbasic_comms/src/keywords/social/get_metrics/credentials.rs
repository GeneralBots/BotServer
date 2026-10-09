use super::*;

pub(super) fn get_platform_credentials(
    state: &dyn BasicRuntime,
    bot_id: Uuid,
    platform: &str,
) -> Result<Value, String> {
    use diesel::prelude::*;

    let mut conn = state.db_pool().get().map_err(|e| format!("DB error: {}", e))?;
    let key = format!("{}_credentials", platform);

    #[derive(QueryableByName)]
    struct SettingRow {
        #[diesel(sql_type = diesel::sql_types::Jsonb)]
        value: Value,
    }

    let query = diesel::sql_query("SELECT value FROM bot_settings WHERE bot_id = $1 AND key = $2")
        .bind::<diesel::sql_types::Uuid, _>(bot_id)
        .bind::<diesel::sql_types::Text, _>(&key);

    let result: Result<Vec<SettingRow>, _> = query.load(&mut *conn);

    match result {
        Ok(rows) if !rows.is_empty() => Ok(rows[0].value.clone()),
        _ => Err(format!("No {} credentials configured", platform)),
    }
}
