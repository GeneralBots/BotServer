diesel::table! {
    global_email_signatures (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        content -> Text,
        is_default -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        content_html -> Text,
        content_plain -> Text,
        position -> Varchar,
        is_active -> Bool,
    }
}

// Canonical declaration lives in `botschema` (#1497); column order matches the
// crate's `Queryable` structs, which map positionally.
pub use botschema::user_email_accounts;

diesel::table! {
    email_drafts (id) {
        id -> Uuid,
        branch_id -> Uuid,
        subject -> Nullable<Varchar>,
        body -> Nullable<Text>,
        recipient -> Nullable<Varchar>,
        cc -> Nullable<Varchar>,
        bcc -> Nullable<Varchar>,
        attachments -> Nullable<Jsonb>,
        saved_at -> Timestamptz,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        user_id -> Uuid,
        account_id -> Uuid,
        to_address -> Text,
        cc_address -> Nullable<Text>,
        bcc_address -> Nullable<Text>,
    }
}

pub use botschema::email_folders;

diesel::table! {
    email_signatures (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        content -> Text,
        is_default -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        user_id -> Uuid,
        content_html -> Text,
        content_plain -> Text,
        is_active -> Bool,
    }
}

diesel::table! {
    scheduled_emails (id) {
        id -> Uuid,
        branch_id -> Uuid,
        subject -> Varchar,
        body -> Text,
        recipient -> Nullable<Varchar>,
        cc -> Nullable<Varchar>,
        bcc -> Nullable<Varchar>,
        send_at -> Timestamptz,
        sent -> Nullable<Bool>,
        sent_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        user_id -> Uuid,
        to_addresses -> Text,
        cc_addresses -> Nullable<Text>,
        bcc_addresses -> Nullable<Text>,
        body_html -> Text,
        body_plain -> Nullable<Text>,
        attachments_json -> Text,
        scheduled_at -> Timestamptz,
        status -> Varchar,
        retry_count -> Int4,
        error_message -> Nullable<Text>,
    }
}

diesel::table! {
    email_templates (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        subject -> Nullable<Varchar>,
        body -> Text,
        variables -> Nullable<Jsonb>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        user_id -> Nullable<Uuid>,
        description -> Nullable<Text>,
        subject_template -> Text,
        body_html_template -> Text,
        body_plain_template -> Nullable<Text>,
        variables_json -> Text,
        category -> Nullable<Varchar>,
        is_shared -> Bool,
        usage_count -> Int4,
    }
}

diesel::table! {
    email_auto_responders (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        trigger_type -> Varchar,
        trigger_value -> Nullable<Varchar>,
        subject -> Nullable<Varchar>,
        body -> Text,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        user_id -> Uuid,
        responder_type -> Varchar,
        body_html -> Text,
        body_plain -> Nullable<Text>,
        start_date -> Nullable<Timestamptz>,
        end_date -> Nullable<Timestamptz>,
        send_to_internal_only -> Bool,
        exclude_addresses -> Nullable<Text>,
        stalwart_sieve_id -> Nullable<Varchar>,
    }
}

diesel::table! {
    email_rules (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        condition_field -> Varchar,
        condition_operator -> Varchar,
        condition_value -> Nullable<Varchar>,
        action_type -> Varchar,
        action_value -> Nullable<Varchar>,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        user_id -> Uuid,
        priority -> Int4,
        conditions_json -> Text,
        actions_json -> Text,
        stop_processing -> Bool,
        stalwart_sieve_id -> Nullable<Varchar>,
    }
}

diesel::table! {
    email_labels (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        color -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        user_id -> Uuid,
        parent_id -> Nullable<Uuid>,
        is_system -> Bool,
    }
}

pub use botschema::email_label_assignments;

diesel::table! {
    distribution_lists (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        members -> Nullable<Text>,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        owner_id -> Uuid,
        email_alias -> Nullable<Varchar>,
        description -> Nullable<Text>,
        members_json -> Text,
        is_public -> Bool,
        stalwart_principal_id -> Nullable<Varchar>,
    }
}

diesel::table! {
    shared_mailboxes (id) {
        id -> Uuid,
        branch_id -> Uuid,
        email -> Varchar,
        display_name -> Nullable<Varchar>,
        members -> Nullable<Jsonb>,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        email_address -> Varchar,
        description -> Nullable<Text>,
        settings_json -> Text,
        stalwart_account_id -> Nullable<Varchar>,
    }
}

pub use botschema::shared_mailbox_members;

pub use botschema::email_tracking;

diesel::table! {
    feature_flags (id) {
        id -> Uuid,
        branch_id -> Uuid,
        feature -> Varchar,
        enabled -> Bool,
        created_at -> Timestamp,
    }
}

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
