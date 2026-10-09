#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub(super) fn test_validate_role() {
        assert_eq!(validate_role("admin"), "admin");
        assert_eq!(validate_role("ADMIN"), "admin");
        assert_eq!(validate_role("contributor"), "contributor");
        assert_eq!(validate_role("viewer"), "viewer");
        assert_eq!(validate_role("unknown"), "member");
    }

    #[test]
    pub(super) fn test_get_permissions_for_role() {
        let admin_perms = get_permissions_for_role("admin");
        assert!(admin_perms.get("read").unwrap().as_bool().unwrap());
        assert!(admin_perms.get("write").unwrap().as_bool().unwrap());
        assert!(admin_perms
            .get("manage_members")
            .unwrap()
            .as_bool()
            .unwrap());

        let viewer_perms = get_permissions_for_role("viewer");
        assert!(viewer_perms.get("read").unwrap().as_bool().unwrap());
        assert!(!viewer_perms.get("write").unwrap().as_bool().unwrap());
        assert!(!viewer_perms.get("delete").unwrap().as_bool().unwrap());
    }
}
