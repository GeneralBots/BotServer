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

pub(crate) use botbasic_core::sanitize_identifier;
pub(crate) use botbasic_types::UserSession;
pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use diesel::prelude::*;
pub(crate) use diesel::sql_types::Text;
pub(crate) use log::{error, info, trace, warn};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::error::Error;
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FieldDefinition {
    pub name: String,
    pub field_type: String,
    #[serde(default)]
    pub length: Option<i32>,
    #[serde(default)]
    pub precision: Option<i32>,
    #[serde(default)]
    pub is_key: bool,
    #[serde(default)]
    pub is_nullable: bool,
    #[serde(default)]
    pub default_value: Option<String>,
    #[serde(default)]
    pub reference_table: Option<String>,
    #[serde(default)]
    pub field_order: i32,
    #[serde(default)]
    pub read_roles: Vec<String>,
    #[serde(default)]
    pub write_roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TableDefinition {
    pub name: String,
    #[serde(default)]
    pub connection_name: String,
    #[serde(default)]
    pub fields: Vec<FieldDefinition>,
    #[serde(default)]
    pub read_roles: Vec<String>,
    #[serde(default)]
    pub write_roles: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ExternalConnection {
    pub name: String,
    pub driver: String,
    pub server: String,
    pub port: Option<i32>,
    pub database: String,
    pub username: String,
    pub password: String,
}

#[derive(QueryableByName)]
struct IdResult {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
}

mod part1;
mod part2;
mod part3;

pub use part1::*;
pub use part2::*;
pub use part3::*;
