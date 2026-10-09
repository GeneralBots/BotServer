/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.com.br. All rights reserved.         |
| Licensed under the MIT License.                                             |
|                                                                             |
| This program is free software: you can redistribute it and/or modify        |
| it under the terms of the MIT License.                                      |
|                                                                             |
| The text of the MIT License can be found in the LICENSE file you have       |
| received along with this program.                                           |
|                                                                             |
| This program is distributed in the hope that it will be useful,             |
| but WITHOUT ANY WARRANTY, without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.                        |
|                                                                             |
| "General Bots" is a registered trademark of pragmatismo.com.br.             |
| The licensing of the program under the MIT License does not imply a         |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

pub(crate) use botbasic_types::{BasicRuntime, UserSession};
pub(crate) use diesel::prelude::*;
pub(crate) use diesel::sql_query;
pub(crate) use diesel::sql_types::Text;
pub(crate) use log::{trace, warn};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use serde_json::Value;
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserRoles {
    pub roles: Vec<String>,
    pub user_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessType {
    Read,
    Write,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableAccessInfo {
    pub table_name: String,
    pub read_roles: Vec<String>,
    pub write_roles: Vec<String>,
    pub field_read_roles: HashMap<String, Vec<String>>,
    pub field_write_roles: HashMap<String, Vec<String>>,
}

#[derive(QueryableByName, Debug)]
struct TableDefRow {
    #[diesel(sql_type = Text)]
    table_name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    read_roles: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    write_roles: Option<String>,
}

#[derive(QueryableByName, Debug)]
struct FieldDefRow {
    #[diesel(sql_type = Text)]
    field_name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    read_roles: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    write_roles: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_roles_has_access() {
        let roles = UserRoles::new(vec!["admin".to_string(), "manager".to_string()]);

        // Empty roles = everyone allowed
        assert!(roles.has_access(&[]));

        // User has admin role
        assert!(roles.has_access(&["admin".to_string()]));

        // User has manager role
        assert!(roles.has_access(&["manager".to_string(), "superuser".to_string()]));

        // User doesn't have superuser role only
        assert!(!roles.has_access(&["superuser".to_string()]));
    }

    #[test]
    fn test_user_roles_case_insensitive() {
        let roles = UserRoles::new(vec!["Admin".to_string()]);

        assert!(roles.has_access(&["admin".to_string()]));
        assert!(roles.has_access(&["ADMIN".to_string()]));
        assert!(roles.has_access(&["Admin".to_string()]));
    }

    #[test]
    fn test_anonymous_user() {
        let roles = UserRoles::anonymous();

        // Anonymous can access if no roles required
        assert!(roles.has_access(&[]));

        // Anonymous cannot access if roles required
        assert!(!roles.has_access(&["admin".to_string()]));
    }

    #[test]
    fn test_table_access_info_field_restrictions() {
        let mut info = TableAccessInfo {
            table_name: "contacts".to_string(),
            read_roles: vec![],
            write_roles: vec![],
            field_read_roles: HashMap::new(),
            field_write_roles: HashMap::new(),
        };

        info.field_read_roles
            .insert("ssn".to_string(), vec!["admin".to_string()]);
        info.field_write_roles
            .insert("salary".to_string(), vec!["hr".to_string()]);

        let admin = UserRoles::new(vec!["admin".to_string()]);
        let hr = UserRoles::new(vec!["hr".to_string()]);
        let user = UserRoles::new(vec!["user".to_string()]);

        // Admin can read SSN
        assert!(info.can_read_field("ssn", &admin));
        // Regular user cannot read SSN
        assert!(!info.can_read_field("ssn", &user));

        // HR can write salary
        assert!(info.can_write_field("salary", &hr));
        // Admin cannot write salary (different role)
        assert!(!info.can_write_field("salary", &admin));

        // Everyone can read/write unrestricted fields
        assert!(info.can_read_field("name", &user));
        assert!(info.can_write_field("name", &user));
    }

    #[test]
    fn test_filter_fields_by_role() {
        let mut info = TableAccessInfo::default();
        info.field_read_roles
            .insert("secret".to_string(), vec!["admin".to_string()]);

        let data = serde_json::json!({
            "id": 1,
            "name": "John",
            "secret": "classified"
        });

        let user = UserRoles::new(vec!["user".to_string()]);
        let filtered = filter_fields_by_role(data.clone(), &user, &Some(info.clone()));

        assert!(filtered.get("id").is_some());
        assert!(filtered.get("name").is_some());
        assert!(filtered.get("secret").is_none());

        // Admin can see everything
        let admin = UserRoles::new(vec!["admin".to_string()]);
        let not_filtered = filter_fields_by_role(data, &admin, &Some(info));

        assert!(not_filtered.get("secret").is_some());
    }

    #[test]
    fn test_parse_roles_string() {
        assert_eq!(parse_roles_string(None), Vec::<String>::new());
        assert_eq!(
            parse_roles_string(Some("".to_string()).as_ref()),
            Vec::<String>::new()
        );
        assert_eq!(
            parse_roles_string(Some("admin".to_string()).as_ref()),
            vec!["admin"]
        );
        assert_eq!(
            parse_roles_string(Some("admin;manager".to_string()).as_ref()),
            vec!["admin", "manager"]
        );
        assert_eq!(
            parse_roles_string(Some(" admin ; manager ; hr ".to_string()).as_ref()),
            vec!["admin", "manager", "hr"]
        );
    }
}

mod part1;
mod part2;
mod part3;

pub use part3::*;
