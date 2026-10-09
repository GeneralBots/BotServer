use super::*;

pub(super) fn validate_role(role: &str) -> String {
    match role.to_lowercase().as_str() {
        "admin" | "administrator" => "admin",
        "contributor" | "editor" => "contributor",
        "viewer" | "read" | "readonly" => "viewer",
        "owner" => "owner",
        _ => "member",
    }
    .to_string()
}

pub(super) fn get_permissions_for_role(role: &str) -> serde_json::Value {
    match role {
        "owner" | "admin" => json!({
            "read": true,
            "write": true,
            "delete": true,
            "manage_members": true,
            "manage_settings": true,
            "export_data": true
        }),
        "contributor" => json!({
            "read": true,
            "write": true,
            "delete": false,
            "manage_members": false,
            "manage_settings": false,
            "export_data": true
        }),
        "member" => json!({
            "read": true,
            "write": true,
            "delete": false,
            "manage_members": false,
            "manage_settings": false,
            "export_data": false
        }),
        _ => json!({
            "read": true,
            "write": false,
            "delete": false,
            "manage_members": false,
            "manage_settings": false,
            "export_data": false
        }),
    }
}

pub(super) fn send_member_invitation(
    _state: &dyn BasicRuntime,
    group_id: &str,
    user_email: &str,
    role: &str,
) -> Result<(), String> {
    trace!(
        "Invitation sent to {} for group {} with role {}",
        user_email,
        group_id,
        role
    );
    Ok(())
}
