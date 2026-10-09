use super::*;

pub(super) fn user_roles_from_headers(headers: &HeaderMap) -> UserRoles {
    let roles = headers
        .get("X-User-Roles")
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            s.split(';')
                .map(|r| r.trim().to_string())
                .filter(|r| !r.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let user_id = headers
        .get("X-User-Id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok());

    if let Some(uid) = user_id {
        UserRoles::with_user_id(roles, uid)
    } else {
        UserRoles::new(roles)
    }
}
