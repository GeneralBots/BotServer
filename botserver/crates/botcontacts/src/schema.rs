// Canonical definition lives in `botschema` (#1497).
pub use botschema::bots;

pub use botschema::organizations;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::crm_contacts;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::crm_accounts;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::crm_pipeline_stages;

pub use botschema::crm_leads;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::crm_opportunities;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::crm_activities;

pub use botschema::crm_notes;

// #1441 P2 — append-only audit trail for destructive/state-changing CRM
// operations (migration 6.7.3-crm-audit-trail).
pub use botschema::crm_audit_logs;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::crm_deals;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::marketing_campaigns;

// Canonical definitions live in `botschema` (#1497): `calendars` there is the
// 14-column shape (the local copy carried a ghost `is_public` and a nullable
// `owner_id`), `calendar_events` the 24-column one (the local copy carried ghost
// `starts_at`/`ends_at`/`is_all_day` and typed `attendees` as text).
pub use botschema::{calendar_events, calendars};

pub use botschema::calendar_event_attendees;

pub use botschema::calendar_event_reminders;

pub use botschema::calendar_shares;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::people::people;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::people_departments;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::tasks;

