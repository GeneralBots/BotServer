pub(crate) use chrono::{DateTime, Duration, Utc};
pub(crate) use rhai::{Array, Dynamic, Engine, Map};
pub(crate) use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: Uuid,

    pub bot_id: Uuid,

    pub session_id: Uuid,

    pub initiated_by: Uuid,

    pub approval_type: String,

    pub status: ApprovalStatus,

    pub channel: ApprovalChannel,

    pub recipient: String,

    pub context: serde_json::Value,

    pub message: String,

    pub timeout_seconds: u64,

    pub default_action: Option<ApprovalDecision>,

    pub current_level: u32,

    pub total_levels: u32,

    pub created_at: DateTime<Utc>,

    pub expires_at: DateTime<Utc>,

    pub reminders_sent: Vec<DateTime<Utc>>,

    pub decision: Option<ApprovalDecision>,

    pub decided_by: Option<String>,

    pub decided_at: Option<DateTime<Utc>>,

    pub comments: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ApprovalStatus {
    #[default]
    Pending,

    Approved,

    Rejected,

    TimedOut,

    Cancelled,

    Escalated,

    Error,
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalDecision {
    Approve,
    Reject,
    Escalate,
    Defer,
    RequestInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ApprovalChannel {
    #[default]
    Email,
    Sms,
    Mobile,
    Teams,
    Slack,
    Webhook,
    InApp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalChain {
    pub name: String,

    pub bot_id: Uuid,

    pub levels: Vec<ApprovalLevel>,

    pub stop_on_reject: bool,

    pub require_all: bool,

    pub description: Option<String>,

    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalLevel {
    pub level: u32,

    pub channel: ApprovalChannel,

    pub recipient: String,

    pub timeout_seconds: u64,

    pub condition: Option<String>,

    pub skippable: bool,

    pub approvers: Vec<String>,

    pub required_approvals: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalAuditEntry {
    pub id: Uuid,

    pub request_id: Uuid,

    pub action: AuditAction,

    pub actor: String,

    pub details: serde_json::Value,

    pub timestamp: DateTime<Utc>,

    pub ip_address: Option<String>,

    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    RequestCreated,
    NotificationSent,
    ReminderSent,
    Approved,
    Rejected,
    Escalated,
    TimedOut,
    Cancelled,
    CommentAdded,
    ContextUpdated,
}

#[derive(Debug, Clone)]
pub struct ApprovalConfig {
    pub enabled: bool,

    pub default_timeout: u64,

    pub reminder_interval: u64,

    pub max_reminders: u32,

    pub audit_enabled: bool,

    pub webhook_url: Option<String>,

    pub email_template: Option<String>,

    pub approval_base_url: Option<String>,
}

pub struct CreateApprovalRequestParams<'a> {
    pub bot_id: Uuid,
    pub session_id: Uuid,
    pub initiated_by: Uuid,
    pub approval_type: &'a str,
    pub channel: ApprovalChannel,
    pub recipient: &'a str,
    pub context: serde_json::Value,
    pub message: &'a str,
    pub timeout_seconds: Option<u64>,
    pub default_action: Option<ApprovalDecision>,
}

#[derive(Debug)]
pub struct ApprovalManager {
    config: ApprovalConfig,
}

#[derive(Debug, Clone)]
pub struct EmailContent {
    pub subject: String,
    pub body: String,
    pub html_body: Option<String>,
}

pub const APPROVAL_SCHEMA: &str = r"
-- Approval requests
CREATE TABLE IF NOT EXISTS approval_requests (
    id UUID PRIMARY KEY,
    bot_id UUID NOT NULL,
    session_id UUID NOT NULL,
    initiated_by UUID NOT NULL,
    approval_type VARCHAR(100) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    channel VARCHAR(50) NOT NULL,
    recipient VARCHAR(500) NOT NULL,
    context JSONB NOT NULL DEFAULT '{}',
    message TEXT NOT NULL,
    timeout_seconds INTEGER NOT NULL DEFAULT 3600,
    default_action VARCHAR(50),
    current_level INTEGER NOT NULL DEFAULT 1,
    total_levels INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    reminders_sent JSONB NOT NULL DEFAULT '[]',
    decision VARCHAR(50),
    decided_by VARCHAR(500),
    decided_at TIMESTAMP WITH TIME ZONE,
    comments TEXT
);

-- Approval chains
CREATE TABLE IF NOT EXISTS approval_chains (
    id UUID PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    bot_id UUID NOT NULL,
    levels JSONB NOT NULL DEFAULT '[]',
    stop_on_reject BOOLEAN NOT NULL DEFAULT true,
    require_all BOOLEAN NOT NULL DEFAULT false,
    description TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    UNIQUE(bot_id, name)
);

-- Approval audit log
CREATE TABLE IF NOT EXISTS approval_audit_log (
    id UUID PRIMARY KEY,
    request_id UUID NOT NULL REFERENCES approval_requests(id) ON DELETE CASCADE,
    action VARCHAR(50) NOT NULL,
    actor VARCHAR(500) NOT NULL,
    details JSONB NOT NULL DEFAULT '{}',
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    ip_address VARCHAR(50),
    user_agent TEXT
);

-- Approval tokens (for secure links)
CREATE TABLE IF NOT EXISTS approval_tokens (
    id UUID PRIMARY KEY,
    request_id UUID NOT NULL REFERENCES approval_requests(id) ON DELETE CASCADE,
    token VARCHAR(100) NOT NULL UNIQUE,
    action VARCHAR(50) NOT NULL,
    used BOOLEAN NOT NULL DEFAULT false,
    used_at TIMESTAMP WITH TIME ZONE,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_approval_requests_bot_id ON approval_requests(bot_id);
CREATE INDEX IF NOT EXISTS idx_approval_requests_session_id ON approval_requests(session_id);
CREATE INDEX IF NOT EXISTS idx_approval_requests_status ON approval_requests(status);
CREATE INDEX IF NOT EXISTS idx_approval_requests_expires_at ON approval_requests(expires_at);
CREATE INDEX IF NOT EXISTS idx_approval_requests_pending ON approval_requests(status, expires_at)
    WHERE status = 'pending';

CREATE INDEX IF NOT EXISTS idx_approval_audit_request_id ON approval_audit_log(request_id);
CREATE INDEX IF NOT EXISTS idx_approval_audit_timestamp ON approval_audit_log(timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_approval_tokens_token ON approval_tokens(token);
CREATE INDEX IF NOT EXISTS idx_approval_tokens_request_id ON approval_tokens(request_id);
";

pub mod sql {
    pub const INSERT_REQUEST: &str = r"
        INSERT INTO approval_requests (
            id, bot_id, session_id, initiated_by, approval_type, status,
            channel, recipient, context, message, timeout_seconds,
            default_action, current_level, total_levels, created_at,
            expires_at, reminders_sent
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17
        )
    ";

    pub const UPDATE_REQUEST: &str = r"
        UPDATE approval_requests
        SET status = $2,
            decision = $3,
            decided_by = $4,
            decided_at = $5,
            comments = $6
        WHERE id = $1
    ";

    pub const GET_REQUEST: &str = r"
        SELECT * FROM approval_requests WHERE id = $1
    ";

    pub const GET_PENDING_REQUESTS: &str = r"
        SELECT * FROM approval_requests
        WHERE status = 'pending'
        AND expires_at > NOW()
        ORDER BY created_at ASC
    ";

    pub const GET_EXPIRED_REQUESTS: &str = r"
        SELECT * FROM approval_requests
        WHERE status = 'pending'
        AND expires_at <= NOW()
    ";

    pub const GET_REQUESTS_BY_SESSION: &str = r"
        SELECT * FROM approval_requests
        WHERE session_id = $1
        ORDER BY created_at DESC
    ";

    pub const UPDATE_REMINDERS: &str = r"
        UPDATE approval_requests
        SET reminders_sent = reminders_sent || $2::jsonb
        WHERE id = $1
    ";

    pub const INSERT_AUDIT: &str = r"
        INSERT INTO approval_audit_log (
            id, request_id, action, actor, details, timestamp, ip_address, user_agent
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8
        )
    ";

    pub const GET_AUDIT_LOG: &str = r"
        SELECT * FROM approval_audit_log
        WHERE request_id = $1
        ORDER BY timestamp ASC
    ";

    pub const INSERT_TOKEN: &str = r"
        INSERT INTO approval_tokens (
            id, request_id, token, action, expires_at, created_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6
        )
    ";

    pub const GET_TOKEN: &str = r"
        SELECT * FROM approval_tokens
        WHERE token = $1 AND used = false AND expires_at > NOW()
    ";

    pub const USE_TOKEN: &str = r"
        UPDATE approval_tokens
        SET used = true, used_at = NOW()
        WHERE token = $1
    ";

    pub const INSERT_CHAIN: &str = r"
        INSERT INTO approval_chains (
            id, name, bot_id, levels, stop_on_reject, require_all, description, created_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8
        )
        ON CONFLICT (bot_id, name)
        DO UPDATE SET
            levels = $4,
            stop_on_reject = $5,
            require_all = $6,
            description = $7
    ";

    pub const GET_CHAIN: &str = r"
        SELECT * FROM approval_chains
        WHERE bot_id = $1 AND name = $2
    ";
}

use std::collections::HashMap;
use log::info;
use uuid::Uuid;

mod part1;
mod part2;
mod part3;
mod part4;

pub use part4::*;
