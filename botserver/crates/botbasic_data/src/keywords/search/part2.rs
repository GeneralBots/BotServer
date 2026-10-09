use super::*;

/// Fallback simple search using ILIKE
pub(super) fn execute_simple_search(
    conn: &mut diesel::PgConnection,
    safe_table: &str,
    safe_query: &str,
    limit: i32,
    searchable_columns: &[String],
) -> Result<Value, String> {
    let search_columns: Vec<String> = searchable_columns
        .iter()
        .map(|col| format!("COALESCE({}::text, '')", col))
        .collect();

    let combined_columns = search_columns.join(" || ' ' || ");

    let query = format!(
        r#"
        SELECT row_to_json(t)::text as row_data
        FROM {safe_table} t
        WHERE {combined_columns} ILIKE '%' || $1 || '%'
        LIMIT $2
        "#,
        safe_table = safe_table,
        combined_columns = combined_columns
    );

    let raw_results: Vec<JsonRow> = diesel::sql_query(&query)
        .bind::<Text, _>(safe_query)
        .bind::<Integer, _>(limit)
        .load(conn)
        .map_err(|e| {
            log::error!("Simple search error: {e}");
            e.to_string()
        })?;

    let results: Vec<Value> = raw_results
        .into_iter()
        .filter_map(|row| serde_json::from_str(&row.row_data).ok())
        .collect();

    Ok(json!({
        "command": "search",
        "table": safe_table,
        "query": safe_query,
        "results": results,
        "count": results.len()
    }))
}
/// Execute autocomplete query for quick suggestions
pub fn execute_autocomplete(
    conn: &mut diesel::PgConnection,
    table_str: &str,
    prefix: &str,
    limit: i32,
) -> Result<Vec<String>, String> {
    trace!("Autocomplete: table={table_str}, prefix={prefix}");

    let safe_table = sanitize_identifier(table_str);
    let safe_prefix = prefix.replace('\'', "''").replace('\\', "\\\\");

    // Find the primary text column (name, title, or first text column)
    let text_column = get_primary_text_column(conn, &safe_table)?;

    let query = format!(
        r#"
        SELECT DISTINCT {text_column}::text as suggestion
        FROM {safe_table}
        WHERE {text_column}::text ILIKE $1 || '%'
        ORDER BY {text_column}
        LIMIT $2
        "#,
        text_column = text_column,
        safe_table = safe_table
    );

    #[derive(QueryableByName)]
    struct SuggestionRow {
        #[diesel(sql_type = Text)]
        suggestion: String,
    }

    let results: Vec<SuggestionRow> = diesel::sql_query(&query)
        .bind::<Text, _>(&safe_prefix)
        .bind::<Integer, _>(limit)
        .load(conn)
        .map_err(|e| {
            log::error!("Autocomplete error: {e}");
            e.to_string()
        })?;

    Ok(results.into_iter().map(|r| r.suggestion).collect())
}
/// Get list of text/searchable columns from a table
pub(super) fn get_searchable_columns(conn: &mut diesel::PgConnection, table_name: &str) -> Result<Vec<String>, String> {
    #[derive(QueryableByName)]
    struct ColumnInfo {
        #[diesel(sql_type = Text)]
        column_name: String,
    }

    let query = r#"
        SELECT column_name::text
        FROM information_schema.columns
        WHERE table_name = $1
        AND data_type IN ('character varying', 'varchar', 'text', 'character', 'char', 'name')
        AND column_name NOT LIKE '%password%'
        AND column_name NOT LIKE '%secret%'
        AND column_name NOT LIKE '%token%'
        ORDER BY ordinal_position
    "#;

    let columns: Vec<ColumnInfo> = diesel::sql_query(query)
        .bind::<Text, _>(table_name)
        .load(conn)
        .map_err(|e| e.to_string())?;

    // Prioritize common search columns
    let priority_columns = ["name", "title", "description", "sku", "product_name", "productname"];
    let mut result: Vec<String> = Vec::new();

    // Add priority columns first
    for col in &priority_columns {
        if columns.iter().any(|c| c.column_name.to_lowercase() == *col) {
            result.push(col.to_string());
        }
    }

    // Add remaining columns
    for col in columns {
        let col_lower = col.column_name.to_lowercase();
        if !result.contains(&col_lower) && !result.contains(&col.column_name) {
            result.push(col.column_name);
        }
    }

    // Limit to first 5 searchable columns for performance
    result.truncate(5);

    Ok(result)
}
/// Get the primary text column for autocomplete (usually name or title)
pub(super) fn get_primary_text_column(conn: &mut diesel::PgConnection, table_name: &str) -> Result<String, String> {
    let columns = get_searchable_columns(conn, table_name)?;

    // Prefer specific column names
    let preferred = ["name", "title", "productname", "product_name", "label", "sku"];

    for pref in &preferred {
        if columns.iter().any(|c| c.to_lowercase() == *pref) {
            return Ok(pref.to_string());
        }
    }

    // Return first available text column
    columns
        .into_iter()
        .next()
        .ok_or_else(|| "No text columns found".to_string())
}
