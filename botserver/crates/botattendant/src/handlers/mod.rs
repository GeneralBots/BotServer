pub mod agents;
pub mod attachments;
pub mod queues;
pub mod sessions;

use diesel::prelude::*;
use uuid::Uuid;

use crate::schema::*;

pub(crate) fn get_bot_context(config: &crate::AttendantConfig) -> Uuid {
    let Ok(mut conn) = config.pool.get() else {
        return Uuid::nil();
    };
    (config.get_default_bot)(&mut conn)
}

pub(crate) fn org_and_bot(config: &crate::AttendantConfig, branch_id: Uuid) -> (Uuid, Uuid) {
    let Ok(mut conn) = config.pool.get() else {
        return (Uuid::nil(), Uuid::nil());
    };
    botschema::bots::table
        .filter(botschema::bots::branch_id.eq(branch_id))
        .select((botschema::bots::org_id, botschema::bots::id))
        .first(&mut conn)
        .unwrap_or((Uuid::nil(), Uuid::nil()))
}

pub(crate) fn generate_session_number(conn: &mut diesel::PgConnection, branch_id: Uuid) -> String {
    let count: i64 = attendant_sessions::table
        .filter(attendant_sessions::branch_id.eq(Some(branch_id)))
        .count()
        .get_result(conn)
        .unwrap_or(0);
    format!("SES-{:06}", count + 1)
}

macro_rules! db_conn {
    ($config:expr) => {
        $config.pool.get().map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
        })?
    };
}

pub(crate) use db_conn;

pub use agents::*;
pub use attachments::*;
pub use queues::*;
pub use sessions::*;
