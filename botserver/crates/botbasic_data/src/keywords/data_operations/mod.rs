pub(crate) use crate::keywords::table_access::{check_table_access, AccessType, UserRoles};
pub(crate) use botbasic_core::{sanitize_identifier, sanitize_sql_value};
pub(crate) use botbasic_types::UserSession;
pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use botbasic_core::utils::{convert_date_to_iso_format, json_value_to_dynamic, to_array};
pub(crate) use diesel::prelude::*;
pub(crate) use diesel::sql_query;
pub(crate) use diesel::sql_types::Text;
pub(crate) use log::{trace, warn};
pub(crate) use rhai::{Array, Dynamic, Engine, Map};
pub(crate) use serde_json::{json, Value};
pub(crate) use std::collections::HashMap;
pub(crate) use std::error::Error;
pub(crate) use std::sync::Arc;

#[derive(diesel::QueryableByName)]
struct BotNameRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    name: String,
}

#[derive(diesel::QueryableByName)]
struct TriggerRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    param: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_identifier() {
        assert_eq!(sanitize_identifier("users"), "users");
        assert_eq!(sanitize_identifier("user_name"), "user_name");
        // Only [a-z0-9_] survives and the result is folded to lower case, so a
        // statement separator or keyword smuggled into a name cannot come back
        // out of the filter.
        assert_eq!(
            sanitize_identifier("users; DROP TABLE users;"),
            "usersdroptableusers"
        );
        assert_eq!(sanitize_identifier("Users"), "users");
        assert!(!sanitize_identifier("users; DROP TABLE users;").contains(';'));
    }

    #[test]
    fn test_sanitize_sql_value() {
        assert_eq!(sanitize_sql_value("hello"), "hello");
        assert_eq!(sanitize_sql_value("it's"), "it''s");
        assert_eq!(sanitize_sql_value("O'Brien"), "O''Brien");
    }

    #[test]
    fn test_parse_condition() {
        let (field, op, value) = parse_condition_internal("status=active").unwrap();
        assert_eq!(field, "status");
        assert_eq!(op, "=");
        assert_eq!(value, "active");

        let (field, op, value) = parse_condition_internal("age>=18").unwrap();
        assert_eq!(field, "age");
        assert_eq!(op, ">=");
        assert_eq!(value, "18");
    }

    #[test]
    fn test_parse_filter_clause() {
        let clause = parse_filter_clause("name=John").unwrap();
        assert!(clause.contains("name"));
        assert!(clause.contains("John"));
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

pub use part1::*;
pub use part2::*;
pub use part3::*;
use part4::*;
use part5::*;
