pub(crate) use axum::{ extract::{Path, Query, State}, response::{Html, IntoResponse}, routing::{get, post}, Router, };
pub(crate) use chrono::Utc as ChronoUtc;
pub(crate) use diesel::prelude::*;
pub(crate) use serde::Deserialize;
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use crate::{get_bot_context, support_tickets, ticket_comments, SupportTicket, TicketComment, TicketsState};

#[derive(Debug, Deserialize)]
pub struct StatusQuery {
    pub status: Option<String>,
    pub priority: Option<String>,
    pub category: Option<String>,
    pub record_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

#[derive(Deserialize)]
pub struct AiSuggestForm {
    pub subject: Option<String>,
    pub description: Option<String>,
}

mod part1;
mod part2;
mod part3;
mod part4;

pub use part1::*;
use part2::*;
use part3::*;
use part4::*;
