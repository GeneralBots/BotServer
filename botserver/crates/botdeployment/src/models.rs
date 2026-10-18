use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use super::schema::projects;

#[derive(Debug, Clone, Queryable, Selectable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = projects)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub org: String,
    pub project_type: String,
    pub deploy_target: String,
    pub repo_url: Option<String>,
    pub deploy_url: Option<String>,
    pub container_name: Option<String>,
    pub custom_domain: Option<String>,
    pub environment: String,
    pub framework: Option<String>,
}
