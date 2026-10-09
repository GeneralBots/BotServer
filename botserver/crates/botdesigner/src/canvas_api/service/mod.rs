pub(crate) use crate::canvas_api::db::{row_to_canvas, CanvasRow, TemplateRow};
pub(crate) use crate::canvas_api::error::CanvasError;
pub(crate) use crate::canvas_api::types::*;
pub(crate) use crate::DbPool;
pub(crate) use chrono::Utc;
pub(crate) use diesel::prelude::*;
pub(crate) use diesel::sql_types::{Text, Uuid as DieselUuid};
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::broadcast;
pub(crate) use uuid::Uuid;

pub struct CanvasService {
    pool: Arc<DbPool>,
    event_sender: broadcast::Sender<CanvasEvent>,
}

mod part1;
mod part2;
mod part3;
