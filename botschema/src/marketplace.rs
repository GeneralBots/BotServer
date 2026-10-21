//! Skill marketplace tables (canonical, #1497).

diesel::table! {
    skill_packages (id) {

    id -> Uuid,
    #[max_length = 80]
    slug -> Varchar,
    #[max_length = 160]
    name -> Varchar,
    description -> Nullable<Text>,
    #[max_length = 32]
    latest_version -> Nullable<Varchar>,
    publisher_org_id -> Nullable<Uuid>,
    #[max_length = 160]
    publisher_name -> Nullable<Varchar>,
    #[max_length = 16]
    visibility -> Varchar,
    #[max_length = 16]
    review_status -> Varchar,
    downloads -> Int8,
    #[max_length = 8]
    icon_glyph -> Nullable<Varchar>,
    tags -> Jsonb,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    }
}


diesel::table! {
    skill_versions (id) {

    id -> Uuid,
    package_id -> Uuid,
    #[max_length = 32]
    version -> Varchar,
    manifest -> Jsonb,
    object_key -> Text,
    changelog -> Nullable<Text>,
    created_at -> Timestamptz,
    }
}


diesel::table! {
    skill_installs (id) {

    id -> Uuid,
    package_id -> Uuid,
    version_id -> Uuid,
    org_id -> Nullable<Uuid>,
    branch_id -> Nullable<Uuid>,
    bot_id -> Uuid,
    installed_by -> Nullable<Uuid>,
    #[max_length = 24]
    status -> Varchar,
    installed_at -> Timestamptz,
    }
}


diesel::joinable!(skill_versions -> skill_packages (package_id));
diesel::joinable!(skill_installs -> skill_packages (package_id));
diesel::joinable!(skill_installs -> skill_versions (version_id));

diesel::allow_tables_to_appear_in_same_query!(skill_packages, skill_versions, skill_installs);
