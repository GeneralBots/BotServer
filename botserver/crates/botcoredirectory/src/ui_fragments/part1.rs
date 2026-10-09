use super::*;

pub fn configure_directory_ui_routes() -> Router<Arc<DirectoryApiState>> {
    Router::new()
        .route("/api/directory/users/list", get(list_users_rows))
        .route("/api/directory/users/search", get(search_users_rows))
        .route("/api/directory/users/list/pagination", get(users_pagination))
        .route("/api/directory/users/:id/profile", get(user_profile))
        .route("/api/directory/users/:id/permissions", get(user_permissions))
        .route("/api/directory/users/:id/security", get(user_security))
        .route("/api/directory/users/:id/activity", get(user_activity))
        .route("/api/directory/users/:id/edit", get(user_edit_form))
        .route("/api/directory/users/:id/delete", axum::routing::delete(delete_user))
        .route("/api/directory/groups/list", get(list_groups_rows))
        .route("/api/directory/groups/search", get(search_groups_rows))
        .route("/api/directory/groups/list/pagination", get(groups_pagination))
        .route("/api/directory/groups/create", axum::routing::post(create_group))
        .route("/api/directory/groups/:id", get(group_detail))
}
