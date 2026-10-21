// use crate::core::{branches};

diesel::table! {
    global_email_signatures (id) {

    id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 100]
    name -> Varchar,
    content_html -> Text,
    content_plain -> Text,
    #[max_length = 20]
    position -> Nullable<Varchar>,
    is_active -> Nullable<Bool>,
    created_at -> Nullable<Timestamptz>,
    updated_at -> Nullable<Timestamptz>,
    branch_id -> Uuid,
    }
}


diesel::table! {
    user_email_accounts (id) {
        id -> Uuid,
        user_id -> Uuid,
        email -> Varchar,
        display_name -> Nullable<Varchar>,
        imap_server -> Varchar,
        imap_port -> Int4,
        smtp_server -> Varchar,
        smtp_port -> Int4,
        username -> Varchar,
        password_encrypted -> Text,
        is_primary -> Bool,
        is_active -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        auth_mode -> Varchar,
        oauth_provider -> Nullable<Varchar>,
        refresh_token_encrypted -> Nullable<Text>,
        access_token_encrypted -> Nullable<Text>,
        token_expires_at -> Nullable<Timestamptz>,
        last_sync_at -> Nullable<Timestamptz>,
        last_error -> Nullable<Text>,
        last_error_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    email_drafts (id) {

    id -> Uuid,
    user_id -> Uuid,
    account_id -> Uuid,
    to_address -> Text,
    cc_address -> Nullable<Text>,
    bcc_address -> Nullable<Text>,
    #[max_length = 500]
    subject -> Nullable<Varchar>,
    body -> Nullable<Text>,
    attachments -> Jsonb,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    email_folders (id) {
        id -> Uuid,
        account_id -> Uuid,
        folder_name -> Varchar,
        folder_path -> Varchar,
        unread_count -> Int4,
        total_count -> Int4,
        last_synced -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

// `6.5.15.1-consolidated` adds `branch_id`/`bot_id` (nullable) to the 9-column
// shape created for signatures, and the flags/timestamps are nullable there.
diesel::table! {
    email_signatures (id) {

    id -> Uuid,
    user_id -> Uuid,
    bot_id -> Nullable<Uuid>,
    #[max_length = 100]
    name -> Varchar,
    content_html -> Text,
    content_plain -> Text,
    is_default -> Nullable<Bool>,
    is_active -> Nullable<Bool>,
    created_at -> Nullable<Timestamptz>,
    updated_at -> Nullable<Timestamptz>,
    branch_id -> Uuid,
    }
}


diesel::table! {
    scheduled_emails (id) {

    id -> Uuid,
    user_id -> Uuid,
    bot_id -> Uuid,
    to_addresses -> Text,
    cc_addresses -> Nullable<Text>,
    bcc_addresses -> Nullable<Text>,
    subject -> Text,
    body_html -> Text,
    body_plain -> Nullable<Text>,
    attachments_json -> Nullable<Text>,
    scheduled_at -> Timestamptz,
    sent_at -> Nullable<Timestamptz>,
    #[max_length = 20]
    status -> Nullable<Varchar>,
    retry_count -> Nullable<Int4>,
    error_message -> Nullable<Text>,
    created_at -> Nullable<Timestamptz>,
    branch_id -> Uuid,
    }
}


diesel::table! {
    email_templates (id) {

    id -> Uuid,
    bot_id -> Uuid,
    user_id -> Nullable<Uuid>,
    #[max_length = 255]
    name -> Varchar,
    description -> Nullable<Text>,
    subject_template -> Text,
    body_html_template -> Text,
    body_plain_template -> Nullable<Text>,
    variables_json -> Nullable<Text>,
    #[max_length = 100]
    category -> Nullable<Varchar>,
    is_shared -> Nullable<Bool>,
    usage_count -> Nullable<Int4>,
    created_at -> Nullable<Timestamptz>,
    updated_at -> Nullable<Timestamptz>,
    branch_id -> Uuid,
    }
}


diesel::table! {
    email_auto_responders (id) {

    id -> Uuid,
    user_id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 50]
    responder_type -> Varchar,
    subject -> Text,
    body_html -> Text,
    body_plain -> Nullable<Text>,
    start_date -> Nullable<Timestamptz>,
    end_date -> Nullable<Timestamptz>,
    send_to_internal_only -> Nullable<Bool>,
    exclude_addresses -> Nullable<Text>,
    is_active -> Nullable<Bool>,
    #[max_length = 255]
    stalwart_sieve_id -> Nullable<Varchar>,
    created_at -> Nullable<Timestamptz>,
    updated_at -> Nullable<Timestamptz>,
    branch_id -> Uuid,
    }
}


diesel::table! {
    email_rules (id) {

    id -> Uuid,
    user_id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 255]
    name -> Varchar,
    priority -> Nullable<Int4>,
    conditions_json -> Text,
    actions_json -> Text,
    stop_processing -> Nullable<Bool>,
    is_active -> Nullable<Bool>,
    #[max_length = 255]
    stalwart_sieve_id -> Nullable<Varchar>,
    created_at -> Nullable<Timestamptz>,
    updated_at -> Nullable<Timestamptz>,
    branch_id -> Uuid,
    }
}


// `6.5.15.1-consolidated` adds `branch_id`/`bot_id` (nullable) and makes the
// colour, the system flag and the timestamp nullable.
diesel::table! {
    email_labels (id) {

    id -> Uuid,
    user_id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 100]
    name -> Varchar,
    #[max_length = 7]
    color -> Nullable<Varchar>,
    parent_id -> Nullable<Uuid>,
    is_system -> Nullable<Bool>,
    created_at -> Nullable<Timestamptz>,
    branch_id -> Uuid,
    }
}


diesel::table! {
    email_label_assignments (id) {

    id -> Uuid,
    #[max_length = 255]
    email_message_id -> Varchar,
    label_id -> Uuid,
    assigned_at -> Nullable<Timestamptz>,
    }
}


diesel::table! {
    distribution_lists (id) {

    id -> Uuid,
    bot_id -> Uuid,
    owner_id -> Uuid,
    #[max_length = 255]
    name -> Varchar,
    #[max_length = 255]
    email_alias -> Nullable<Varchar>,
    description -> Nullable<Text>,
    members_json -> Text,
    is_public -> Nullable<Bool>,
    created_at -> Nullable<Timestamptz>,
    updated_at -> Nullable<Timestamptz>,
    branch_id -> Uuid,
    }
}


// `6.5.15.1-consolidated` adds `branch_id`/`bot_id` (nullable); `display_name`
// is nullable in the database and `settings_json`/`is_active`/`created_at` are
// nullable there too (the table is created by the mail bootstrap first).
diesel::table! {
    shared_mailboxes (id) {

    id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 255]
    email_address -> Varchar,
    #[max_length = 255]
    display_name -> Varchar,
    description -> Nullable<Text>,
    settings_json -> Nullable<Text>,
    #[max_length = 255]
    stalwart_account_id -> Nullable<Varchar>,
    is_active -> Nullable<Bool>,
    created_at -> Nullable<Timestamptz>,
    branch_id -> Uuid,
    }
}


diesel::table! {
    shared_mailbox_members (id) {

    id -> Uuid,
    mailbox_id -> Uuid,
    user_id -> Uuid,
    #[max_length = 20]
    permission_level -> Nullable<Varchar>,
    added_at -> Nullable<Timestamptz>,
    }
}


// Email sync (canonical, #1497).

diesel::table! {
    email_messages (id) {

    id -> Uuid,
    account_id -> Uuid,
    #[max_length = 512]
    message_id_header -> Nullable<Varchar>,
    #[max_length = 512]
    in_reply_to -> Nullable<Varchar>,
    subject -> Text,
    normalized_subject -> Text,
    #[max_length = 512]
    from_address -> Varchar,
    to_addresses -> Nullable<Text>,
    body_text -> Nullable<Text>,
    body_html -> Nullable<Text>,
    has_attachments -> Nullable<Bool>,
    #[max_length = 255]
    folder -> Varchar,
    uid -> Int8,
    flags -> Nullable<Jsonb>,
    is_read -> Nullable<Bool>,
    is_flagged -> Nullable<Bool>,
    received_at -> Timestamptz,
    synced_at -> Timestamptz,
    }
}
