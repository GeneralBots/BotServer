#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    pub(super) fn test_suggestion_json_context() {
        let suggestion = json!({
            "type": "context",
            "context": "products",
            "text": "View Products",
            "action": {
                "type": "select_context",
                "context": "products"
            }
        });

        assert_eq!(suggestion["type"], "context");
        assert_eq!(suggestion["action"]["type"], "select_context");
    }

    #[test]
    pub(super) fn test_suggestion_json_tool_no_params() {
        let suggestion = json!({
            "type": "tool",
            "tool": "search_kb",
            "text": "Search Knowledge Base",
            "action": {
                "type": "invoke_tool",
                "tool": "search_kb",
                "params": Option::<Vec<String>>::None,
                "prompt_for_params": true
            }
        });

        assert_eq!(suggestion["type"], "tool");
        assert_eq!(suggestion["action"]["prompt_for_params"], true);
    }

    #[test]
    pub(super) fn test_suggestion_json_tool_with_params() {
        let params = vec!["query".to_string(), "products".to_string()];
        let suggestion = json!({
            "type": "tool",
            "tool": "search_kb",
            "text": "Search Products",
            "action": {
                "type": "invoke_tool",
                "tool": "search_kb",
                "params": params,
                "prompt_for_params": false
            }
        });

        assert_eq!(suggestion["type"], "tool");
        assert_eq!(suggestion["action"]["prompt_for_params"], false);
        assert!(suggestion["action"]["params"].is_array());
    }
}
