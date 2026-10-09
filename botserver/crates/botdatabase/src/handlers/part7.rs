use super::*;

pub async fn batch_delete(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(payload): Json<BatchDeleteRequest>,
) -> Result<Json<ApiResult>, (StatusCode, Json<serde_json::Value>)> {
    let bot_id = extract_bot_id(&headers)?;
    let safe_name = sanitize_identifier(&name);
    if safe_name != name {
        return Err(error_response("Invalid table name"));
    }

    if payload.ids.is_empty() {
        return Err(error_response("No rows selected for deletion"));
    }

    let mut conn = get_bot_conn_for_env(&state, bot_id, &headers)?;

    let pk_info: Vec<PkInfo> = sql_query(
        "SELECT a.attrelid::regclass::text AS table_name,
                a.attname AS column_name
         FROM pg_index i
         JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
         WHERE i.indisprimary AND a.attrelid::regclass::text = $1",
    )
    .bind::<diesel::sql_types::Text, _>(&safe_name)
    .get_results(&mut conn)
    .map_err(|e| internal_error(&format!("Failed to query primary keys: {e}")))?;

    if pk_info.is_empty() {
        return Err(error_response("Table has no primary key"));
    }

    let pk_col = sanitize_identifier(&pk_info[0].column_name);
    let escaped_ids: Vec<String> = payload
        .ids
        .iter()
        .map(|id| format!("'{}'", id.replace('\'', "''")))
        .collect();

    let sql = format!(
        "DELETE FROM {safe_name} WHERE {pk_col} IN ({})",
        escaped_ids.join(", ")
    );

    let affected = diesel::sql_query(sql)
        .execute(&mut conn)
        .map_err(|e| internal_error(&format!("Batch delete failed: {e}")))?;

    Ok(Json(ApiResult {
        success: true,
        message: format!("{affected} row(s) deleted from {safe_name}"),
    }))
}
pub async fn get_foreign_keys(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<Json<Vec<ForeignKeyInfo>>, (StatusCode, Json<serde_json::Value>)> {
    let bot_id = extract_bot_id(&headers)?;
    let safe_name = sanitize_identifier(&name);
    if safe_name != name {
        return Err(error_response("Invalid table name"));
    }

    let mut conn = get_bot_conn_for_env(&state, bot_id, &headers)?;

    #[derive(QueryableByName, Debug)]
    struct FkRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        constraint_name: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        column_name: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        foreign_table: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        foreign_column: String,
    }

    let fks: Vec<FkRow> = sql_query(
        "SELECT tc.constraint_name::text,
                kcu.column_name::text,
                ccu.table_name::text AS foreign_table,
                ccu.column_name::text AS foreign_column
         FROM information_schema.table_constraints tc
         JOIN information_schema.key_column_usage kcu
           ON tc.constraint_name = kcu.constraint_name
         JOIN information_schema.constraint_column_usage ccu
           ON ccu.constraint_name = tc.constraint_name
         WHERE tc.constraint_type = 'FOREIGN KEY'
           AND tc.table_schema = 'public'
           AND tc.table_name = $1",
    )
    .bind::<diesel::sql_types::Text, _>(&safe_name)
    .get_results(&mut conn)
    .map_err(|e| internal_error(&format!("Failed to query foreign keys: {e}")))?;

    let result: Vec<ForeignKeyInfo> = fks
        .into_iter()
        .map(|fk| ForeignKeyInfo {
            constraint_name: fk.constraint_name,
            column_name: fk.column_name,
            foreign_table: fk.foreign_table,
            foreign_column: fk.foreign_column,
        })
        .collect();

    Ok(Json(result))
}
