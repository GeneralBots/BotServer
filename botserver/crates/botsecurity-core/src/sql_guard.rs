use std::collections::HashSet;
use std::sync::LazyLock;

/// System tables that are always allowed (core application tables)
static SYSTEM_TABLES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "automations",
        "bots",
        "bot_configurations",
        "bot_memories",
        "clicks",
        "group_members",
        "groups",
        "message_history",
        "organizations",
        "table_access",
        "tasks",
        "trigger_kinds",
        "user_login_tokens",
        "user_preferences",
        "user_sessions",
        "users",
        "a2a_messages",
        "api_records",
        "calendar_events",
        "documents",
        "email_accounts",
        "meetings",
        "notifications",
        "oauth_providers",
        "oauth_tokens",
        "research_sessions",
        "sources",
    ])
});

/// Check if a table exists in the database
/// This allows dynamically created tables (from app generator) to be accessed
pub fn table_exists_in_database(conn: &mut diesel::PgConnection, table_name: &str) -> bool {
    use diesel::prelude::*;
    use diesel::sql_query;
    use diesel::sql_types::Text;

    #[derive(diesel::QueryableByName)]
    struct TableExists {
        #[diesel(sql_type = diesel::sql_types::Bool)]
        exists: bool,
    }

    let result: Result<TableExists, _> = sql_query(
        "SELECT EXISTS (
            SELECT FROM information_schema.tables
            WHERE table_schema = 'public'
            AND table_name = $1
        ) as exists"
    )
    .bind::<Text, _>(table_name)
    .get_result(conn);

    match result {
        Ok(r) => r.exists,
        Err(_) => false,
    }
}

static ALLOWED_ORDER_COLUMNS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "id",
        "created_at",
        "updated_at",
        "name",
        "email",
        "title",
        "status",
        "priority",
        "due_date",
        "start_date",
        "end_date",
        "order",
        "position",
        "timestamp",
    ])
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SqlGuardError {
    InvalidTableName(String),
    InvalidColumnName(String),
    InvalidOrderDirection(String),
    InvalidIdentifier(String),
    PotentialInjection(String),
}

impl std::fmt::Display for SqlGuardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTableName(name) => write!(f, "Invalid table name: {name}"),
            Self::InvalidColumnName(name) => write!(f, "Invalid column name: {name}"),
            Self::InvalidOrderDirection(dir) => write!(f, "Invalid order direction: {dir}"),
            Self::InvalidIdentifier(id) => write!(f, "Invalid identifier: {id}"),
            Self::PotentialInjection(input) => write!(f, "Potential SQL injection detected: {input}"),
        }
    }
}

impl std::error::Error for SqlGuardError {}

/// Rejects a table name that is not a usable SQL identifier. Shared by every
/// validation path: safelist check, dynamic-table check, and the query builders.
fn check_table_syntax(table: &str) -> Result<String, SqlGuardError> {
    let sanitized = sanitize_identifier(table);

    // Check basic identifier validity (no SQL injection)
    if sanitized.is_empty() || sanitized.len() > 63 {
        return Err(SqlGuardError::InvalidTableName(table.to_string()));
    }

    // Check for dangerous patterns
    if sanitized.contains(';') || sanitized.contains("--") || sanitized.contains("/*") {
        return Err(SqlGuardError::PotentialInjection(table.to_string()));
    }

    Ok(sanitized)
}

/// Validate table name - checks system tables safelist only
/// For full validation including dynamic tables, use validate_table_name_with_conn
pub fn validate_table_name(table: &str) -> Result<&str, SqlGuardError> {
    // Deny by default: only the known system tables pass without a database.
    // Callers that legitimately serve dynamically created tables (the app
    // generator) hold a connection and must use
    // `validate_table_name_with_conn`, which additionally requires the table to
    // exist. Letting this function wave through any syntactically valid name
    // would let an unvalidated table name reach a query.
    let sanitized = check_table_syntax(table)?;

    if SYSTEM_TABLES.contains(sanitized.as_str()) {
        return Ok(table);
    }

    Err(SqlGuardError::InvalidTableName(table.to_string()))
}

/// Validate the syntax of a table name whose existence has already been
/// confirmed with `validate_table_name_with_conn` (or an equivalent
/// connection-aware check). It applies every injection and identifier rule, but
/// skips the system-table safelist so tables created at runtime stay reachable.
pub fn validate_dynamic_table_name(table: &str) -> Result<&str, SqlGuardError> {
    check_table_syntax(table)?;
    Ok(table)
}

/// Validate table name with database connection - checks if table actually exists
pub fn validate_table_name_with_conn<'a>(
    conn: &mut diesel::PgConnection,
    table: &'a str,
) -> Result<&'a str, SqlGuardError> {
    let sanitized = sanitize_identifier(table);

    // First do basic validation
    if sanitized.is_empty() || sanitized.len() > 63 {
        return Err(SqlGuardError::InvalidTableName(table.to_string()));
    }

    if sanitized.contains(';') || sanitized.contains("--") || sanitized.contains("/*") {
        return Err(SqlGuardError::PotentialInjection(table.to_string()));
    }

    // System tables are always allowed
    if SYSTEM_TABLES.contains(sanitized.as_str()) {
        return Ok(table);
    }

    // Check if table exists in database (for dynamically created tables)
    if table_exists_in_database(conn, &sanitized) {
        Ok(table)
    } else {
        Err(SqlGuardError::InvalidTableName(table.to_string()))
    }
}

pub fn validate_order_column(column: &str) -> Result<&str, SqlGuardError> {
    let sanitized = sanitize_identifier(column);

    if ALLOWED_ORDER_COLUMNS.contains(sanitized.as_str()) {
        Ok(column)
    } else {
        Err(SqlGuardError::InvalidColumnName(column.to_string()))
    }
}

pub fn validate_order_direction(direction: &str) -> Result<&'static str, SqlGuardError> {
    match direction.to_uppercase().as_str() {
        "ASC" => Ok("ASC"),
        "DESC" => Ok("DESC"),
        _ => Err(SqlGuardError::InvalidOrderDirection(direction.to_string())),
    }
}

pub fn sanitize_identifier(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

pub fn validate_identifier(name: &str) -> Result<String, SqlGuardError> {
    let sanitized = sanitize_identifier(name);

    if sanitized.is_empty() {
        return Err(SqlGuardError::InvalidIdentifier(name.to_string()));
    }

    if sanitized.len() > 64 {
        return Err(SqlGuardError::InvalidIdentifier("Identifier too long".to_string()));
    }

    if sanitized.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        return Err(SqlGuardError::InvalidIdentifier("Identifier cannot start with digit".to_string()));
    }

    Ok(sanitized)
}

pub fn check_for_injection_patterns(input: &str) -> Result<(), SqlGuardError> {
    let lower = input.to_lowercase();

    let dangerous_patterns = [
        "--",
        "/*",
        "*/",
        ";",
        "union",
        "select",
        "insert",
        "update",
        "delete",
        "drop",
        "truncate",
        "exec",
        "execute",
        "xp_",
        "sp_",
        "0x",
        "char(",
        "nchar(",
        "varchar(",
        "nvarchar(",
        "cast(",
        "convert(",
        "@@",
        "waitfor",
        "delay",
        "benchmark",
        "sleep(",
    ];

    for pattern in dangerous_patterns {
        if lower.contains(pattern) {
            return Err(SqlGuardError::PotentialInjection(format!(
                "Dangerous pattern '{}' detected",
                pattern
            )));
        }
    }

    Ok(())
}

pub fn escape_string_literal(value: &str) -> String {
    value.replace('\'', "''").replace('\\', "\\\\")
}

pub fn build_safe_select_query(
    table: &str,
    order_by: Option<&str>,
    order_dir: Option<&str>,
    limit: i32,
    offset: i32,
) -> Result<String, SqlGuardError> {
    build_select_query_for(validate_table_name(table)?, order_by, order_dir, limit, offset)
}

/// Same as [`build_safe_select_query`] for a table that exists in the database
/// but is not part of the system safelist. The caller must have confirmed its
/// existence with a connection-aware check first.
pub fn build_safe_select_query_for_dynamic_table(
    table: &str,
    order_by: Option<&str>,
    order_dir: Option<&str>,
    limit: i32,
    offset: i32,
) -> Result<String, SqlGuardError> {
    build_select_query_for(
        validate_dynamic_table_name(table)?,
        order_by,
        order_dir,
        limit,
        offset,
    )
}

fn build_select_query_for(
    validated_table: &str,
    order_by: Option<&str>,
    order_dir: Option<&str>,
    limit: i32,
    offset: i32,
) -> Result<String, SqlGuardError> {

    let safe_limit = limit.clamp(1, 1000);
    let safe_offset = offset.max(0);

    let order_clause = match (order_by, order_dir) {
        (Some(col), Some(dir)) => {
            let validated_col = validate_order_column(col)?;
            let validated_dir = validate_order_direction(dir)?;
            format!("ORDER BY {} {}", validated_col, validated_dir)
        }
        (Some(col), None) => {
            let validated_col = validate_order_column(col)?;
            format!("ORDER BY {} ASC", validated_col)
        }
        _ => "ORDER BY id ASC".to_string(),
    };

    Ok(format!(
        "SELECT row_to_json(t.*) as data FROM {} t {} LIMIT {} OFFSET {}",
        validated_table, order_clause, safe_limit, safe_offset
    ))
}

pub fn build_safe_count_query(table: &str) -> Result<String, SqlGuardError> {
    let validated_table = validate_table_name(table)?;
    Ok(format!("SELECT COUNT(*) as count FROM {}", validated_table))
}

/// Same as [`build_safe_count_query`] for a table that exists in the database
/// but is not part of the system safelist.
pub fn build_safe_count_query_for_dynamic_table(table: &str) -> Result<String, SqlGuardError> {
    let validated_table = validate_dynamic_table_name(table)?;
    Ok(format!("SELECT COUNT(*) as count FROM {}", validated_table))
}

pub fn build_safe_delete_query(table: &str) -> Result<String, SqlGuardError> {
    let validated_table = validate_table_name(table)?;
    Ok(format!("DELETE FROM {} WHERE id = $1", validated_table))
}

pub fn build_safe_select_by_id_query(table: &str) -> Result<String, SqlGuardError> {
    let validated_table = validate_table_name(table)?;
    Ok(format!(
        "SELECT row_to_json(t.*) as data FROM {} t WHERE id = $1",
        validated_table
    ))
}

/// Same as [`build_safe_select_by_id_query`] for a table that exists in the
/// database but is not part of the system safelist.
pub fn build_safe_select_by_id_query_for_dynamic_table(
    table: &str,
) -> Result<String, SqlGuardError> {
    let validated_table = validate_dynamic_table_name(table)?;
    Ok(format!(
        "SELECT row_to_json(t.*) as data FROM {} t WHERE id = $1",
        validated_table
    ))
}

pub fn register_dynamic_table(table_name: &'static str) {
    log::info!("Dynamic table registration requested for: {}", table_name);
}

/// Check if table is in the system tables safelist
pub fn is_system_table(table: &str) -> bool {
    let sanitized = sanitize_identifier(table);
    SYSTEM_TABLES.contains(sanitized.as_str())
}

/// Check if table is allowed (system table or valid identifier)
/// For full check including database existence, use is_table_allowed_with_conn
/// Reports whether `table` may be queried without a database round-trip.
/// Deny by default, mirroring [`validate_table_name`]: anything outside
/// `SYSTEM_TABLES` needs `is_table_allowed_with_conn` to confirm it exists.
pub fn is_table_allowed(table: &str) -> bool {
    let sanitized = sanitize_identifier(table);
    if sanitized.is_empty() || sanitized.len() > 63 {
        return false;
    }
    SYSTEM_TABLES.contains(sanitized.as_str())
}

/// Check if table is allowed with database connection
pub fn is_table_allowed_with_conn(conn: &mut diesel::PgConnection, table: &str) -> bool {
    let sanitized = sanitize_identifier(table);
    if SYSTEM_TABLES.contains(sanitized.as_str()) {
        return true;
    }
    table_exists_in_database(conn, &sanitized)
}

#[cfg(test)]
#[path = "sql_guard_tests.rs"]
mod tests;
