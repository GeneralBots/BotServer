use super::*;

/// Process-lifetime cache of idle per-bot postgres connections, so the SQL
/// runner reuses a connection instead of opening a fresh TCP connection
/// (plus auth handshake) for every query. Keyed by the resolved per-bot
/// database URL, which is stable within a process lifetime. `postgres::Client`
/// is `Send` but not `Sync`, so connections are moved in/out of the stash
/// rather than shared.
pub(super) fn postgres_pool_stash() -> &'static std::sync::Mutex<
    std::collections::HashMap<String, Vec<postgres::Client>>,
> {
    static STASH: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<String, Vec<postgres::Client>>>,
    > = std::sync::OnceLock::new();
    STASH.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}
pub(super) fn checkout_postgres_client(db_url: &str) -> Option<postgres::Client> {
    postgres_pool_stash()
        .lock()
        .ok()
        .and_then(|mut stash| stash.get_mut(db_url).and_then(|conns| conns.pop()))
}
pub(super) fn checkin_postgres_client(db_url: &str, client: postgres::Client) {
    if let Ok(mut stash) = postgres_pool_stash().lock() {
        let conns = stash.entry(db_url.to_string()).or_default();
        // Cap idle connections per database to avoid unbounded growth.
        if conns.len() < 8 {
            conns.push(client);
        }
    }
}
pub(super) fn execute_via_postgres_sync(
    db_url: &str,
    query: &str,
) -> Result<(Vec<String>, Vec<serde_json::Value>), String> {
    let mut client = match checkout_postgres_client(db_url) {
        Some(c) => c,
        None => postgres::Client::connect(db_url, postgres::NoTls)
            .map_err(|e| format!("Connection failed: {e}"))?,
    };

    // Use the simple (text) query protocol: every value arrives as a String,
    // so NUMERIC/arrays/enums and any other unmapped type degrade to text
    // instead of a binary deserialization panic. This is the only panic-free
    // path for a generic SQL runner over an arbitrary schema.
    let result = client
        .simple_query(query)
        .map_err(|e| format!("Query failed: {e}"))
        .map(|messages| {
            let mut columns: Vec<String> = Vec::new();
            for msg in &messages {
                if let postgres::SimpleQueryMessage::RowDescription(cols) = msg {
                    columns = cols.iter().map(|c| c.name().to_string()).collect();
                    break;
                }
            }

            let mut rows: Vec<serde_json::Value> = Vec::new();
            for msg in &messages {
                if let postgres::SimpleQueryMessage::Row(row) = msg {
                    let mut obj = serde_json::Map::new();
                    for (i, col) in columns.iter().enumerate() {
                        let value = row.try_get(i).unwrap_or(None).map(str::to_string);
                        obj.insert(col.clone(), parse_cell(&value));
                    }
                    rows.push(serde_json::Value::Object(obj));
                }
            }

            (columns, rows)
        });

    // A successful query means the connection is still healthy — return it to
    // the idle stash for reuse. On error the connection state is unknown, so
    // it is dropped (a fresh one is opened on the next request).
    match result {
        Ok(pair) => {
            checkin_postgres_client(db_url, client);
            Ok(pair)
        }
        Err(e) => Err(e),
    }
}
pub(super) async fn execute_via_postgres(
    state: Arc<AppState>,
    bot_id: &uuid::Uuid,
    query: &str,
    page: i64,
    page_size: i64,
    total_rows: i64,
    pk_column: Option<String>,
    col_names: Vec<String>,
) -> Result<Json<TableDataResponse>, (StatusCode, Json<serde_json::Value>)> {
    let db_url = get_bot_database_url(&state, *bot_id).await?;
    let query = query.to_string();

    let result = tokio::task::spawn_blocking(move || execute_via_postgres_sync(&db_url, &query))
        .await
        .map_err(|e| internal_error(&format!("Task join error: {e}")))
        .map_err(|e: (StatusCode, Json<serde_json::Value>)| e)?;

    match result {
        Ok((_columns, rows)) => {
            // The query aliases each column as c1..cN (text-cast); remap the
            // keys back to the real column names so the grid can address cells
            // and the PK column by name. Without this, the frontend could not
            // find `row[pk_column]` and fell back to the row index (0, 1, ...),
            // producing UPDATE ... WHERE id = '0' (500 on UUID PKs) (#906).
            let rows: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|row| {
                    let mut obj = serde_json::Map::new();
                    if let serde_json::Value::Object(map) = row {
                        for (i, col) in col_names.iter().enumerate() {
                            let key = format!("c{}", i + 1);
                            let value = map
                                .get(&key)
                                .cloned()
                                .unwrap_or(serde_json::Value::Null);
                            obj.insert(col.clone(), value);
                        }
                    }
                    serde_json::Value::Object(obj)
                })
                .collect();

            Ok(Json(TableDataResponse {
                columns: col_names,
                rows,
                total: total_rows,
                total_rows,
                page,
                page_size,
                pk_column,
            }))
        }
        Err(e) => Err(internal_error(&e)),
    }
}
/// Removes leading SQL comments (`-- ...` and `/* ... */`) so the statement
/// classifier inspects the real first keyword instead of being spoofed by a
/// comment prefix (e.g. `/* x */ DROP TABLE users`).
pub(super) fn strip_leading_comments(sql: &str) -> &str {
    let mut s = sql.trim_start();
    loop {
        if s.starts_with("--") {
            match s.find('\n') {
                Some(i) => s = s[i + 1..].trim_start(),
                None => return "",
            }
        } else if s.starts_with("/*") {
            match s.find("*/") {
                Some(i) => s = s[i + 2..].trim_start(),
                None => return "",
            }
        } else {
            return s;
        }
    }
}
/// Returns true when the (lowercased, comment-stripped) query wraps a DML/DDL
/// statement inside a `WITH` common table expression (e.g. a data-modifying CTE).
pub(super) fn contains_dml_keyword(sql: &str) -> bool {
    ["insert", "update", "delete", "drop", "alter", "create", "truncate"]
        .iter()
        .any(|kw| sql.split_whitespace().any(|w| w == *kw))
}
pub async fn execute_query(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<QueryRequest>,
) -> Result<Json<QueryResult>, (StatusCode, Json<serde_json::Value>)> {
    let bot_id = extract_bot_id(&headers)?;

    let stripped = strip_leading_comments(&payload.query);
    let trimmed = stripped.trim().to_lowercase();
    if trimmed.is_empty() {
        return Err(error_response("Query cannot be empty"));
    }

    // Reject multiple statements: a single trailing semicolon is allowed, but
    // any content after a semicolon (a second statement) is rejected.
    let body = trimmed.trim_end();
    if let Some(idx) = body.find(';') {
        if !body[idx + 1..].trim().is_empty() {
            return Err(error_response("Multiple statements not allowed"));
        }
    }

    let first_word = trimmed.split_whitespace().next().unwrap_or("");
    let is_mutation = matches!(
        first_word,
        "insert" | "update" | "delete" | "drop" | "alter" | "create" | "truncate"
    ) || (first_word == "with" && contains_dml_keyword(&trimmed));

    let start = std::time::Instant::now();

    if is_mutation {
        let mut conn = get_bot_conn_for_env(&state, bot_id, &headers)?;
        let affected = diesel::sql_query(&payload.query)
            .execute(&mut conn)
            .map_err(|e| internal_error(&format!("Query execution failed: {e}")))?;

        let duration = start.elapsed().as_millis();
        return Ok(Json(QueryResult {
            columns: Vec::new(),
            rows: Vec::new(),
            row_count: affected as i64,
            is_mutation: true,
            duration_ms: duration,
        }));
    }

    let limited_query = if trimmed.starts_with("select") && !trimmed.contains("limit") {
        format!(
            "{} LIMIT {MAX_QUERY_ROWS}",
            payload.query.trim_end_matches(';')
        )
    } else {
        payload.query.clone()
    };

    let db_url = get_bot_database_url(&state, bot_id).await?;
    let query_clone = limited_query.clone();

    let pg_result = tokio::task::spawn_blocking(move || execute_via_postgres_sync(&db_url, &query_clone))
        .await
        .map_err(|e| internal_error(&format!("Task join error: {e}")))?;

    let duration = start.elapsed().as_millis();

    match pg_result {
        Ok((columns, rows)) => Ok(Json(QueryResult {
            row_count: rows.len() as i64,
            columns,
            rows,
            is_mutation: false,
            duration_ms: duration,
        })),
        Err(e) => Err(internal_error(&format!("Query failed: {e}"))),
    }
}
