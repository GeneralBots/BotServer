use diesel::PgConnection;
use std::sync::Arc;
use uuid::Uuid;

pub use botlib::db_pool::DbPool;

pub type AppState = Arc<DbPool>;

pub type GetDefaultBotFn = Arc<dyn Fn(&mut PgConnection) -> Uuid + Send + Sync>;

pub type TriggerContactChangeFn = Arc<dyn Fn(&mut PgConnection, Uuid, &str, Uuid) + Send + Sync>;

pub type TriggerDealStageChangeFn = Arc<dyn Fn(&mut PgConnection, Uuid, &str, &str, Uuid) + Send + Sync>;

#[derive(Clone)]
pub struct CrateState {
    pub db_pool: DbPool,
    pub get_default_bot: GetDefaultBotFn,
    pub trigger_contact_change: TriggerContactChangeFn,
    pub trigger_deal_stage_change: TriggerDealStageChangeFn,
}

impl CrateState {
    pub fn new(
        db_pool: DbPool,
        get_default_bot: GetDefaultBotFn,
        trigger_contact_change: TriggerContactChangeFn,
        trigger_deal_stage_change: TriggerDealStageChangeFn,
    ) -> Self {
        Self {
            db_pool,
            get_default_bot,
            trigger_contact_change,
            trigger_deal_stage_change,
        }
    }

    pub fn get_bot_context(&self) -> Uuid {
        use diesel::prelude::*;
        use crate::schema::bots::dsl::{bots, branch_id, is_default_for_branch};

        let Ok(mut conn) = self.db_pool.get() else {
            return Uuid::nil();
        };
        bots
            .filter(is_default_for_branch.eq(true))
            .select(branch_id)
            .first::<Uuid>(&mut conn)
            .unwrap_or(Uuid::nil())
    }

    /// Resolves the org that owns the given branch (branch → org via the
    /// branches table). The org is the gborg tenant that owns the workspace
    /// — it must never be conflated with the branch id.
    pub fn org_for_branch(&self, branch_id: Uuid) -> Uuid {
        use diesel::prelude::*;
        #[derive(diesel::QueryableByName)]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            org_id: Uuid,
        }
        let Ok(mut conn) = self.db_pool.get() else {
            return Uuid::nil();
        };
        diesel::sql_query(
            "SELECT org_id FROM branches WHERE id = $1 LIMIT 1",
        )
        .bind::<diesel::sql_types::Uuid, _>(branch_id)
        .get_result::<Row>(&mut conn)
        .map(|r| r.org_id)
        .unwrap_or_else(|_| Uuid::nil())
    }

    /// Resolves a bot id inside the given branch (the branch's default bot
    /// when flagged, otherwise any active bot in the branch).
    pub fn bot_for_branch(&self, branch_id: Uuid) -> Uuid {
        use diesel::prelude::*;
        use crate::schema::bots::dsl::{bots, branch_id as b_branch, id, is_active, is_default_for_branch};
        let Ok(mut conn) = self.db_pool.get() else {
            return Uuid::nil();
        };
        bots
            .filter(b_branch.eq(branch_id))
            .filter(is_active.eq(true))
            .order_by(is_default_for_branch.desc())
            .select(id)
            .first::<Uuid>(&mut conn)
            .unwrap_or(Uuid::nil())
    }
}

pub mod schema;
pub mod schema_ext;
pub mod models;
pub mod requests;
pub mod account_requests;
pub mod bulk_types;
pub mod stages;
pub mod scope;
pub mod error;
pub mod audit;
pub mod authz;
pub mod migration;
pub mod contacts_api;
pub(crate) mod contacts_api_helpers;
pub mod handlers;
pub mod ui;
pub mod routes;
pub mod sales_funnel;
pub mod forecast;
pub mod email_integration;
pub mod routes_sales;

#[cfg(feature = "calendar")]
pub mod calendar_types;
#[cfg(feature = "calendar")]
pub mod calendar_service;
#[cfg(feature = "calendar")]
pub(crate) mod calendar_service_helpers;
#[cfg(feature = "calendar")]
pub mod calendar_routes;

#[cfg(feature = "tasks")]
pub mod tasks_types;
#[cfg(feature = "tasks")]
pub mod tasks_service;
#[cfg(feature = "tasks")]
pub(crate) mod tasks_service_helpers;
#[cfg(feature = "tasks")]
pub mod tasks_routes;

#[cfg(feature = "external_sync")]
pub mod sync_types;
#[cfg(feature = "external_sync")]
pub mod sync_service;
#[cfg(feature = "external_sync")]
pub mod google_client;
#[cfg(feature = "external_sync")]
pub mod microsoft_client;
#[cfg(feature = "external_sync")]
pub mod sync_routes;

pub use error::ContactsError;
pub use migration::create_contacts_tables_migration;
