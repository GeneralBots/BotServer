use crate::core::bots;
use crate::core::rbac_groups;
use crate::core::users;

diesel::table! {
    kb_collections (id) {

    id -> Uuid,
    bot_id -> Uuid,
    name -> Text,
    folder_path -> Text,
    qdrant_collection -> Text,
    document_count -> Int4,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    }
}


diesel::table! {
    kb_group_associations (id) {
        id         -> Uuid,
        kb_id      -> Uuid,
        group_id   -> Uuid,
        granted_by -> Nullable<Uuid>,
        granted_at -> Timestamptz,
    }
}

diesel::joinable!(kb_collections -> bots (bot_id));
diesel::joinable!(kb_group_associations -> kb_collections (kb_id));
diesel::joinable!(kb_group_associations -> rbac_groups (group_id));
diesel::joinable!(kb_group_associations -> users (granted_by));

diesel::allow_tables_to_appear_in_same_query!(kb_collections, kb_group_associations,);
