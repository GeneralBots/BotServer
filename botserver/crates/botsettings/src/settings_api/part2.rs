use super::*;

pub fn read_pref(conn: &mut diesel::PgConnection, user_id: uuid::Uuid, key: &str) -> serde_json::Value {
    diesel::sql_query(
        "SELECT preference_value::text AS value FROM user_preferences WHERE user_id = $1 AND preference_key = $2",
    )
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .bind::<diesel::sql_types::Text, _>(key)
    .get_result::<TextRow>(conn)
    .ok()
    .and_then(|r| r.value)
    .and_then(|v| serde_json::from_str(&v).ok())
    .unwrap_or(serde_json::Value::Null)
}
pub fn write_pref(conn: &mut diesel::PgConnection, user_id: uuid::Uuid, key: &str, value: &serde_json::Value) {
    let json = value.to_string();
    let _ = diesel::sql_query(
        "INSERT INTO user_preferences (id, user_id, preference_key, preference_value, created_at, updated_at)
         VALUES ($1, $2, $3, $4::jsonb, NOW(), NOW())
         ON CONFLICT (user_id, preference_key)
         DO UPDATE SET preference_value = EXCLUDED.preference_value, updated_at = NOW()",
    )
    .bind::<diesel::sql_types::Uuid, _>(uuid::Uuid::new_v4())
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .bind::<diesel::sql_types::Text, _>(key)
    .bind::<diesel::sql_types::Text, _>(&json)
    .execute(conn);
}
