/*****************************************************************************\
|  Table Schema Migration Module
|  Automatically syncs table.bas definitions with database schema
\*****************************************************************************/

use botbasic_types::BasicRuntime;
use botbasic_types::UserSession;
use botbasic_core::security_utils::sanitize_identifier;
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::Text;
use log::{info, warn};
use rhai::Engine;
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

use crate::keywords::table_definition::{
    map_type_to_sql, parse_table_definition, FieldDefinition, TableDefinition,
};

/// Schema migration result
#[derive(Debug, Default)]
pub struct MigrationResult {
    pub tables_created: usize,
    pub tables_altered: usize,
    pub columns_added: usize,
    pub columns_dropped: usize,
    pub errors: Vec<String>,
}

/// Column metadata from database
#[derive(Debug, Clone)]
pub struct DbColumn {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
}

/// Columns that should never be dropped automatically
const PROTECTED_COLUMNS: &[&str] = &[
    "id",
    "bot_id",
    "org_id",
    "user_id",
    "created_at",
    "updated_at",
    "deleted_at",
    "is_deleted",
    "version",
    "tenant_id",
];

/// PostgreSQL hard limit on columns per table (attribute slots).
/// Dropped columns keep their slots forever, so after many ADD/DROP cycles
/// even a small table hits this cap and rejects new columns.
const PG_MAX_COLUMNS: i64 = 1600;

/// Safety margin: compact before we are this close to the hard limit,
/// so a single schema change can never push a table over the edge.
const PG_COMPACT_THRESHOLD: i64 = 1400;

/// Count total attribute slots (live + dropped) used by a table.
fn count_attribute_slots(
    table_name: &str,
    conn: &mut diesel::PgConnection,
) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let query = format!(
        "SELECT count(*) FROM pg_attribute a
         JOIN pg_class c ON c.oid = a.attrelid
         JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE c.relname = '{}' AND n.nspname = 'public' AND a.attnum > 0",
        sanitize_identifier(table_name)
    );

    #[derive(QueryableByName)]
    struct SlotRow {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        count: i64,
    }

    let rows: Vec<SlotRow> = sql_query(&query).load(conn)?;
    Ok(rows.first().map(|r| r.count).unwrap_or(0))
}

/// Rebuild a table without its dropped-column slots.
/// Postgres never reuses dropped attribute slots, so after roughly 1600
/// ADD/DROP cycles `ALTER TABLE ... ADD COLUMN` fails with
/// "tables can have at most 1600 columns". Creating a fresh table with
/// `LIKE ... INCLUDING ALL` copies only the live columns (resetting attnum)
/// along with indexes, constraints and defaults, then we move the data over.
fn compact_table(
    table_name: &str,
    conn: &mut diesel::PgConnection,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let safe = sanitize_identifier(table_name);
    let tmp = format!("{}__compact", safe);

    info!("Compacting table {} to reclaim dropped-column slots", table_name);

    // Run the whole swap atomically so a failure mid-way cannot leave the
    // table missing or half-migrated.
    conn.transaction(|conn| {
        // Copy schema (live columns only, fresh attnum) plus indexes/constraints.
        sql_query(format!("CREATE TABLE {} (LIKE {} INCLUDING ALL)", tmp, safe))
            .execute(conn)
            .map_err(|e| format!("Failed to create compact table {}: {}", tmp, e))?;

        // Move the data (column order matches: LIKE preserves live column order).
        sql_query(format!("INSERT INTO {} SELECT * FROM {}", tmp, safe))
            .execute(conn)
            .map_err(|e| format!("Failed to copy data while compacting {}: {}", table_name, e))?;

        // Swap tables.
        sql_query(format!("DROP TABLE {}", safe))
            .execute(conn)
            .map_err(|e| format!("Failed to drop old {} during compaction: {}", table_name, e))?;
        sql_query(format!("ALTER TABLE {} RENAME TO {}", tmp, safe))
            .execute(conn)
            .map_err(|e| format!("Failed to rename compact table to {}: {}", table_name, e))?;
        Ok::<(), Box<dyn Error + Send + Sync>>(())
    })?;

    info!("Compaction of {} complete", table_name);
    Ok(())
}

/// Compare and sync table schema with definition
pub fn sync_table_schema(
    table: &TableDefinition,
    existing_columns: &[DbColumn],
    create_sql: &str,
    conn: &mut diesel::PgConnection,
) -> Result<MigrationResult, Box<dyn Error + Send + Sync>> {
    let mut result = MigrationResult::default();

    // If no columns exist, create the table
    if existing_columns.is_empty() {
        info!("Creating new table: {}", table.name);
        sql_query(create_sql)
            .execute(conn)
            .map_err(|e| format!("Failed to create table {}: {}", table.name, e))?;
        result.tables_created += 1;
        return Ok(result);
    }

    let table_name = sanitize_identifier(&table.name);
    let defined_col_names: std::collections::HashSet<String> =
        table.fields.iter().map(|f| f.name.to_lowercase()).collect();
    let existing_col_names: std::collections::HashSet<String> =
        existing_columns.iter().map(|c| c.name.to_lowercase()).collect();

    // Add missing columns
    let mut missing_columns: Vec<&FieldDefinition> = Vec::new();
    for field in &table.fields {
        if !existing_col_names.contains(&field.name.to_lowercase()) {
            missing_columns.push(field);
        }
    }

    if !missing_columns.is_empty() {
        info!(
            "Table {} is missing {} columns, adding them",
            table.name,
            missing_columns.len()
        );

        // PostgreSQL never reuses dropped-column slots. If this table has been
        // through many ADD/DROP cycles it may be close to the 1600 hard cap,
        // in which case the ALTERs below would fail. Compact it first so the
        // rebuilt table has a fresh attnum sequence and room for new columns.
        let slots = count_attribute_slots(&table_name, conn)?;
        if slots + missing_columns.len() as i64 > PG_COMPACT_THRESHOLD {
            warn!(
                "Table {} uses {} attribute slots (near the {} cap); compacting before adding {} columns",
                table.name,
                slots,
                PG_MAX_COLUMNS,
                missing_columns.len()
            );
            compact_table(&table_name, conn)?;
        }

        for field in &missing_columns {
            let sql_type = map_type_to_sql(field, "postgres");
            let column_sql = format!(
                "ALTER TABLE {} ADD COLUMN IF NOT EXISTS {} {}",
                table_name,
                sanitize_identifier(&field.name.to_lowercase()),
                sql_type
            );

            info!(
                "Adding column: {}.{} ({})",
                table.name, field.name, sql_type
            );
            match sql_query(&column_sql).execute(conn) {
                Ok(_) => {
                    result.columns_added += 1;
                    info!("Successfully added column {}.{}", table.name, field.name);
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if !err_str.contains("already exists") && !err_str.contains("duplicate column")
                    {
                        let error_msg =
                            format!("Failed to add column {}.{}: {}", table.name, field.name, e);
                        log::error!("{}", error_msg);
                        result.errors.push(error_msg);
                    } else {
                        info!(
                            "Column {}.{} already exists, skipping",
                            table.name, field.name
                        );
                    }
                }
            }
        }
        result.tables_altered += 1;
    }

    // Drop columns that were removed from definition (with protection)
    let mut orphaned_columns: Vec<&DbColumn> = Vec::new();
    for col in existing_columns {
        if !defined_col_names.contains(&col.name.to_lowercase())
            && !PROTECTED_COLUMNS.contains(&col.name.as_str())
        {
            orphaned_columns.push(col);
        }
    }

    if !orphaned_columns.is_empty() {
        warn!(
            "Table {} has {} orphaned columns not in definition:",
            table.name,
            orphaned_columns.len()
        );

        for col in &orphaned_columns {
            info!("Dropping orphaned column: {}.{}", table.name, col.name);
            let drop_sql = format!(
                "ALTER TABLE {} DROP COLUMN IF EXISTS {}",
                table_name,
                sanitize_identifier(&col.name)
            );

            match sql_query(&drop_sql).execute(conn) {
                Ok(_) => {
                    result.columns_dropped += 1;
                    info!("Successfully dropped column {}.{}", table.name, col.name);
                }
                Err(e) => {
                    let error_msg =
                        format!("Failed to drop column {}.{}: {}", table.name, col.name, e);
                    log::error!("{}", error_msg);
                    result.errors.push(error_msg);
                }
            }
        }
        if result.columns_dropped > 0 {
            result.tables_altered += 1;
        }
    }

    Ok(result)
}

/// Get existing columns from a table
pub fn get_table_columns(
    table_name: &str,
    conn: &mut diesel::PgConnection,
) -> Result<Vec<DbColumn>, Box<dyn Error + Send + Sync>> {
    let query = format!(
        "SELECT column_name, data_type, is_nullable
         FROM information_schema.columns
         WHERE table_name = '{}' AND table_schema = 'public'
         ORDER BY ordinal_position",
        sanitize_identifier(table_name)
    );

    #[derive(QueryableByName)]
    struct ColumnRow {
        #[diesel(sql_type = Text)]
        column_name: String,
        #[diesel(sql_type = Text)]
        data_type: String,
        #[diesel(sql_type = Text)]
        is_nullable: String,
    }

    let rows: Vec<ColumnRow> = match sql_query(&query).load(conn) {
        Ok(r) => r,
        Err(e) => {
            // Table doesn't exist
            return Err(format!("Table {} does not exist: {}", table_name, e).into());
        }
    };

    Ok(rows
        .into_iter()
        .map(|row| DbColumn {
            name: row.column_name,
            data_type: row.data_type,
            is_nullable: row.is_nullable == "YES",
        })
        .collect())
}

/// Process table definitions with schema sync for a specific bot
pub fn sync_bot_tables(
    state: Arc<dyn BasicRuntime>,
    bot_id: Uuid,
    source: &str,
) -> Result<MigrationResult, Box<dyn Error + Send + Sync>> {
    let tables = parse_table_definition(source)?;
    let mut result = MigrationResult::default();

    info!(
        "Processing {} table definitions with schema sync for bot {}",
        tables.len(),
        bot_id
    );

    // Get bot's database connection
    let pool = state.bot_database_manager().get_bot_pool(bot_id)
            .ok_or_else(|| format!("No database pool for bot {}", bot_id))?;
    let mut conn = pool.get()?;

    for table in &tables {
        if table.connection_name != "default" {
            continue; // Skip external connections for now
        }

        info!("Syncing table: {}", table.name);

        // Get existing columns
        let existing_columns = get_table_columns(&table.name, &mut conn).unwrap_or_default();

        // Generate CREATE TABLE SQL
        let create_sql = crate::keywords::table_definition::generate_create_table_sql(table, "postgres");

        // Sync schema
        match sync_table_schema(table, &existing_columns, &create_sql, &mut conn) {
            Ok(sync_result) => {
                result.tables_created += sync_result.tables_created;
                result.tables_altered += sync_result.tables_altered;
                result.columns_added += sync_result.columns_added;
                result.columns_dropped += sync_result.columns_dropped;
                result.errors.extend(sync_result.errors);
            }
            Err(e) => {
                let error_msg = format!("Failed to sync table {}: {}", table.name, e);
                log::error!("{}", error_msg);
                result.errors.push(error_msg);
            }
        }
    }

    // Log summary
    info!("Schema sync summary for bot {}: {} tables created, {} altered, {} columns added, {} columns dropped, {} errors",
        bot_id, result.tables_created, result.tables_altered, result.columns_added, result.columns_dropped, result.errors.len());

    if !result.errors.is_empty() {
        warn!("Schema sync completed with {} errors:", result.errors.len());
        for error in &result.errors {
            warn!("  - {}", error);
        }
    }

    Ok(result)
}

/// Validate that all required columns exist
pub fn validate_table_schema(
    table_name: &str,
    required_fields: &[FieldDefinition],
    conn: &mut diesel::PgConnection,
) -> Result<bool, Box<dyn Error + Send + Sync>> {
    let existing_columns = get_table_columns(table_name, conn)?;
    let existing_col_names: std::collections::HashSet<String> =
        existing_columns.iter().map(|c| c.name.to_lowercase()).collect();

    let mut missing = Vec::new();
    for field in required_fields {
        if !existing_col_names.contains(&field.name.to_lowercase()) {
            missing.push(field.name.clone());
        }
    }

    if !missing.is_empty() {
        warn!("Table {} is missing columns: {:?}", table_name, missing);
        return Ok(false);
    }

    Ok(true)
}

pub fn register_table_migration_keywords(
    _state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    _engine: &mut Engine,
) {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_column_creation() {
        let col = DbColumn {
            name: "test_col".to_string(),
            data_type: "character varying".to_string(),
            is_nullable: true,
        };
        assert_eq!(col.name, "test_col");
        assert!(col.is_nullable);
    }
}
