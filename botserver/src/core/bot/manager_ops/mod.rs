pub(crate) use botlib::security::SafeCommand;
pub(crate) use botcore::shared::schema::organizations;
pub(crate) use botcore::shared::utils::DbPool;
pub(crate) use diesel::prelude::*;
pub(crate) use diesel::OptionalExtension;
pub(crate) use log::{debug, error, info, warn};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use axum::response::IntoResponse;
pub(crate) use super::manager::{BotConfig, BotManager, BotTemplate, DialogFile};

/// Why a bot access check failed. Callers map this to a response so a client
/// can tell "this bot is private, sign in" apart from "no such bot" and from a
/// genuine internal failure — a bare 403 left the chat window reporting
/// "connection failed" for a private bot with no way to recover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BotAccessDenial {
    /// No bot row matches the requested name or slug.
    NotFound,
    /// The bot is private and the caller is not a member of its organization.
    AuthRequired,
    /// The check itself could not complete (database unavailable, etc.).
    Internal(String),
}

mod part1;
mod part2;
mod part3;

pub use part1::*;
pub use part2::*;
pub use part3::*;
