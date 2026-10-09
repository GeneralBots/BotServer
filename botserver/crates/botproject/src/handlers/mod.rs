pub(crate) use axum::{ extract::{Extension, Multipart, Path, Query, State}, http::StatusCode, response::IntoResponse, Json, };
pub(crate) use botsecurity_auth::auth_api::types::AuthenticatedUser;
pub(crate) use chrono::Utc;
pub(crate) use serde::Deserialize;
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use crate::export;
pub(crate) use crate::import::{ImportFormat, ImportOptions, ProjectImportService};
pub(crate) use crate::service::ProjectService;
pub(crate) use crate::types::*;

type ApiError = (StatusCode, Json<serde_json::Value>);

#[derive(Deserialize)]
pub struct ExportQuery {
    pub format: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use botsecurity_auth::auth_api::types::Role;

    fn test_project(org: Uuid) -> Project {
        Project {
            id: Uuid::new_v4(),
            organization_id: org,
            name: "p".to_string(),
            description: None,
            start_date: chrono::NaiveDate::from_ymd_opt(2026, 8, 16).unwrap(),
            end_date: None,
            status: ProjectStatus::Planning,
            owner_id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            settings: ProjectSettings::default(),
        }
    }

    #[test]
    fn user_org_uses_organization_when_present() {
        let org = Uuid::new_v4();
        let user = AuthenticatedUser::new(Uuid::new_v4(), "tester".into()).with_organization(org);
        assert_eq!(user_org(&user), org);
    }

    #[test]
    fn user_org_falls_back_to_user_id() {
        let user = AuthenticatedUser::new(Uuid::new_v4(), "tester".into());
        assert_eq!(user_org(&user), user.user_id);
    }

    #[test]
    fn project_access_is_scoped_to_organization() {
        let org = Uuid::new_v4();
        let user = AuthenticatedUser::new(Uuid::new_v4(), "tester".into()).with_organization(org);

        // Same organization -> allowed.
        assert!(can_access_project(&user, &test_project(org)));
        // Different organization -> denied (the IDOR / cross-tenant leak guard).
        assert!(!can_access_project(&user, &test_project(Uuid::new_v4())));
    }

    #[test]
    fn admin_can_access_any_organization() {
        let admin = AuthenticatedUser::new(Uuid::new_v4(), "admin".into()).with_role(Role::Admin);
        assert!(can_access_project(&admin, &test_project(Uuid::new_v4())));
    }
}

mod part1;
mod part2;
mod part3;

pub use part1::*;
pub use part2::*;
pub use part3::*;
