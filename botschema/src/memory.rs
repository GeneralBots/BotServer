//! Long-term user memory (canonical, #1497).

diesel::table! {
    user_memories (id) {
        id -> Uuid,
        org_id -> Nullable<Uuid>,
        branch_id -> Nullable<Uuid>,
        owner_user_id -> Uuid,
        scope -> Text,
        kind -> Text,
        content -> Text,
        source -> Text,
        confidence -> Float4,
        pinned -> Bool,
        superseded_by -> Nullable<Uuid>,
        embedding_ref -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}
