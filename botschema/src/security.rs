//! Identity and KYC tables (canonical, #1497).

diesel::table! {
    identity_profiles (id) {

    id -> Uuid,
    bot_id -> Uuid,
    person_id -> Uuid,
    legal_name -> Text,
    #[max_length = 50]
    tax_id -> Varchar,
    date_of_birth -> Nullable<Date>,
    #[max_length = 50]
    nationality -> Nullable<Varchar>,
    #[max_length = 255]
    email -> Nullable<Varchar>,
    #[max_length = 50]
    phone -> Nullable<Varchar>,
    address -> Nullable<Jsonb>,
    risk_score -> Nullable<Int4>,
    #[max_length = 30]
    kyc_status -> Varchar,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    identity_faces (id) {

    id -> Uuid,
    bot_id -> Uuid,
    profile_id -> Uuid,
    photo_url -> Text,
    embedding -> Nullable<Jsonb>,
    quality_score -> Nullable<Numeric>,
    is_primary -> Bool,
    created_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    identity_documents (id) {

    id -> Uuid,
    bot_id -> Uuid,
    profile_id -> Uuid,
    #[max_length = 30]
    document_type -> Varchar,
    #[max_length = 100]
    document_number -> Varchar,
    #[max_length = 10]
    issuing_country -> Nullable<Varchar>,
    issue_date -> Nullable<Date>,
    expiry_date -> Nullable<Date>,
    front_image_url -> Nullable<Text>,
    back_image_url -> Nullable<Text>,
    selfie_image_url -> Nullable<Text>,
    ocr_data -> Nullable<Jsonb>,
    #[max_length = 30]
    verification_status -> Varchar,
    created_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    identity_signatures (id) {

    id -> Uuid,
    bot_id -> Uuid,
    profile_id -> Uuid,
    document_id -> Uuid,
    signature_data -> Text,
    signature_image_url -> Nullable<Text>,
    #[max_length = 64]
    ip_address -> Nullable<Varchar>,
    user_agent -> Nullable<Text>,
    signed_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    identity_signed_documents (id) {

    id -> Uuid,
    bot_id -> Uuid,
    signature_id -> Uuid,
    #[max_length = 128]
    document_hash -> Varchar,
    document_name -> Text,
    #[max_length = 30]
    signature_algorithm -> Varchar,
    signed_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    identity_kyc_workflows (id) {

    id -> Uuid,
    bot_id -> Uuid,
    profile_id -> Uuid,
    #[max_length = 100]
    workflow_name -> Varchar,
    #[max_length = 100]
    current_step -> Varchar,
    steps_completed -> Jsonb,
    total_steps -> Int4,
    #[max_length = 30]
    status -> Varchar,
    started_at -> Timestamptz,
    completed_at -> Nullable<Timestamptz>,
    branch_id -> Uuid,
    }
}


diesel::allow_tables_to_appear_in_same_query!(
    identity_profiles,
    identity_faces,
    identity_documents,
    identity_signatures,
    identity_signed_documents,
    identity_kyc_workflows,
);
