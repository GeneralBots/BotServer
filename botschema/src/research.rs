use crate::core::{branches};

diesel::table! {
    kb_documents (id) {

    id -> Uuid,
    bot_id -> Uuid,
    collection_name -> Text,
    file_path -> Text,
    file_size -> Int8,
    file_hash -> Text,
    first_published_at -> Timestamptz,
    last_modified_at -> Timestamptz,
    indexed_at -> Nullable<Timestamptz>,
    fail_count -> Int4,
    last_failed_at -> Nullable<Timestamptz>,
    metadata -> Nullable<Jsonb>,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    }
}


diesel::table! {
    user_kb_associations (id) {

    id -> Text,
    user_id -> Text,
    bot_id -> Text,
    kb_name -> Text,
    is_website -> Int4,
    website_url -> Nullable<Text>,
    created_at -> Text,
    updated_at -> Text,
    branch_id -> Uuid,
    }
}


diesel::table! {
    research_projects (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 255]
    name -> Varchar,
    description -> Nullable<Text>,
    #[max_length = 50]
    status -> Varchar,
    owner_id -> Uuid,
    tags -> Array<Nullable<Text>>,
    settings -> Jsonb,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    research_sources (id) {
        id -> Uuid,
        project_id -> Uuid,
        source_type -> Varchar,
        name -> Varchar,
        url -> Nullable<Text>,
        content -> Nullable<Text>,
        summary -> Nullable<Text>,
        metadata -> Jsonb,
        credibility_score -> Nullable<Int4>,
        is_verified -> Bool,
        added_by -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    research_notes (id) {
        id -> Uuid,
        project_id -> Uuid,
        source_id -> Nullable<Uuid>,
        title -> Nullable<Varchar>,
        content -> Text,
        note_type -> Varchar,
        tags -> Array<Nullable<Text>>,
        highlight_text -> Nullable<Text>,
        highlight_position -> Nullable<Jsonb>,
        created_by -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    research_findings (id) {
        id -> Uuid,
        project_id -> Uuid,
        title -> Varchar,
        content -> Text,
        finding_type -> Varchar,
        confidence_level -> Nullable<Varchar>,
        supporting_sources -> Jsonb,
        related_findings -> Jsonb,
        status -> Varchar,
        created_by -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    research_citations (id) {
        id -> Uuid,
        source_id -> Uuid,
        citation_style -> Varchar,
        formatted_citation -> Text,
        bibtex -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    research_collaborators (id) {
        id -> Uuid,
        project_id -> Uuid,
        user_id -> Uuid,
        role -> Varchar,
        invited_by -> Nullable<Uuid>,
        joined_at -> Timestamptz,
    }
}

diesel::table! {
    research_exports (id) {
        id -> Uuid,
        project_id -> Uuid,
        export_type -> Varchar,
        format -> Varchar,
        file_url -> Nullable<Text>,
        file_size -> Nullable<Int4>,
        status -> Varchar,
        created_by -> Uuid,
        created_at -> Timestamptz,
        completed_at -> Nullable<Timestamptz>,
    }
}

diesel::joinable!(research_projects -> branches (branch_id));
diesel::joinable!(research_sources -> research_projects (project_id));
diesel::joinable!(research_notes -> research_projects (project_id));
diesel::joinable!(research_findings -> research_projects (project_id));
diesel::joinable!(research_citations -> research_sources (source_id));
diesel::joinable!(research_collaborators -> research_projects (project_id));
diesel::joinable!(research_exports -> research_projects (project_id));

diesel::allow_tables_to_appear_in_same_query!(
    kb_documents,
    user_kb_associations,
    research_projects,
    research_sources,
    research_notes,
    research_findings,
    research_citations,
    research_collaborators,
    research_exports,
);
