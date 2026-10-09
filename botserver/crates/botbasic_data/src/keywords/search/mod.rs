//! SEARCH keyword implementation for full-text search with autocomplete support
//!
//! Provides fast product search across tables using PostgreSQL full-text search
//! and trigram similarity for fuzzy matching and autocomplete.
//!
//! Usage in .bas files:
//!   results = SEARCH "Products.csv", "chocolate", 10
//!   results = SEARCH "products", query, limit

pub(crate) use crate::keywords::table_access::{check_table_access, filter_fields_by_role, AccessType, UserRoles};
pub(crate) use botbasic_core::security_utils::sanitize_identifier;
pub(crate) use botbasic_types::UserSession;
pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use botbasic_core::utils;
pub(crate) use botbasic_core::utils::to_array;
pub(crate) use diesel::prelude::*;
pub(crate) use diesel::sql_types::{Integer, Text};
pub(crate) use log::{trace, warn};
pub(crate) use rhai::{Dynamic, Engine};
pub(crate) use serde_json::{json, Value};
pub(crate) use std::sync::Arc;

#[derive(QueryableByName)]
struct JsonRow {
    #[diesel(sql_type = Text)]
    row_data: String,
}

#[derive(QueryableByName)]
struct SearchResultRow {
    #[diesel(sql_type = Text)]
    row_data: String,
    #[diesel(sql_type = diesel::sql_types::Float)]
    relevance: f32,
}

#[cfg(test)]
mod tests {


    #[test]
    fn test_sanitize_search_query() {
        let query = "test' OR '1'='1";
        let safe = query.replace('\'', "''").replace('\\', "\\\\");

        // Doubling every quote keeps the value inside a single literal. The
        // assertion is the doubling itself: a substring check for "' OR '"
        // cannot work, because doubling legitimately turns "' OR '" into
        // "'' OR ''", which still contains that sequence.
        assert_eq!(safe, "test'' OR ''1''=''1");
        assert_eq!(safe.matches("''").count(), query.matches('\'').count());
    }
}

mod part1;
mod part2;

pub use part1::*;
pub use part2::*;
