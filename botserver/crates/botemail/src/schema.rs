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

diesel::table! {
    crm_contacts (id) {
        id -> Uuid,
        branch_id -> Uuid,
        first_name -> Varchar,
        last_name -> Nullable<Varchar>,
        email -> Nullable<Varchar>,
        phone -> Nullable<Varchar>,
        mobile -> Nullable<Varchar>,
        company -> Nullable<Varchar>,
        job_title -> Nullable<Varchar>,
        source -> Nullable<Varchar>,
        status -> Nullable<Varchar>,
        tags -> Nullable<Text>,
        custom_fields -> Nullable<Jsonb>,
        address_street -> Nullable<Varchar>,
        address_city -> Nullable<Varchar>,
        address_state -> Nullable<Varchar>,
        address_country -> Nullable<Varchar>,
        address_zip -> Nullable<Varchar>,
        notes -> Nullable<Text>,
        owner_id -> Nullable<Uuid>,
        pass_hash -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        address_line1 -> Nullable<Varchar>,
        address_line2 -> Nullable<Varchar>,
        city -> Nullable<Varchar>,
        state -> Nullable<Varchar>,
        postal_code -> Nullable<Varchar>,
        country -> Nullable<Varchar>,
    }
}

diesel::table! {
    email_messages (id) {
        id -> Uuid,
        account_id -> Uuid,
        message_id_header -> Nullable<Varchar>,
        in_reply_to -> Nullable<Varchar>,
        subject -> Text,
        normalized_subject -> Text,
        from_address -> Varchar,
        to_addresses -> Nullable<Text>,
        body_text -> Nullable<Text>,
        body_html -> Nullable<Text>,
        has_attachments -> Bool,
        folder -> Varchar,
        uid -> Int8,
        flags -> Jsonb,
        is_read -> Bool,
        is_flagged -> Bool,
        received_at -> Timestamptz,
        synced_at -> Timestamptz,
    }
}

diesel::table! {
    crm_accounts (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        industry -> Nullable<Varchar>,
        website -> Nullable<Varchar>,
        phone -> Nullable<Varchar>,
        email -> Nullable<Varchar>,
        address_street -> Nullable<Varchar>,
        address_city -> Nullable<Varchar>,
        address_state -> Nullable<Varchar>,
        address_country -> Nullable<Varchar>,
        address_zip -> Nullable<Varchar>,
        notes -> Nullable<Text>,
        owner_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        employees_count -> Nullable<Int4>,
        annual_revenue -> Nullable<Float8>,
        address_line1 -> Nullable<Varchar>,
        address_line2 -> Nullable<Varchar>,
        city -> Nullable<Varchar>,
        state -> Nullable<Varchar>,
        postal_code -> Nullable<Varchar>,
        country -> Nullable<Varchar>,
        description -> Nullable<Text>,
        tags -> Text,
        custom_fields -> Jsonb,
    }
}
