use super::*;

pub async fn list_users_rows(State(state): State<Arc<DirectoryApiState>>) -> Html<String> {
    let query = SearchQuery {
        q: None,
        role: None,
        status: None,
        page: None,
    };
    Html(load_users_rows(&state, &query))
}
pub async fn search_users_rows(
    State(state): State<Arc<DirectoryApiState>>,
    Query(query): Query<SearchQuery>,
) -> Html<String> {
    Html(load_users_rows(&state, &query))
}
pub(super) fn load_users_rows(state: &Arc<DirectoryApiState>, query: &SearchQuery) -> String {
    let mut conn = match get_conn(state) {
        Some(c) => c,
        None => return empty_rows(),
    };

    #[derive(Debug, diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct UserRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: uuid::Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)]
        username: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        email: String,
        #[diesel(sql_type = diesel::sql_types::Bool)]
        is_active: bool,
        #[diesel(sql_type = diesel::sql_types::Bool)]
        is_admin: bool,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)]
        created_at: DateTime<Utc>,
    }

    let mut sql = String::from(
        "SELECT id, username, email, is_active, is_admin, created_at FROM users WHERE 1=1",
    );
    let mut binds: Vec<String> = Vec::new();

    if let Some(q) = query.q.as_ref() {
        if !q.trim().is_empty() {
            let pattern = format!("%{}%", q.trim());
            sql.push_str(" AND (username ILIKE $1 OR email ILIKE $1)");
            binds.push(pattern);
        }
    }
    if let Some(role) = query.role.as_ref() {
        if role == "admin" {
            sql.push_str(" AND is_admin = true");
        } else if role == "user" {
            sql.push_str(" AND is_admin = false");
        }
    }
    if let Some(status) = query.status.as_ref() {
        if status == "active" {
            sql.push_str(" AND is_active = true");
        } else if status == "inactive" || status == "suspended" {
            sql.push_str(" AND is_active = false");
        }
    }
    sql.push_str(" ORDER BY created_at DESC LIMIT 50");

    let rows = if binds.is_empty() {
        diesel::sql_query(&sql).load::<UserRow>(&mut conn)
    } else {
        diesel::sql_query(&sql)
            .bind::<diesel::sql_types::Text, _>(binds[0].clone())
            .load::<UserRow>(&mut conn)
    };

    let rows: Vec<UserRow> = rows.unwrap_or_default();

    if rows.is_empty() {
        return empty_rows();
    }

    let mut html = String::new();
    for user in rows {
        let initials: String = user
            .username
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .filter_map(|w| w.chars().next())
            .take(2)
            .collect::<String>()
            .to_uppercase();

        let role_class = if user.is_admin { "admin" } else { "user" };
        let role_label = if user.is_admin { "Admin" } else { "User" };
        let status_class = if user.is_active { "active" } else { "inactive" };
        let status_label = if user.is_active { "Active" } else { "Inactive" };

        html.push_str(&format!(
            r##"<tr>
    <td><input type="checkbox" value="{id}"></td>
    <td>
        <div class="user-cell">
            <div class="user-avatar">{initials}</div>
            <div class="user-info">
                <span class="user-name">{username}</span>
                <span class="user-username">@{username}</span>
            </div>
        </div>
    </td>
    <td>{email}</td>
    <td><span class="role-badge {role_class}">{role_label}</span></td>
    <td><span class="status-badge {status_class}"><span class="status-dot"></span>{status_label}</span></td>
    <td>{created}</td>
    <td>
        <div class="action-buttons">
            <button class="action-btn" onclick="openDetailPanel('{id}')">View</button>
            <button class="action-btn" onclick="editUser('{id}')">Edit</button>
            <button class="action-btn danger" onclick="confirmDeleteUser('{id}', '{username}')">Delete</button>
        </div>
    </td>
</tr>"##,
            id = user.id,
            username = html_escape(&user.username),
            email = html_escape(&user.email),
            created = user.created_at.format("%b %d, %Y"),
        ));
    }

    html
}
pub(super) fn empty_rows() -> String {
    r##"<tr>
    <td colspan="7" class="loading-cell">No users found</td>
</tr>"##
    .to_string()
}
pub async fn users_pagination(State(state): State<Arc<DirectoryApiState>>) -> Html<String> {
    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => return Html(String::new()),
    };
    let total = diesel::sql_query("SELECT COUNT(*) as count FROM users")
        .get_result::<CountResult>(&mut conn)
        .map(|r| r.count)
        .unwrap_or(0);

    Html(pagination_html(total, 50))
}
pub async fn list_groups_rows(State(state): State<Arc<DirectoryApiState>>) -> Html<String> {
    Html(load_groups_rows(&state, None))
}
pub async fn search_groups_rows(
    State(state): State<Arc<DirectoryApiState>>,
    Query(query): Query<SearchQuery>,
) -> Html<String> {
    Html(load_groups_rows(&state, query.q.as_deref()))
}
pub(super) fn load_groups_rows(state: &Arc<DirectoryApiState>, search: Option<&str>) -> String {
    let mut conn = match get_conn(state) {
        Some(c) => c,
        None => return empty_rows(),
    };

    #[derive(Debug, diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct GroupRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: uuid::Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        display_name: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        description: Option<String>,
        #[diesel(sql_type = diesel::sql_types::Bool)]
        is_active: bool,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)]
        created_at: DateTime<Utc>,
    }

    let mut sql = String::from(
        "SELECT id, name, display_name, description, is_active, created_at FROM rbac_groups WHERE 1=1",
    );
    let mut binds: Vec<String> = Vec::new();
    if let Some(q) = search {
        if !q.trim().is_empty() {
            let pattern = format!("%{}%", q.trim());
            sql.push_str(" AND (name ILIKE $1 OR display_name ILIKE $1)");
            binds.push(pattern);
        }
    }
    sql.push_str(" ORDER BY name LIMIT 50");

    let rows = if binds.is_empty() {
        diesel::sql_query(&sql).load::<GroupRow>(&mut conn)
    } else {
        diesel::sql_query(&sql)
            .bind::<diesel::sql_types::Text, _>(binds[0].clone())
            .load::<GroupRow>(&mut conn)
    };

    let rows: Vec<GroupRow> = rows.unwrap_or_default();

    if rows.is_empty() {
        return empty_rows();
    }

    let mut html = String::new();
    for group in rows {
        let status_class = if group.is_active { "active" } else { "inactive" };
        let status_label = if group.is_active { "Active" } else { "Inactive" };
        let desc = group.description.unwrap_or_else(|| "—".to_string());
        html.push_str(&format!(
            r##"<tr>
    <td>
        <div class="group-cell">
            <div class="group-icon">{initial}</div>
            <div class="group-info">
                <span class="group-name">{display_name}</span>
                <span class="group-slug">@{name}</span>
            </div>
        </div>
    </td>
    <td>{desc}</td>
    <td><span class="status-badge {status_class}"><span class="status-dot"></span>{status_label}</span></td>
    <td>{created}</td>
    <td>
        <div class="action-buttons">
            <button class="action-btn" onclick="openGroupDetail('{id}')">View</button>
        </div>
    </td>
</tr>"##,
            initial = group.display_name.chars().next().unwrap_or('G'),
            display_name = html_escape(&group.display_name),
            name = html_escape(&group.name),
            desc = html_escape(&desc),
            created = group.created_at.format("%b %d, %Y"),
            id = group.id,
        ));
    }

    html
}
pub async fn groups_pagination(State(state): State<Arc<DirectoryApiState>>) -> Html<String> {
    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => return Html(String::new()),
    };
    let total = diesel::sql_query("SELECT COUNT(*) as count FROM rbac_groups")
        .get_result::<CountResult>(&mut conn)
        .map(|r| r.count)
        .unwrap_or(0);

    Html(pagination_html(total, 50))
}
pub(super) fn pagination_html(total: i64, per_page: i64) -> String {
    if total <= per_page {
        return String::new();
    }
    let pages = (total + per_page - 1) / per_page;
    let mut html = String::from(
        r##"<button class="page-btn" disabled>‹</button>
<button class="page-btn active">1</button>"##,
    );
    for page in 2..=pages.min(10) {
        html.push_str(&format!(
            r##"<button class="page-btn">{page}</button>"##
        ));
    }
    html.push_str(r##"<button class="page-btn">›</button>"##);
    html
}
