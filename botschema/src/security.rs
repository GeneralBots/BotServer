//! Identity and KYC tables (canonical, #1497).

diesel::table! {
    identity_profiles (id) {
        id -> Uuid,
        branch_id -> Uuid,
        person_id -> Nullable<Uuid>,
        email -> Nullable<Varchar>,
        phone -> Nullable<Varchar>,
        verification_level -> Nullable<Varchar>,
        metadata -> Nullable<Jsonb>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    identity_faces (id) {
        id -> Uuid,
        branch_id -> Uuid,
        profile_id -> Uuid,
        image_hash -> Varchar,
        encoding -> Nullable<Bytea>,
        verified -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    identity_documents (id) {
        id -> Uuid,
        branch_id -> Uuid,
        profile_id -> Uuid,
        document_type -> Varchar,
        document_number -> Nullable<Varchar>,
        file_path -> Nullable<Varchar>,
        verified -> Nullable<Bool>,
        verified_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    identity_signatures (id) {
        id -> Uuid,
        branch_id -> Uuid,
        profile_id -> Nullable<Uuid>,
        signature_hash -> Varchar,
        signed_at -> Timestamptz,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    identity_signed_documents (id) {
        id -> Uuid,
        branch_id -> Uuid,
        signature_id -> Uuid,
        document_content -> Text,
        document_hash -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    identity_kyc_workflows (id) {
        id -> Uuid,
        branch_id -> Uuid,
        profile_id -> Uuid,
        workflow_type -> Varchar,
        status -> Nullable<Varchar>,
        steps -> Array<Jsonb>,
        completed_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
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
