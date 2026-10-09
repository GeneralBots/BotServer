//! Reform #1505 — read, edit and LLM-rephrase an AutoTask-produced `.bas`.
//!
//! An AutoTask item leaves a `.bas` in the bot's repository (the canonical
//! `.gbdialog`). These endpoints let the suite editor open that file, save an
//! edited revision, or ask the LLM to rephrase it while preserving the
//! declared MCP manifest contract. Every write goes through
//! [`crate::types::BotSourceOps`], so it is committed and pushed to ALM and the
//! pull monitor compiles it.

pub(crate) use std::sync::Arc;
pub(crate) use axum::{extract::Path, extract::Query, extract::State, Json};
pub(crate) use diesel::RunQueryDsl;
pub(crate) use log::{info, warn};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use uuid::Uuid;
pub(crate) use crate::api::AutoTaskApi;
pub(crate) use crate::handlers::{canonical_bot_id, err_msg};

/// Query naming only a bot (listing endpoints).
#[derive(Debug, Deserialize)]
pub struct BotQuery {
    pub bot_id: Option<String>,
}

/// Query for reading a source file.
#[derive(Debug, Deserialize)]
pub struct SourceQuery {
    pub bot_id: Option<String>,
    /// File name inside `.gbdialog` (e.g. `classify_media.bas`).
    pub name: String,
}

/// Body for saving an edited revision.
#[derive(Debug, Deserialize)]
pub struct UpdateSourceRequest {
    pub bot_id: Option<String>,
    pub name: String,
    pub content: String,
    /// Commit message override (defaults to a generic edit message).
    pub message: Option<String>,
}

/// Body for the LLM rephrase flow.
#[derive(Debug, Deserialize)]
pub struct RephraseSourceRequest {
    pub bot_id: Option<String>,
    pub name: String,
    /// What the user wants changed (e.g. "file receipts under finance/").
    pub instruction: String,
    /// When true the rewritten source is returned without being committed.
    pub dry_run: Option<bool>,
}

/// Body for attaching a database schema to a task.
#[derive(Debug, Deserialize)]
pub struct AttachTablesRequest {
    pub bot_id: Option<String>,
    /// Raw BASIC `BEGIN TABLE … END TABLE` blocks the task needs.
    pub tables: String,
}

/// Response shared by the three endpoints.
#[derive(Debug, Serialize)]
pub struct SourceResponse {
    pub success: bool,
    pub name: String,
    pub content: Option<String>,
    pub committed: bool,
    pub message: String,
    pub error: Option<String>,
}

/// Response for the source listing.
#[derive(Debug, Serialize)]
pub struct SourceListResponse {
    pub success: bool,
    pub project_id: Option<Uuid>,
    pub files: Vec<crate::types::SourceFile>,
    pub message: String,
    pub error: Option<String>,
}

/// Response for one AutoTask item with its resolved source.
#[derive(Debug, Serialize)]
pub struct ItemSourceResponse {
    pub success: bool,
    pub item_id: String,
    pub name: String,
    /// Vibe project owning the bot's repository (editor's project mode).
    pub project_id: Option<Uuid>,
    pub content: Option<String>,
    pub committed: bool,
    pub message: String,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::strip_code_fence;

    #[test]
    fn strips_fenced_completion() {
        let out = strip_code_fence("```basic\nTALK \"hi\"\n```");
        assert_eq!(out, "TALK \"hi\"");
    }

    #[test]
    fn keeps_unfenced_completion() {
        assert_eq!(strip_code_fence("TALK \"hi\""), "TALK \"hi\"");
    }
}

mod part1;
mod part2;

pub use part1::*;
pub use part2::*;
