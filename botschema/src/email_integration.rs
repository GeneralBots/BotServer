// Email integration schema tables

diesel::table! {
    feature_flags (id) {
        id -> Uuid,
        org_id -> Uuid,
        branch_id -> Nullable<Uuid>,
        feature -> Varchar,
        enabled -> Bool,
        created_at -> Timestamp,
    }
}

diesel::table! {
    email_crm_links (id) {
        id -> Uuid,
        email_id -> Uuid,
        contact_id -> Nullable<Uuid>,
        opportunity_id -> Nullable<Uuid>,
        logged_at -> Timestamp,
    }
}

diesel::table! {
    email_campaign_links (id) {
        id -> Uuid,
        email_id -> Uuid,
        campaign_id -> Nullable<Uuid>,
        list_id -> Nullable<Uuid>,
        sent_at -> Timestamp,
    }
}

diesel::table! {
    email_snooze (id) {
        id -> Uuid,
        email_id -> Uuid,
        snooze_until -> Timestamp,
        created_at -> Timestamp,
    }
}

diesel::table! {
    email_flags (id) {

    id -> Uuid,
    email_id -> Uuid,
    #[max_length = 255]
    follow_up_date -> Nullable<Varchar>,
    #[max_length = 255]
    flag_type -> Nullable<Varchar>,
    completed -> Bool,
    created_at -> Timestamp,
    }
}


diesel::table! {
    email_nudges (id) {
        id -> Uuid,
        email_id -> Uuid,
        last_sent -> Nullable<Timestamp>,
        dismissed -> Bool,
        created_at -> Timestamp,
    }
}
