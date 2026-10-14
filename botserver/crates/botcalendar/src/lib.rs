pub mod caldav;
pub mod caldav_xml;
pub mod conflict_resolution;

mod caldav_http;
mod caldav_store;

pub use caldav::create_caldav_router;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
use diesel::prelude::*;
use icalendar::{
    Calendar, CalendarDateTime, Component, DatePerhapsTime, Event as IcalEvent, EventLike,
    Property,
};
use log::info;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

pub use botlib::db_pool::DbPool;

// The calendar tables are declared once, in `botschema`, so every reader and
// writer compiles against the same columns. This crate used to carry its own
// duplicates, which omitted `branch_id` — a column that is `NOT NULL` without a
// default, so every calendar write was rejected by PostgreSQL while the handler
// still reported success (see issue #1338).
use botschema::{calendar_event_attendees, calendar_events, calendar_shares, calendars};

// #1370 — this crate root used to be a 2685-line file. It is split by
// responsibility: scope (tenant resolution), models (records + payloads),
// convert (iCal and record mapping), handlers (calendar/event CRUD),
// sharing, ui_list/ui_events/ui_views (HTMX fragments), conflicts,
// caldav_api (CalDAV HTTP surface) and tests. The public surface is kept by
// the glob re-exports below.
mod caldav_api;
mod conflicts;
mod convert;
mod handlers;
mod models;
mod scope;
mod sharing;
mod ui_events;
mod ui_list;
mod ui_views;

#[cfg(test)]
mod tests;

pub(crate) use caldav_api::*;
pub use conflicts::*;
pub use convert::*;
pub use handlers::*;
pub use models::*;
pub use scope::*;
pub use sharing::*;
pub use ui_events::*;
pub use ui_list::*;
pub use ui_views::*;
