pub use botschema::global_email_signatures;

// Canonical declaration lives in `botschema` (#1497); column order matches the
// crate's `Queryable` structs, which map positionally.
pub use botschema::user_email_accounts;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::email_drafts;

pub use botschema::email_folders;

// Canonical definition lives in `botschema` (#1497). The local copy carried a
// ghost `content` column and non-null flags; the database has neither.
pub use botschema::email_signatures;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::scheduled_emails;

pub use botschema::email_templates;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::email_auto_responders;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::email_rules;

// Canonical definition lives in `botschema` (#1497). The local copy carried a
// ghost `updated_at`; `bot_id` was missing.
pub use botschema::email_labels;

pub use botschema::email_label_assignments;

pub use botschema::distribution_lists;

// Canonical definition lives in `botschema` (#1497). The local copy carried
// ghost `email`/`members`/`updated_at` columns; `bot_id` was missing.
pub use botschema::shared_mailboxes;

pub use botschema::shared_mailbox_members;

pub use botschema::email_tracking;

// Canonical definition lives in `botschema` (#1497): the database has `org_id`
// and a nullable `branch_id`, which the local copy was missing.
pub use botschema::feature_flags;

pub use botschema::email_crm_links;

pub use botschema::email_campaign_links;

pub use botschema::email_snooze;

pub use botschema::email_flags;

pub use botschema::email_nudges;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::crm_contacts;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::email_messages;


// Canonical definition lives in `botschema` (#1497).
pub use botschema::crm_accounts;
