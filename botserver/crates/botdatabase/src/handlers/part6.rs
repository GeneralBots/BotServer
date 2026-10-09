use super::*;

pub async fn add_column(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(payload): Json<AddColumnRequest>,
) -> Result<Json<ApiResult>, (StatusCode, Json<serde_json::Value>)> {
    let bot_id = extract_bot_id(&headers)?;
    let safe_name = sanitize_identifier(&name);
    if safe_name != name {
        return Err(error_response("Invalid table name"));
    }

    let col_name = sanitize_identifier(&payload.name);
    if col_name != payload.name || col_name.is_empty() {
        return Err(error_response("Invalid column name"));
    }

    let data_type = payload
        .data_type
        .clone()
        .or_else(|| payload.col_type.clone())
        .ok_or_else(|| error_response("Missing column data type"))?;

    let mut conn = get_bot_conn_for_env(&state, bot_id, &headers)?;

    let nullable = payload
        .nullable
        .or_else(|| payload.not_null.map(|n| !n))
        .unwrap_or(true);
    let null_clause = if nullable { "" } else { " NOT NULL" };
    let default = payload
        .default
        .as_ref()
        .map(|d| format!(" DEFAULT {d}"))
        .unwrap_or_default();

    let sql = format!(
        "ALTER TABLE {safe_name} ADD COLUMN IF NOT EXISTS {col_name} {data_type}{null_clause}{default}",
    );

    diesel::sql_query(sql)
        .execute(&mut conn)
        .map_err(|e| internal_error(&format!("Add column failed: {e}")))?;

    Ok(Json(ApiResult {
        success: true,
        message: format!("Column {col_name} added to {safe_name}"),
    }))
}
pub async fn update_row(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(payload): Json<UpdateRowRequest>,
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
    let safe_pk = payload.pk_value.replace('\'', "''");

    let obj = payload
        .data
        .as_object()
        .ok_or_else(|| error_response("Request body must be a JSON object"))?;

    if obj.is_empty() {
        return Err(error_response("No columns to update"));
    }

    let set_clauses: Vec<String> = obj
        .iter()
        .map(|(k, v)| {
            let col = sanitize_identifier(k);
            let val = match v {
                serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                serde_json::Value::Null => "NULL".to_string(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => {
                    if *b { "TRUE".to_string() } else { "FALSE".to_string() }
                }
                other => format!("'{}'", other.to_string().replace('\'', "''")),
            };
            format!("{col} = {val}")
        })
        .collect();

    let sql = format!(
        "UPDATE {safe_name} SET {} WHERE {pk_col} = '{safe_pk}'",
        set_clauses.join(", ")
    );

    let affected = diesel::sql_query(sql)
        .execute(&mut conn)
        .map_err(|e| internal_error(&format!("Update failed: {e}")))?;

    if affected == 0 {
        return Err(error_response("Row not found"));
    }

    Ok(Json(ApiResult {
        success: true,
        message: format!("Row updated in {safe_name}"),
    }))
}
pub(super) fn json_literal(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => {
            if *b { "TRUE".to_string() } else { "FALSE".to_string() }
        }
        other => format!("'{}'", other.to_string().replace('\'', "''")),
    }
}
pub async fn update_row_by_pk(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((name, id)): Path<(String, String)>,
    Json(payload): Json<UpdateCellRequest>,
) -> Result<Json<ApiResult>, (StatusCode, Json<serde_json::Value>)> {
    let bot_id = extract_bot_id(&headers)?;
    let safe_name = sanitize_identifier(&name);
    if safe_name != name {
        return Err(error_response("Invalid table name"));
    }

    let column = sanitize_identifier(&payload.column);
    if column != payload.column || column.is_empty() {
        return Err(error_response("Invalid column name"));
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
    let value = payload.value.as_ref().map(json_literal).unwrap_or_else(|| "NULL".to_string());

    let sql = format!(
        "UPDATE {safe_name} SET {column} = {value} WHERE {pk_col} = '{safe_id}'"
    );

    let affected = diesel::sql_query(sql)
        .execute(&mut conn)
        .map_err(|e| internal_error(&format!("Update failed: {e}")))?;

    if affected == 0 {
        return Err(error_response("Row not found"));
    }

    Ok(Json(ApiResult {
        success: true,
        message: format!("Row updated in {safe_name}"),
    }))
}
pub async fn export_table_csv(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<axum::response::Response, (StatusCode, Json<serde_json::Value>)> {
    let bot_id = extract_bot_id(&headers)?;
    let safe_name = sanitize_identifier(&name);
    if safe_name != name {
        return Err(error_response("Invalid table name"));
    }

    let db_url = get_bot_database_url(&state, bot_id).await?;
    let query = format!("SELECT * FROM {safe_name}");

    let result = tokio::task::spawn_blocking(move || execute_via_postgres_sync(&db_url, &query))
        .await
        .map_err(|e| internal_error(&format!("Task join error: {e}")))
        .map_err(|e: (StatusCode, Json<serde_json::Value>)| e)?
        .map_err(|e| internal_error(&format!("Export failed: {e}")))?;

    let (columns, rows) = result;

    let mut csv = String::new();
    csv.push_str(&columns.join(","));
    csv.push('\n');

    for row in rows {
        let fields: Vec<String> = columns
            .iter()
            .map(|col| {
                let value = row
                    .get(col)
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                csv_escape(&value)
            })
            .collect();
        csv.push_str(&fields.join(","));
        csv.push('\n');
    }

    Ok(axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/csv")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{safe_name}.csv\""),
        )
        .body(axum::body::Body::from(csv))
        .map_err(|e| internal_error(&format!("Export failed: {e}")))?)
}
pub(super) fn csv_escape(value: &serde_json::Value) -> String {
    let text = match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        other => other.to_string(),
    };
    if text.contains(',') || text.contains('"') || text.contains('\n') || text.contains('\r') {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text
    }
}
