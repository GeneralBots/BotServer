//! External connector tables (canonical, #1497).

diesel::table! {
    connector_connections (id) {
        id -> Uuid,
        org_id -> Uuid,
        kind -> Text,
        display_name -> Nullable<Text>,
        vault_token_ref -> Text,
        status -> Text,
        cursors -> Jsonb,
        last_sync_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    indexed_items (id) {
        id -> Uuid,
        connection_id -> Uuid,
        external_id -> Text,
        title -> Text,
        body_tsv -> Nullable<Text>,
        vector_ref -> Nullable<Text>,
        acl -> Jsonb,
        container -> Nullable<Text>,
        external_url -> Nullable<Text>,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::joinable!(indexed_items -> connector_connections (connection_id));

diesel::allow_tables_to_appear_in_same_query!(connector_connections, indexed_items);
