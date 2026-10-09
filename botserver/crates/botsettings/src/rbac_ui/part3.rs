use super::*;

pub async fn assigned_groups_for_user(State(state): State<Arc<AppState>>, Path(user_id): Path<Uuid>) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::{rbac_groups, rbac_user_groups};

        rbac_user_groups::table
            .inner_join(rbac_groups::table)
            .filter(rbac_user_groups::user_id.eq(user_id))
            .filter(rbac_groups::is_active.eq(true))
            .select(RbacGroup::as_select())
            .load::<RbacGroup>(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))
    })
    .await;

    match result {
        Ok(Ok(groups)) => {
            let mut html = String::new();
            for group in &groups {
                html.push_str(&format!(
                    r##"<div class="list-item" onclick="selectAssignedGroup(this, '{id}')">{name}</div>"##,
                    id = group.id,
                    name = group.display_name
                ));
            }
            if groups.is_empty() {
                html.push_str(r##"<div class="empty-state">Not a member of any group</div>"##);
            }
            Html(html).into_response()
        }
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "assigned_groups_for_user query failed");
            Html(render_error("assigned_groups_for_user")).into_response()
        }
        Err(e) => {
            tracing::warn!(error = %e, "assigned_groups_for_user task failed");
            Html(render_error("assigned_groups_for_user")).into_response()
        }
    }
}
pub(super) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
