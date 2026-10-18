//! Skill marketplace tables (canonical, #1497).

diesel::table! {
    skill_packages (id) {
        id -> Uuid,
        slug -> Text,
        name -> Text,
        description -> Nullable<Text>,
        latest_version -> Nullable<Text>,
        publisher_org_id -> Nullable<Uuid>,
        publisher_name -> Nullable<Text>,
        visibility -> Text,
        review_status -> Text,
        downloads -> BigInt,
        icon_glyph -> Nullable<Text>,
        tags -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    skill_versions (id) {
        id -> Uuid,
        package_id -> Uuid,
        version -> Text,
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
        status -> Text,
        installed_at -> Timestamptz,
    }
}

diesel::joinable!(skill_versions -> skill_packages (package_id));
diesel::joinable!(skill_installs -> skill_packages (package_id));
diesel::joinable!(skill_installs -> skill_versions (version_id));

diesel::allow_tables_to_appear_in_same_query!(skill_packages, skill_versions, skill_installs);
