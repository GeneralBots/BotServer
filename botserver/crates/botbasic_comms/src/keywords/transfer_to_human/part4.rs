

pub fn get_tool_definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "transfer_to_human",
            "description": "Transfer the conversation to a human attendant. Use this when the customer explicitly asks to speak with a person, when the issue is too complex for automated handling, or when emotional support is needed.",
            "parameters": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "If someone wants to talk to somebody specific, provide their name or alias. Leave empty for any available attendant."
                    },
                    "department": {
                        "type": "string",
                        "description": "Department to transfer to: sales, support, technical, billing, etc."
                    },
                    "priority": {
                        "type": "string",
                        "description": "Priority level for the transfer: normal, high, or urgent",
                        "default": "normal"
                    },
                    "reason": {
                        "type": "string",
                        "description": "Brief reason for the transfer to help the attendant understand the context"
                    }
                },
                "required": []
            }
        }
    })
}
