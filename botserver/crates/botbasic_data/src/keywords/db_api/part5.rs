use super::*;

pub async fn search_records_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(table): Path<String>,
    Json(payload): Json<SearchRequest>,
) -> impl IntoResponse {
    let table_name = sanitize_identifier(&table);
    let user_roles = user_roles_from_headers(&headers);
    let limit = payload.limit.unwrap_or(20).min(100);
    // The search term is bound via $1 (not inlined), so no SQL-string escaping
    // is needed; only LIKE wildcards are escaped below. The previous `''`
    // single-quote escaping double-escaped bound values (e.g. "O'Brien" became
    // "O''Brien" and never matched).

    let mut conn = match state.conn.get() {
        Ok(c) => c,
        Err(e) => {
            let sanitized = log_and_sanitize(&e, "search_records_db_connection", None);
            return (StatusCode::INTERNAL_SERVER_ERROR, sanitized).into_response()
        }
    };

    // Check table-level read access
    let access_info =
        match check_table_access(&mut conn, &table_name, &user_roles, AccessType::Read) {
            Ok(info) => info,
            Err(e) => {
                return (StatusCode::FORBIDDEN, Json(json!({ "error": e }))).into_response();
            }
        };

    let safe_search = payload.query.replace('%', "\\%").replace('_', "\\_");

    // Discover the table's text columns so search works on any table shape
    // (previously hardcoded title/name/description, which 500s on generic tables).
    #[derive(QueryableByName)]
    struct TextColRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        column_name: String,
    }

    let text_cols: Vec<String> = sql_query(
        "SELECT column_name FROM information_schema.columns \
         WHERE table_schema = 'public' AND table_name = $1 \
           AND data_type IN ('text', 'character varying', 'character', 'citext', 'name') \
         ORDER BY ordinal_position ASC",
    )
    .bind::<diesel::sql_types::Text, _>(&table_name)
    .load::<TextColRow>(&mut conn)
    .unwrap_or_default()
    .into_iter()
    .map(|r| r.column_name)
    .collect();

    if text_cols.is_empty() {
        // No searchable text columns — return an empty result instead of 500.
        return (StatusCode::OK, Json(json!({ "data": Vec::<Value>::new() }))).into_response();
    }

    let concat = text_cols
        .iter()
        .map(|c| format!("COALESCE(t.{c}::text, '')"))
        .collect::<Vec<_>>()
        .join(" || ' ' || ");

    let query = format!(
        "SELECT row_to_json(t.*) as data FROM {} t \
         WHERE {concat} ILIKE '%' || $1 || '%' LIMIT {}",
        table_name, limit
    );

    let rows: Result<Vec<JsonRow>, _> = sql_query(&query)
        .bind::<diesel::sql_types::Text, _>(&safe_search)
        .get_results(&mut conn);

    match rows {
        Ok(data) => {
            // Filter fields based on user roles
            let filtered_data: Vec<Value> = data
                .into_iter()
                .map(|r| filter_fields_by_role(r.data, &user_roles, &access_info))
                .collect();
            (StatusCode::OK, Json(json!({ "data": filtered_data }))).into_response()
        }
        Err(e) => {
            let sanitized = log_and_sanitize(&e, &format!("search_records_{}", table_name), None);
            (StatusCode::INTERNAL_SERVER_ERROR, sanitized)
                .into_response()
        }
    }
}
pub(super) fn value_to_sql(value: &Value) -> String {
    match value {
        Value::Null => "NULL".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        Value::Array(_) | Value::Object(_) => {
            format!("'{}'", value.to_string().replace('\'', "''"))
        }
    }
}
