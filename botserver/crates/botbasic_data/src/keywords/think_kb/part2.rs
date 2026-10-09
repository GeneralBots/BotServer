use super::*;

pub(super) fn get_user_group_ids(
    conn: &mut diesel::PgConnection,
    user_id: uuid::Uuid,
) -> Result<Vec<uuid::Uuid>, String> {
    diesel::sql_query(
        "SELECT group_id FROM rbac_user_groups WHERE user_id = $1",
    )
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .load::<GroupIdRow>(conn)
    .map(|rows| rows.into_iter().map(|r| r.group_id).collect())
    .map_err(|e| format!("Failed to fetch user groups: {e}"))
}
pub(super) fn get_accessible_kb_ids(
    conn: &mut diesel::PgConnection,
    user_id: uuid::Uuid,
) -> Result<Vec<uuid::Uuid>, String> {
    let user_groups = get_user_group_ids(conn, user_id)?;
    if user_groups.is_empty() {
        diesel::sql_query(
            "SELECT id FROM kb_collections kc
            WHERE NOT EXISTS (
                SELECT 1 FROM kb_group_associations kga WHERE kga.kb_id = kc.id
            )",
        )
        .load::<KbIdRow>(conn)
        .map(|rows| rows.into_iter().map(|r| r.id).collect())
        .map_err(|e| format!("Failed to query accessible KBs: {e}"))
    } else {
        diesel::sql_query(
            "SELECT id FROM kb_collections kc
            WHERE NOT EXISTS (
                SELECT 1 FROM kb_group_associations kga WHERE kga.kb_id = kc.id
            )
            OR EXISTS (
                SELECT 1 FROM kb_group_associations kga
                WHERE kga.kb_id = kc.id
                AND kga.group_id = ANY($1::uuid[])
            )",
        )
        .bind::<diesel::sql_types::Array<diesel::sql_types::Uuid>, _>(user_groups)
        .load::<KbIdRow>(conn)
        .map(|rows| rows.into_iter().map(|r| r.id).collect())
        .map_err(|e| format!("Failed to query accessible KBs: {e}"))
    }
}
