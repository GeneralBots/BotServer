use super::*;

pub async fn user_assignment_panel(State(state): State<Arc<AppState>>, Path(user_id): Path<Uuid>) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::users;
        users::table
            .find(user_id)
            .first::<User>(&mut db_conn)
            .map_err(|e| format!("User not found: {e}"))
    })
    .await;

    match result {
        Ok(Ok(user)) => {
            let html = format!(
                r##"<div class="assignment-container">
                <h4>Manage Access for: {username}</h4>
                <p class="text-muted">{email}</p>

                <div class="assignment-tabs">
                    <button class="tab-btn active" onclick="showTab('roles-tab', this)">Roles</button>
                    <button class="tab-btn" onclick="showTab('groups-tab', this)">Groups</button>
                </div>

                <div id="roles-tab" class="tab-content active">
                    <div class="assignment-panel">
                        <div class="list-box">
                            <div class="list-box-header">Available Roles</div>
                            <div class="list-box-content" id="available-roles"
                                 hx-get="/settings/rbac/users/{uid}/available-roles"
                                 hx-trigger="load, roleAssigned from:body, roleRemoved from:body">
                                <div class="loading">Loading...</div>
                            </div>
                        </div>

                        <div class="transfer-buttons">
                            <button class="transfer-btn" id="assign-role-btn"
                                    hx-post="/api/rbac/users/{uid}/roles/__ROLE_ID__"
                                    hx-trigger="click"
                                    hx-swap="none"
                                    hx-on::after-request="htmx.trigger(document.body, 'roleAssigned')"
                                    disabled>→</button>
                            <button class="transfer-btn" id="remove-role-btn"
                                    hx-delete="/api/rbac/users/{uid}/roles/__ROLE_ID__"
                                    hx-trigger="click"
                                    hx-swap="none"
                                    hx-on::after-request="htmx.trigger(document.body, 'roleRemoved')"
                                    disabled>←</button>
                        </div>

                        <div class="list-box">
                            <div class="list-box-header">Assigned Roles</div>
                            <div class="list-box-content" id="assigned-roles"
                                 hx-get="/settings/rbac/users/{uid}/assigned-roles"
                                 hx-trigger="load, roleAssigned from:body, roleRemoved from:body">
                                <div class="loading">Loading...</div>
                            </div>
                        </div>
                    </div>
                </div>

                <div id="groups-tab" class="tab-content" style="display:none;">
                    <div class="assignment-panel">
                        <div class="list-box">
                            <div class="list-box-header">Available Groups</div>
                            <div class="list-box-content" id="available-groups"
                                 hx-get="/settings/rbac/users/{uid}/available-groups"
                                 hx-trigger="load, groupAdded from:body, groupRemoved from:body">
                                <div class="loading">Loading...</div>
                            </div>
                        </div>

                        <div class="transfer-buttons">
                            <button class="transfer-btn" id="add-group-btn"
                                    hx-post="/api/rbac/users/{uid}/groups/__GROUP_ID__"
                                    hx-trigger="click"
                                    hx-swap="none"
                                    hx-on::after-request="htmx.trigger(document.body, 'groupAdded')"
                                    disabled>→</button>
                            <button class="transfer-btn" id="remove-group-btn"
                                    hx-delete="/api/rbac/users/{uid}/groups/__GROUP_ID__"
                                    hx-trigger="click"
                                    hx-swap="none"
                                    hx-on::after-request="htmx.trigger(document.body, 'groupRemoved')"
                                    disabled>←</button>
                        </div>

                        <div class="list-box">
                            <div class="list-box-header">Member Of</div>
                            <div class="list-box-content" id="assigned-groups"
                                 hx-get="/settings/rbac/users/{uid}/assigned-groups"
                                 hx-trigger="load, groupAdded from:body, groupRemoved from:body">
                                <div class="loading">Loading...</div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            <script>
            function showTab(tabId, btn) {{
                document.querySelectorAll('.tab-content').forEach(t => t.style.display = 'none');
                document.querySelectorAll('.assignment-tabs .tab-btn').forEach(b => b.classList.remove('active'));
                document.getElementById(tabId).style.display = 'block';
                btn.classList.add('active');
            }}

            function selectAvailableRole(el, roleId) {{
                document.querySelectorAll('#available-roles .list-item').forEach(i => i.classList.remove('selected'));
                el.classList.add('selected');
                const btn = document.getElementById('assign-role-btn');
                btn.disabled = false;
                btn.setAttribute('hx-post', '/api/rbac/users/{uid}/roles/' + roleId);
                htmx.process(btn);
            }}

            function selectAssignedRole(el, roleId) {{
                document.querySelectorAll('#assigned-roles .list-item').forEach(i => i.classList.remove('selected'));
                el.classList.add('selected');
                const btn = document.getElementById('remove-role-btn');
                btn.disabled = false;
                btn.setAttribute('hx-delete', '/api/rbac/users/{uid}/roles/' + roleId);
                htmx.process(btn);
            }}

            function selectAvailableGroup(el, groupId) {{
                document.querySelectorAll('#available-groups .list-item').forEach(i => i.classList.remove('selected'));
                el.classList.add('selected');
                const btn = document.getElementById('add-group-btn');
                btn.disabled = false;
                btn.setAttribute('hx-post', '/api/rbac/users/{uid}/groups/' + groupId);
                htmx.process(btn);
            }}

            function selectAssignedGroup(el, groupId) {{
                document.querySelectorAll('#assigned-groups .list-item').forEach(i => i.classList.remove('selected'));
                el.classList.add('selected');
                const btn = document.getElementById('remove-group-btn');
                btn.disabled = false;
                btn.setAttribute('hx-delete', '/api/rbac/users/{uid}/groups/' + groupId);
                htmx.process(btn);
            }}
            </script>"##,
                username = html_escape(&user.username),
                email = html_escape(&user.email),
                uid = user_id
            );
            Html(html).into_response()
        }
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "user_assignment_panel query failed");
            Html(render_error("user_assignment_panel")).into_response()
        }
        Err(e) => {
            tracing::warn!(error = %e, "user_assignment_panel task failed");
            Html(render_error("user_assignment_panel")).into_response()
        }
    }
}
pub async fn available_roles_for_user(State(state): State<Arc<AppState>>, Path(user_id): Path<Uuid>) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::{rbac_roles, rbac_user_roles};

        let assigned_role_ids: Vec<Uuid> = rbac_user_roles::table
            .filter(rbac_user_roles::user_id.eq(user_id))
            .select(rbac_user_roles::role_id)
            .load(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))?;

        rbac_roles::table
            .filter(rbac_roles::is_active.eq(true))
            .filter(rbac_roles::id.ne_all(assigned_role_ids))
            .order(rbac_roles::display_name.asc())
            .load::<RbacRole>(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))
    })
    .await;

    match result {
        Ok(Ok(roles)) => {
            let mut html = String::new();
            for role in &roles {
                html.push_str(&format!(
                    r##"<div class="list-item" onclick="selectAvailableRole(this, '{id}')">{name}</div>"##,
                    id = role.id,
                    name = role.display_name
                ));
            }
            if roles.is_empty() {
                html.push_str(r##"<div class="empty-state">All roles assigned</div>"##);
            }
            Html(html).into_response()
        }
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "available_roles_for_user query failed");
            Html(render_error("available_roles_for_user")).into_response()
        }
        Err(e) => {
            tracing::warn!(error = %e, "available_roles_for_user task failed");
            Html(render_error("available_roles_for_user")).into_response()
        }
    }
}
pub async fn assigned_roles_for_user(State(state): State<Arc<AppState>>, Path(user_id): Path<Uuid>) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::{rbac_roles, rbac_user_roles};

        rbac_user_roles::table
            .inner_join(rbac_roles::table)
            .filter(rbac_user_roles::user_id.eq(user_id))
            .filter(rbac_roles::is_active.eq(true))
            .select(RbacRole::as_select())
            .load::<RbacRole>(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))
    })
    .await;

    match result {
        Ok(Ok(roles)) => {
            let mut html = String::new();
            for role in &roles {
                html.push_str(&format!(
                    r##"<div class="list-item" onclick="selectAssignedRole(this, '{id}')">{name}</div>"##,
                    id = role.id,
                    name = role.display_name
                ));
            }
            if roles.is_empty() {
                html.push_str(r##"<div class="empty-state">No roles assigned</div>"##);
            }
            Html(html).into_response()
        }
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "assigned_roles_for_user query failed");
            Html(render_error("assigned_roles_for_user")).into_response()
        }
        Err(e) => {
            tracing::warn!(error = %e, "assigned_roles_for_user task failed");
            Html(render_error("assigned_roles_for_user")).into_response()
        }
    }
}
pub async fn available_groups_for_user(State(state): State<Arc<AppState>>, Path(user_id): Path<Uuid>) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::{rbac_groups, rbac_user_groups};

        let assigned_group_ids: Vec<Uuid> = rbac_user_groups::table
            .filter(rbac_user_groups::user_id.eq(user_id))
            .select(rbac_user_groups::group_id)
            .load(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))?;

        rbac_groups::table
            .filter(rbac_groups::is_active.eq(true))
            .filter(rbac_groups::id.ne_all(assigned_group_ids))
            .order(rbac_groups::display_name.asc())
            .load::<RbacGroup>(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))
    })
    .await;

    match result {
        Ok(Ok(groups)) => {
            let mut html = String::new();
            for group in &groups {
                html.push_str(&format!(
                    r##"<div class="list-item" onclick="selectAvailableGroup(this, '{id}')">{name}</div>"##,
                    id = group.id,
                    name = group.display_name
                ));
            }
            if groups.is_empty() {
                html.push_str(r##"<div class="empty-state">All groups joined</div>"##);
            }
            Html(html).into_response()
        }
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "available_groups_for_user query failed");
            Html(render_error("available_groups_for_user")).into_response()
        }
        Err(e) => {
            tracing::warn!(error = %e, "available_groups_for_user task failed");
            Html(render_error("available_groups_for_user")).into_response()
        }
    }
}
