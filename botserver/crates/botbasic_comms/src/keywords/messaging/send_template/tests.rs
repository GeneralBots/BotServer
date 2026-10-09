#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub(super) fn test_send_template_valid_email() {
        let result = send_template_message("welcome", "user@example.com", "email", None);
        assert!(result.get("success").unwrap().as_bool().unwrap());
    }

    #[test]
    pub(super) fn test_send_template_invalid_email() {
        let result = send_template_message("welcome", "invalid-email", "email", None);
        assert!(!result.get("success").unwrap().as_bool().unwrap());
    }

    #[test]
    pub(super) fn test_send_template_invalid_channel() {
        let result = send_template_message("welcome", "user@example.com", "invalid", None);
        assert!(!result.get("success").unwrap().as_bool().unwrap());
    }

    #[test]
    pub(super) fn test_send_template_batch() {
        let recipients: Array = vec![
            Dynamic::from("user1@example.com"),
            Dynamic::from("user2@example.com"),
        ];

        let result = send_template_batch("welcome", &recipients, "email", None);
        assert_eq!(result.get("total").unwrap().as_int().unwrap(), 2);
        assert_eq!(result.get("sent").unwrap().as_int().unwrap(), 2);
    }

    #[test]
    pub(super) fn test_create_template() {
        let result = create_message_template("test", "email", Some("Subject"), "Hello {{name}}!");
        assert!(result.get("success").unwrap().as_bool().unwrap());
    }

    #[test]
    pub(super) fn test_create_template_empty_name() {
        let result = create_message_template("", "email", None, "Content");
        assert!(!result.get("success").unwrap().as_bool().unwrap());
    }

    #[test]
    pub(super) fn test_extract_template_variables() {
        let content = "Hello {{name}}, your order {{order_id}} is ready!";
        let vars = extract_template_variables(content);
        assert_eq!(vars.len(), 2);
    }

    #[test]
    pub(super) fn test_extract_template_variables_empty() {
        let content = "Hello, no variables here!";
        let vars = extract_template_variables(content);
        assert!(vars.is_empty());
    }

    #[test]
    pub(super) fn test_generate_message_id() {
        let id = generate_message_id();
        assert!(id.starts_with("msg_"));
    }
}
