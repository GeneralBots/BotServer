use super::*;

pub async fn insert_row(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<ApiResult>, (StatusCode, Json<serde_json::Value>)> {
    let bot_id = extract_bot_id(&headers)?;
    let safe_name = sanitize_identifier(&name);
    if safe_name != name {
        return Err(error_response("Invalid table name"));
    }

    let mut conn = get_bot_conn_for_env(&state, bot_id, &headers)?;

    let obj = match payload {
        serde_json::Value::Object(map) => {
            if map.contains_key("data") {
                map.get("data")
                    .and_then(|d| d.as_object())
                    .cloned()
                    .ok_or_else(|| error_response("Request body must be a JSON object"))?
            } else {
                map
            }
        }
        _ => return Err(error_response("Request body must be a JSON object")),
    };

    if obj.is_empty() {
        return Err(error_response("Cannot insert empty row"));
    }

    let columns: Vec<String> = obj.keys().map(|k| sanitize_identifier(k)).collect();
    let values: Vec<String> = obj
        .values()
        .map(|v| match v {
            serde_json::Value::String(s) => {
                format!("'{}'", s.replace('\'', "''"))
            }
            serde_json::Value::Null => "NULL".to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => {
                if *b { "TRUE".to_string() } else { "FALSE".to_string() }
            }
            other => format!("'{}'", other.to_string().replace('\'', "''")),
        })
        .collect();

    let col_list = columns.join(", ");
    let val_list = values.join(", ");

    let sql = format!("INSERT INTO {safe_name} ({col_list}) VALUES ({val_list})");

    diesel::sql_query(sql)
        .execute(&mut conn)
        .map_err(|e| internal_error(&format!("Insert failed: {e}")))?;

    Ok(Json(ApiResult {
        success: true,
        message: format!("Row inserted into {safe_name}"),
    }))
}
pub async fn delete_row(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((name, id)): Path<(String, String)>,
) -> Result<Json<ApiResult>, (StatusCode, Json<serde_json::Value>)> {
    let bot_id = extract_bot_id(&headers)?;
    let safe_name = sanitize_identifier(&name);
    if safe_name != name {
        return Err(error_response("Invalid table name"));
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
    let safe_id = id.replace('\'', "''");

    let affected = diesel::sql_query(format!(
        "DELETE FROM {safe_name} WHERE {pk_col} = '{safe_id}'"
    ))
    .execute(&mut conn)
    .map_err(|e| internal_error(&format!("Delete failed: {e}")))?;

    if affected == 0 {
        return Err(error_response("Row not found"));
    }

    Ok(Json(ApiResult {
        success: true,
        message: format!("Row deleted from {safe_name}"),
    }))
}
pub async fn create_table(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<CreateTableRequest>,
) -> Result<Json<ApiResult>, (StatusCode, Json<serde_json::Value>)> {
    let bot_id = extract_bot_id(&headers)?;
    let table_name = payload.table_name.clone().unwrap_or_else(|| payload.name.clone());
    let safe_name = sanitize_identifier(&table_name);
    if safe_name != table_name || safe_name.is_empty() {
        return Err(error_response("Invalid table name"));
    }

    let mut conn = get_bot_conn_for_env(&state, bot_id, &headers)?;

    if payload.columns.is_empty() {
        return Err(error_response("At least one column is required"));
    }

    let col_defs: Vec<String> = payload
        .columns
        .iter()
        .map(|col| {
            let col_name = sanitize_identifier(&col.name);
            let data_type = col
                .data_type
                .clone()
                .or_else(|| col.col_type.clone())
                .ok_or_else(|| error_response("Missing column data type"))?;
            let nullable = col
                .nullable
                .or_else(|| col.not_null.map(|n| !n))
                .unwrap_or(true);
            let null_clause = if nullable { "" } else { " NOT NULL" };
            let pk_clause = if col.primary_key.unwrap_or(false) { " PRIMARY KEY" } else { "" };
            let default = column_default(col, &data_type);
            Ok::<String, (StatusCode, Json<serde_json::Value>)>(format!(
                "    {col_name} {data_type}{null_clause}{pk_clause}{default}"
            ))
        })
        .collect::<Result<Vec<String>, _>>()?;

    let sql = format!(
        "CREATE TABLE IF NOT EXISTS {safe_name} (\n{}\n)",
        col_defs.join(",\n")
    );

    diesel::sql_query(sql)
        .execute(&mut conn)
        .map_err(|e| internal_error(&format!("Create table failed: {e}")))?;

    Ok(Json(ApiResult {
        success: true,
        message: format!("Table {safe_name} created successfully"),
    }))
}
pub async fn alter_table(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(payload): Json<AlterTableRequest>,
) -> Result<Json<ApiResult>, (StatusCode, Json<serde_json::Value>)> {
    let bot_id = extract_bot_id(&headers)?;
    let safe_name = sanitize_identifier(&name);
    if safe_name != name {
        return Err(error_response("Invalid table name"));
    }

    let mut conn = get_bot_conn_for_env(&state, bot_id, &headers)?;
    let mut messages = Vec::new();

    if let Some(add_cols) = &payload.add_columns {
        for col in add_cols {
            let col_name = sanitize_identifier(&col.name);
            let data_type = col
                .data_type
                .clone()
                .or_else(|| col.col_type.clone())
                .ok_or_else(|| error_response("Missing column data type"))?;
            let nullable = col
                .nullable
                .or_else(|| col.not_null.map(|n| !n))
                .unwrap_or(true);
            let null_clause = if nullable { "" } else { " NOT NULL" };
            let default = column_default(col, &data_type);

            let sql = format!(
                "ALTER TABLE {safe_name} ADD COLUMN IF NOT EXISTS {col_name} {data_type}{null_clause}{default}",
            );
            diesel::sql_query(sql)
                .execute(&mut conn)
                .map_err(|e| internal_error(&format!("Add column failed: {e}")))?;
            messages.push(format!("Added column {col_name}"));
        }
    }

    if let Some(drop_cols) = &payload.drop_columns {
        for col in drop_cols {
            let col_name = sanitize_identifier(col);
            let sql = format!("ALTER TABLE {safe_name} DROP COLUMN IF EXISTS {col_name}");
            diesel::sql_query(sql)
                .execute(&mut conn)
                .map_err(|e| internal_error(&format!("Drop column failed: {e}")))?;
            messages.push(format!("Dropped column {col_name}"));
        }
    }

    if let Some(new_name) = &payload.rename_to {
        let safe_new = sanitize_identifier(new_name);
        if safe_new != *new_name {
            return Err(error_response("Invalid new table name"));
        }
        let sql = format!("ALTER TABLE {safe_name} RENAME TO {safe_new}");
        diesel::sql_query(sql)
            .execute(&mut conn)
            .map_err(|e| internal_error(&format!("Rename table failed: {e}")))?;
        messages.push(format!("Renamed table to {safe_new}"));
    }

    if messages.is_empty() {
        return Err(error_response("No alter operations specified"));
    }

    Ok(Json(ApiResult {
        success: true,
        message: messages.join("; "),
    }))
}
pub async fn drop_table(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<Json<ApiResult>, (StatusCode, Json<serde_json::Value>)> {
    let bot_id = extract_bot_id(&headers)?;
    let safe_name = sanitize_identifier(&name);
    if safe_name != name {
        return Err(error_response("Invalid table name"));
    }

    let mut conn = get_bot_conn_for_env(&state, bot_id, &headers)?;

    diesel::sql_query(format!("DROP TABLE IF EXISTS {safe_name}"))
        .execute(&mut conn)
        .map_err(|e| internal_error(&format!("Drop table failed: {e}")))?;

    Ok(Json(ApiResult {
        success: true,
        message: format!("Table {safe_name} dropped"),
    }))
}
