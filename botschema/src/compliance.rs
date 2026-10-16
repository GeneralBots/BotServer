use crate::core::{branches};

diesel::table! {
    legal_documents (id) {
        id -> Uuid,
        branch_id -> Uuid,
        slug -> Varchar,
        title -> Varchar,
        content -> Text,
        document_type -> Varchar,
        version -> Varchar,
        effective_date -> Timestamptz,
        is_active -> Bool,
        requires_acceptance -> Bool,
        metadata -> Jsonb,
        created_by -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    legal_document_versions (id) {
        id -> Uuid,
        document_id -> Uuid,
        version -> Varchar,
        content -> Text,
        change_summary -> Nullable<Text>,
        created_by -> Nullable<Uuid>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    cookie_consents (id) {
        id -> Uuid,
        branch_id -> Uuid,
        user_id -> Nullable<Uuid>,
        session_id -> Nullable<Varchar>,
        ip_address -> Nullable<Varchar>,
        user_agent -> Nullable<Text>,
        country_code -> Nullable<Varchar>,
        consent_necessary -> Bool,
        consent_analytics -> Bool,
        consent_marketing -> Bool,
        consent_preferences -> Bool,
        consent_functional -> Bool,
        consent_version -> Varchar,
        consent_given_at -> Timestamptz,
        consent_updated_at -> Timestamptz,
        consent_withdrawn_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    consent_history (id) {
        id -> Uuid,
        consent_id -> Uuid,
        action -> Varchar,
        previous_consents -> Jsonb,
        new_consents -> Jsonb,
        ip_address -> Nullable<Varchar>,
        user_agent -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    legal_acceptances (id) {
        id -> Uuid,
        branch_id -> Uuid,
        user_id -> Uuid,
        document_id -> Uuid,
        document_version -> Varchar,
        accepted_at -> Timestamptz,
        ip_address -> Nullable<Varchar>,
        user_agent -> Nullable<Text>,
    }
}

diesel::table! {
    data_deletion_requests (id) {
        id -> Uuid,
        branch_id -> Uuid,
        user_id -> Uuid,
        request_type -> Varchar,
        status -> Varchar,
        reason -> Nullable<Text>,
        requested_at -> Timestamptz,
        scheduled_for -> Nullable<Timestamptz>,
        completed_at -> Nullable<Timestamptz>,
        confirmation_token -> Varchar,
        confirmed_at -> Nullable<Timestamptz>,
        processed_by -> Nullable<Uuid>,
        notes -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    data_export_requests (id) {
        id -> Uuid,
        branch_id -> Uuid,
        user_id -> Uuid,
        status -> Varchar,
        format -> Varchar,
        include_sections -> Jsonb,
        requested_at -> Timestamptz,
        started_at -> Nullable<Timestamptz>,
        completed_at -> Nullable<Timestamptz>,
        file_url -> Nullable<Text>,
        file_size -> Nullable<Int4>,
        expires_at -> Nullable<Timestamptz>,
        error_message -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    compliance_checks (id) {
        id -> Uuid,
        branch_id -> Uuid,
        framework -> Varchar,
        control_id -> Varchar,
        control_name -> Varchar,
        status -> Varchar,
        score -> Numeric,
        checked_at -> Timestamptz,
        checked_by -> Nullable<Uuid>,
        evidence -> Jsonb,
        notes -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    compliance_issues (id) {
        id -> Uuid,
        org_id -> Uuid,
        bot_id -> Uuid,
        check_id -> Nullable<Uuid>,
        severity -> Varchar,
        title -> Varchar,
        description -> Text,
        remediation -> Nullable<Text>,
        due_date -> Nullable<Timestamptz>,
        assigned_to -> Nullable<Uuid>,
        status -> Varchar,
        resolved_at -> Nullable<Timestamptz>,
        resolved_by -> Nullable<Uuid>,
        resolution_notes -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        branch_id -> Uuid,
    }
}

diesel::table! {
    compliance_audit_log (id) {
        id -> Uuid,
        org_id -> Uuid,
        bot_id -> Uuid,
        event_type -> Varchar,
        user_id -> Nullable<Uuid>,
        resource_type -> Varchar,
        resource_id -> Varchar,
        action -> Varchar,
        result -> Varchar,
        ip_address -> Nullable<Varchar>,
        user_agent -> Nullable<Text>,
        metadata -> Jsonb,
        created_at -> Timestamptz,
        branch_id -> Uuid,
    }
}

diesel::table! {
    compliance_evidence (id) {
        id -> Uuid,
        org_id -> Uuid,
        bot_id -> Uuid,
        check_id -> Nullable<Uuid>,
        issue_id -> Nullable<Uuid>,
        evidence_type -> Varchar,
        title -> Varchar,
        description -> Nullable<Text>,
        file_url -> Nullable<Text>,
        file_name -> Nullable<Varchar>,
        file_size -> Nullable<Int4>,
        mime_type -> Nullable<Varchar>,
        metadata -> Jsonb,
        collected_at -> Timestamptz,
        collected_by -> Nullable<Uuid>,
        created_at -> Timestamptz,
        branch_id -> Uuid,
    }
}

diesel::table! {
    compliance_risk_assessments (id) {
        id -> Uuid,
        org_id -> Uuid,
        bot_id -> Uuid,
        title -> Varchar,
        assessor_id -> Uuid,
        methodology -> Varchar,
        overall_risk_score -> Numeric,
        status -> Varchar,
        started_at -> Timestamptz,
        completed_at -> Nullable<Timestamptz>,
        next_review_date -> Nullable<Date>,
        notes -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        branch_id -> Uuid,
    }
}

diesel::table! {
    compliance_risks (id) {
        id -> Uuid,
        assessment_id -> Uuid,
        title -> Varchar,
        description -> Nullable<Text>,
        category -> Varchar,
        likelihood_score -> Int4,
        impact_score -> Int4,
        risk_score -> Int4,
        risk_level -> Varchar,
        current_controls -> Jsonb,
        treatment_strategy -> Varchar,
        status -> Varchar,
        owner_id -> Nullable<Uuid>,
        due_date -> Nullable<Date>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    compliance_training_records (id) {
        id -> Uuid,
        org_id -> Uuid,
        bot_id -> Uuid,
        user_id -> Uuid,
        training_type -> Varchar,
        training_name -> Varchar,
        provider -> Nullable<Varchar>,
        score -> Nullable<Int4>,
        passed -> Bool,
        completion_date -> Timestamptz,
        valid_until -> Nullable<Timestamptz>,
        certificate_url -> Nullable<Text>,
        metadata -> Jsonb,
        created_at -> Timestamptz,
        branch_id -> Uuid,
    }
}

diesel::table! {
    compliance_access_reviews (id) {
        id -> Uuid,
        org_id -> Uuid,
        bot_id -> Uuid,
        user_id -> Uuid,
        reviewer_id -> Uuid,
        review_date -> Timestamptz,
        permissions_reviewed -> Jsonb,
        anomalies -> Jsonb,
        recommendations -> Jsonb,
        status -> Varchar,
        approved_at -> Nullable<Timestamptz>,
        notes -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        branch_id -> Uuid,
    }
}

diesel::joinable!(legal_documents -> branches (branch_id));
diesel::joinable!(legal_document_versions -> legal_documents (document_id));
diesel::joinable!(cookie_consents -> branches (branch_id));
diesel::joinable!(consent_history -> cookie_consents (consent_id));
diesel::joinable!(legal_acceptances -> branches (branch_id));
diesel::joinable!(legal_acceptances -> legal_documents (document_id));
diesel::joinable!(data_deletion_requests -> branches (branch_id));
diesel::joinable!(data_export_requests -> branches (branch_id));

diesel::joinable!(compliance_checks -> branches (branch_id));
diesel::joinable!(compliance_issues -> branches (branch_id));
diesel::joinable!(compliance_issues -> compliance_checks (check_id));
diesel::joinable!(compliance_audit_log -> branches (branch_id));
diesel::joinable!(compliance_evidence -> branches (branch_id));
diesel::joinable!(compliance_risk_assessments -> branches (branch_id));
diesel::joinable!(compliance_risks -> compliance_risk_assessments (assessment_id));
diesel::joinable!(compliance_training_records -> branches (branch_id));
diesel::joinable!(compliance_access_reviews -> branches (branch_id));
