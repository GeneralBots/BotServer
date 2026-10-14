use super::*;

pub async fn create_group(
    State(state): State<Arc<DirectoryApiState>>,
    axum::extract::Form(form): axum::extract::Form<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let name = form.get("name").cloned().unwrap_or_default();
    let display_name = form.get("display_name").cloned().unwrap_or_else(|| name.clone());
    if name.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Html("Group name is required".to_string()));
    }

    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("Database unavailable".to_string()),
            );
        }
    };

    let group_id = uuid::Uuid::new_v4();
    let result = diesel::sql_query(
        "INSERT INTO rbac_groups (id, name, display_name, description, is_active, created_by, created_at, updated_at)
         VALUES ($1, $2, $3, $4, true, NULL, NOW(), NOW())",
    )
    .bind::<diesel::sql_types::Uuid, _>(group_id)
    .bind::<diesel::sql_types::Text, _>(&name)
    .bind::<diesel::sql_types::Text, _>(&display_name)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(
        form.get("description").cloned(),
    )
    .execute(&mut conn);

    match result {
        Ok(_) => (StatusCode::OK, Html(load_groups_rows(&state, None))),
        Err(e) => {
            log::error!("Failed to create group: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Html("Failed to create group".to_string()))
        }
    }
}
pub async fn user_profile(
    State(state): State<Arc<DirectoryApiState>>,
    Path(id): Path<uuid::Uuid>,
) -> Html<String> {
    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => return Html(String::new()),
    };

    #[derive(Debug, diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct UserRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        username: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        email: String,
        #[diesel(sql_type = diesel::sql_types::Bool)]
        is_admin: bool,
        #[diesel(sql_type = diesel::sql_types::Bool)]
        is_active: bool,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)]
        created_at: DateTime<Utc>,
    }

    let user: Option<UserRow> = diesel::sql_query(
        "SELECT username, email, is_admin, is_active, created_at FROM users WHERE id = $1",
    )
    .bind::<diesel::sql_types::Uuid, _>(id)
    .get_result(&mut conn)
    .ok();

    match user {
        Some(u) => Html(format!(
            r##"<div class="detail-section">
    <div class="detail-label">Username</div>
    <div class="detail-value">{username}</div>
</div>
<div class="detail-section">
    <div class="detail-label">Email</div>
    <div class="detail-value">{email}</div>
</div>
<div class="detail-section">
    <div class="detail-label">Role</div>
    <div class="detail-value">{role}</div>
</div>
<div class="detail-section">
    <div class="detail-label">Status</div>
    <div class="detail-value">{status}</div>
</div>
<div class="detail-section">
    <div class="detail-label">Member Since</div>
    <div class="detail-value">{created}</div>
</div>"##,
            username = html_escape(&u.username),
            email = html_escape(&u.email),
            role = if u.is_admin { "Admin" } else { "User" },
            status = if u.is_active { "Active" } else { "Inactive" },
            created = u.created_at.format("%b %d, %Y"),
        )),
        None => Html(r#"<p class="text-muted">User not found</p>"#.to_string()),
    }
}
pub async fn user_permissions(Path(_id): Path<uuid::Uuid>) -> Html<String> {
    Html(
        r##"<div class="permission-list">
    <div class="permission-item"><span>Admin access</span><span class="permission-value">Not granted</span></div>
    <div class="permission-item"><span>Bot management</span><span class="permission-value">Not granted</span></div>
    <div class="permission-item"><span>Group management</span><span class="permission-value">Not granted</span></div>
</div>"##
        .to_string(),
    )
}
pub async fn user_security(Path(_id): Path<uuid::Uuid>) -> Html<String> {
    Html(
        r##"<div class="security-list">
    <div class="security-item"><span>Two-factor authentication</span><span class="security-value">Disabled</span></div>
    <div class="security-item"><span>Password last changed</span><span class="security-value">—</span></div>
    <div class="security-item"><span>Active sessions</span><span class="security-value">—</span></div>
</div>"##
        .to_string(),
    )
}
pub async fn user_activity(
    State(state): State<Arc<DirectoryApiState>>,
    Path(id): Path<uuid::Uuid>,
) -> Html<String> {
    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => return Html(String::new()),
    };

    #[derive(Debug, diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct SessionRow {
        #[diesel(sql_type = diesel::sql_types::Timestamptz)]
        created_at: DateTime<Utc>,
    }

    let sessions: Vec<SessionRow> = diesel::sql_query(
        "SELECT created_at FROM user_sessions WHERE user_id = $1::text ORDER BY created_at DESC LIMIT 10",
    )
    .bind::<diesel::sql_types::Uuid, _>(id)
    .load::<SessionRow>(&mut conn)
    .unwrap_or_default();

    if sessions.is_empty() {
        return Html(r#"<p class="text-muted">No recent activity</p>"#.to_string());
    }

    let mut html = String::new();
    for s in sessions {
        html.push_str(&format!(
            r##"<div class="activity-entry"><span class="activity-dot"></span><span>Session started</span><span class="activity-time">{}</span></div>"##,
            s.created_at.format("%b %d, %H:%M")
        ));
    }
    Html(html)
}
pub async fn user_edit_form(
    State(state): State<Arc<DirectoryApiState>>,
    Path(id): Path<uuid::Uuid>,
) -> Html<String> {
    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => return Html(String::new()),
    };

    #[derive(Debug, diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct UserRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        username: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        email: String,
        #[diesel(sql_type = diesel::sql_types::Bool)]
        is_admin: bool,
        #[diesel(sql_type = diesel::sql_types::Bool)]
        is_active: bool,
    }

    let user: Option<UserRow> = diesel::sql_query(
        "SELECT username, email, is_admin, is_active FROM users WHERE id = $1",
    )
    .bind::<diesel::sql_types::Uuid, _>(id)
    .get_result(&mut conn)
    .ok();

    match user {
        Some(u) => Html(format!(
            r##"<form hx-post="/api/directory/users/{id}" hx-target="#users-table-body" hx-swap="innerHTML">
    <div class="modal-header">
        <h2>Edit User</h2>
        <button type="button" class="close-btn" onclick="document.getElementById('edit-user-modal').close()">×</button>
    </div>
    <div class="form-group">
        <label>Username</label>
        <input type="text" name="username" value="{username}">
    </div>
    <div class="form-group">
        <label>Email</label>
        <input type="email" name="email" value="{email}">
    </div>
    <div class="form-row">
        <div class="form-group">
            <label>Role</label>
            <select name="role">
                <option value="user" {user_sel}>User</option>
                <option value="admin" {admin_sel}>Admin</option>
            </select>
        </div>
        <div class="form-group">
            <label>Status</label>
            <select name="status">
                <option value="active" {active_sel}>Active</option>
                <option value="inactive" {inactive_sel}>Inactive</option>
            </select>
        </div>
    </div>
    <div class="modal-footer">
        <button type="button" class="btn-secondary" onclick="document.getElementById('edit-user-modal').close()">Cancel</button>
        <button type="submit" class="btn-primary">Save Changes</button>
    </div>
</form>"##,
            id = id,
            username = html_escape(&u.username),
            email = html_escape(&u.email),
            user_sel = if u.is_admin { "" } else { "selected" },
            admin_sel = if u.is_admin { "selected" } else { "" },
            active_sel = if u.is_active { "selected" } else { "" },
            inactive_sel = if u.is_active { "" } else { "selected" },
        )),
        None => Html(r#"<p>User not found</p>"#.to_string()),
    }
}
pub async fn delete_user(
    State(state): State<Arc<DirectoryApiState>>,
    Path(id): Path<uuid::Uuid>,
) -> impl IntoResponse {
    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("Database unavailable".to_string()),
            );
        }
    };

    let result = diesel::sql_query("DELETE FROM users WHERE id = $1")
        .bind::<diesel::sql_types::Uuid, _>(id)
        .execute(&mut conn);

    match result {
        Ok(_) => (
            StatusCode::OK,
            Html(load_users_rows(&state, &SearchQuery { q: None, role: None, status: None, page: None })),
        ),
        Err(e) => {
            log::error!("Failed to delete user {}: {}", id, e);
            (StatusCode::INTERNAL_SERVER_ERROR, Html("Failed to delete user".to_string()))
        }
    }
}
