use crate::core::{branches};

diesel::table! {
    canvases (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        description -> Nullable<Text>,
        width -> Int4,
        height -> Int4,
        background_color -> Nullable<Varchar>,
        thumbnail_url -> Nullable<Text>,
        is_public -> Bool,
        is_template -> Bool,
        created_by -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        org_id -> Uuid,
        bot_id -> Uuid,
    }
}

diesel::table! {
    canvas_elements (id) {
        id -> Uuid,
        canvas_id -> Uuid,
        element_type -> Varchar,
        x -> Float8,
        y -> Float8,
        width -> Float8,
        height -> Float8,
        rotation -> Float8,
        z_index -> Int4,
        locked -> Bool,
        properties -> Jsonb,
        created_by -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    canvas_collaborators (id) {
        id -> Uuid,
        canvas_id -> Uuid,
        user_id -> Uuid,
        permission -> Varchar,
        added_by -> Nullable<Uuid>,
        added_at -> Timestamptz,
    }
}

diesel::table! {
    canvas_versions (id) {
        id -> Uuid,
        canvas_id -> Uuid,
        version_number -> Int4,
        name -> Nullable<Varchar>,
        elements_snapshot -> Jsonb,
        created_by -> Uuid,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    canvas_comments (id) {
        id -> Uuid,
        canvas_id -> Uuid,
        element_id -> Nullable<Uuid>,
        parent_comment_id -> Nullable<Uuid>,
        author_id -> Uuid,
        content -> Text,
        x_position -> Nullable<Float8>,
        y_position -> Nullable<Float8>,
        resolved -> Bool,
        resolved_by -> Nullable<Uuid>,
        resolved_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(canvases -> branches (branch_id));
diesel::joinable!(canvas_elements -> canvases (canvas_id));
diesel::joinable!(canvas_collaborators -> canvases (canvas_id));
diesel::joinable!(canvas_versions -> canvases (canvas_id));
diesel::joinable!(canvas_comments -> canvases (canvas_id));
