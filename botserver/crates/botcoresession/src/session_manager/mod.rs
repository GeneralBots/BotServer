pub(crate) use crate::schema::{message_history, user_sessions, users};
pub(crate) use crate::session_data::{SessionData, UserSession};
pub(crate) use chrono::Utc;
pub(crate) use diesel::prelude::*;
pub(crate) use diesel::r2d2::{ConnectionManager, Pool};
pub(crate) use diesel::PgConnection;
pub(crate) use log::{trace, warn};
#[cfg(feature = "cache")]
pub(crate) use redis::Client;
pub(crate) use std::collections::{HashMap, HashSet};
pub(crate) use std::error::Error;
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use botsecurity_crypto::encryption::{encrypt_field, decrypt_field, derive_scope_key, load_master_encryption_key};

pub struct SessionManager {
    pool: Pool<ConnectionManager<PgConnection>>,
    master_key: Vec<u8>,
    sessions: HashMap<Uuid, SessionData>,
    waiting_for_input: HashSet<Uuid>,
    branch_id: Uuid,
    #[cfg(feature = "cache")]
    redis: Option<Arc<Client>>,
}

mod part1;
mod part2;
mod part3;
