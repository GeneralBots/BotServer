
use std::collections::HashMap;

#[must_use]
pub fn get_script(name: &str) -> Option<&'static str> {
    match name {
        "greeting" => Some(GREETING_SCRIPT),
        "kb_search" => Some(KB_SEARCH_SCRIPT),
        "attendance" => Some(ATTENDANCE_SCRIPT),
        "error_handling" => Some(ERROR_HANDLING_SCRIPT),
        "llm_tools" => Some(LLM_TOOLS_SCRIPT),
        "data_operations" => Some(DATA_OPERATIONS_SCRIPT),
        "http_integration" => Some(HTTP_INTEGRATION_SCRIPT),
        "menu_flow" => Some(MENU_FLOW_SCRIPT),
        "simple_echo" => Some(SIMPLE_ECHO_SCRIPT),
        "variables" => Some(VARIABLES_SCRIPT),
        _ => None,
    }
}

#[must_use]
pub fn available_scripts() -> Vec<&'static str> {
    vec![
        "greeting",
        "kb_search",
        "attendance",
        "error_handling",
        "llm_tools",
        "data_operations",
        "http_integration",
        "menu_flow",
        "simple_echo",
        "variables",
    ]
}

#[must_use]
pub fn all_scripts() -> HashMap<&'static str, &'static str> {
    let mut scripts = HashMap::new();
    for name in available_scripts() {
        if let Some(content) = get_script(name) {
            scripts.insert(name, content);
        }
    }
    scripts
}

pub const GREETING_SCRIPT: &str = r#"
' Greeting Flow Script
' Simple greeting and response pattern

REM Initialize greeting
greeting$ = "Hello! Welcome to our service."
TALK greeting$

REM Wait for user response
HEAR userInput$

REM Check for specific keywords
IF INSTR(UCASE$(userInput$), "HELP") > 0 THEN
    TALK "I can help you with: Products, Support, or Billing. What would you like to know?"
ELSEIF INSTR(UCASE$(userInput$), "BYE") > 0 THEN
    TALK "Goodbye! Have a great day!"
    END
ELSE
    TALK "Thank you for your message. How can I assist you today?"
END IF
"#;

pub const KB_SEARCH_SCRIPT: &str = r#"
' Knowledge Base Search Script
' Demonstrates searching the knowledge base

REM Prompt user for query
TALK "What would you like to know about? I can search our knowledge base for you."

REM Get user input
HEAR query$

REM Search knowledge base
results = FIND "kb" WHERE "content LIKE '%" + query$ + "%'"

IF results.count > 0 THEN
    TALK "I found " + STR$(results.count) + " result(s):"
    FOR i = 0 TO results.count - 1
        TALK "- " + results(i).title
    NEXT i
    TALK "Would you like more details on any of these?"
ELSE
    TALK "I couldn't find anything about that. Let me connect you with a human agent."
    TRANSFER HUMAN
END IF
"#;

pub const ATTENDANCE_SCRIPT: &str = r#"
' Attendance / Human Handoff Script
' Demonstrates transferring to human agents

REM Check user request
TALK "I can help you with automated support, or connect you to a human agent."
TALK "Type 'agent' to speak with a person, or describe your issue."

HEAR response$

IF INSTR(UCASE$(response$), "AGENT") > 0 OR INSTR(UCASE$(response$), "HUMAN") > 0 THEN
    TALK "I'll connect you with an agent now. Please wait..."

    REM Get queue position
    position = GET_QUEUE_POSITION()

    IF position > 0 THEN
        TALK "You are number " + STR$(position) + " in the queue."
        TALK "Estimated wait time: " + STR$(position * 2) + " minutes."
    END IF

    REM Transfer to human
    TRANSFER HUMAN
ELSE
    REM Try to handle with bot
    TALK "Let me try to help you with that."
    ASK llm response$
    TALK llm.response
END IF
"#;

pub const ERROR_HANDLING_SCRIPT: &str = r#"
' Error Handling Script
' Demonstrates ON ERROR RESUME NEXT patterns

REM Enable error handling
ON ERROR RESUME NEXT

REM Try a potentially failing operation
result = FIND "users" WHERE "id = '12345'"

IF ERR <> 0 THEN
    TALK "Sorry, I encountered an error: " + ERR.MESSAGE$
    ERR.CLEAR
    REM Try alternative approach
    result = GET_CACHED_USER("12345")
END IF

REM Validate input
HEAR userInput$

IF LEN(userInput$) = 0 THEN
    TALK "I didn't receive any input. Please try again."
    GOTO retry_input
END IF

IF LEN(userInput$) > 1000 THEN
    TALK "Your message is too long. Please keep it under 1000 characters."
    GOTO retry_input
END IF

REM Process validated input
TALK "Processing your request: " + LEFT$(userInput$, 50) + "..."

retry_input:
"#;

pub const LLM_TOOLS_SCRIPT: &str = r#"
' LLM Tools Script
' Demonstrates LLM with function calling / tools

REM Define available tools
TOOL "get_weather" DESCRIPTION "Get current weather for a location" PARAMS "location:string"
TOOL "search_products" DESCRIPTION "Search product catalog" PARAMS "query:string,category:string?"
TOOL "create_ticket" DESCRIPTION "Create a support ticket" PARAMS "subject:string,description:string,priority:string?"

REM Set system prompt
SYSTEM_PROMPT = "You are a helpful assistant. Use the available tools to help users."

REM Main conversation loop
TALK "Hello! I can help you with weather, products, or create support tickets."

conversation_loop:
HEAR userMessage$

IF INSTR(UCASE$(userMessage$), "EXIT") > 0 THEN
    TALK "Goodbye!"
    END
END IF

REM Send to LLM with tools
ASK llm userMessage$ WITH TOOLS

REM Check if LLM wants to call a tool
IF llm.tool_call THEN
    REM Execute the tool
    tool_result = EXECUTE_TOOL(llm.tool_name, llm.tool_args)

    REM Send result back to LLM
    ASK llm tool_result AS TOOL_RESPONSE
END IF

REM Output final response
TALK llm.response

GOTO conversation_loop
"#;

pub const DATA_OPERATIONS_SCRIPT: &str = r#"
' Data Operations Script
' Demonstrates FIND, SAVE, UPDATE, DELETE operations

REM Create a new record
new_customer.name = "John Doe"
new_customer.email = "john@example.com"
new_customer.phone = "+15551234567"

SAVE "customers" new_customer
TALK "Customer created with ID: " + new_customer.id

REM Find records
customers = FIND "customers" WHERE "email LIKE '%example.com'"
TALK "Found " + STR$(customers.count) + " customers from example.com"

REM Update a record
customer = FIND_ONE "customers" WHERE "email = 'john@example.com'"
IF customer THEN
    customer.status = "active"
    customer.verified_at = NOW()
    UPDATE "customers" customer
    TALK "Customer updated successfully"
END IF

REM Delete a record (soft delete)
DELETE "customers" WHERE "status = 'inactive' AND created_at < DATE_SUB(NOW(), 30, 'day')"
TALK "Cleaned up inactive customers"

REM Transaction example
BEGIN TRANSACTION
    order.customer_id = customer.id
    order.total = 99.99
    order.status = "pending"
    SAVE "orders" order

    customer.last_order_at = NOW()
    UPDATE "customers" customer
COMMIT TRANSACTION
"#;

mod part2;
use part2::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_script() {
        assert!(get_script("greeting").is_some());
        assert!(get_script("kb_search").is_some());
        assert!(get_script("nonexistent").is_none());
    }

    #[test]
    fn test_available_scripts() {
        let scripts = available_scripts();
        assert!(!scripts.is_empty());
        assert!(scripts.contains(&"greeting"));
        assert!(scripts.contains(&"attendance"));
    }

    #[test]
    fn test_all_scripts() {
        let scripts = all_scripts();
        assert_eq!(scripts.len(), available_scripts().len());
    }

    #[test]
    fn test_greeting_script_content() {
        let script = get_script("greeting").unwrap();
        assert!(script.contains("TALK"));
        assert!(script.contains("HEAR"));
        assert!(script.contains("greeting"));
    }

    #[test]
    fn test_kb_search_script_content() {
        let script = get_script("kb_search").unwrap();
        assert!(script.contains("FIND"));
        assert!(script.contains("TRANSFER HUMAN"));
    }

    #[test]
    fn test_attendance_script_content() {
        let script = get_script("attendance").unwrap();
        assert!(script.contains("TRANSFER HUMAN"));
        assert!(script.contains("GET_QUEUE_POSITION"));
    }

    #[test]
    fn test_error_handling_script_content() {
        let script = get_script("error_handling").unwrap();
        assert!(script.contains("ON ERROR RESUME NEXT"));
        assert!(script.contains("ERR"));
    }

    #[test]
    fn test_llm_tools_script_content() {
        let script = get_script("llm_tools").unwrap();
        assert!(script.contains("TOOL"));
        assert!(script.contains("ASK llm"));
        assert!(script.contains("WITH TOOLS"));
    }

    #[test]
    fn test_data_operations_script_content() {
        let script = get_script("data_operations").unwrap();
        assert!(script.contains("SAVE"));
        assert!(script.contains("FIND"));
        assert!(script.contains("UPDATE"));
        assert!(script.contains("DELETE"));
        assert!(script.contains("TRANSACTION"));
    }

    #[test]
    fn test_http_integration_script_content() {
        let script = get_script("http_integration").unwrap();
        assert!(script.contains("GET"));
        assert!(script.contains("POST"));
        assert!(script.contains("GRAPHQL"));
        assert!(script.contains("SOAP"));
    }

    #[test]
    fn test_menu_flow_script_content() {
        let script = get_script("menu_flow").unwrap();
        assert!(script.contains("SELECT CASE"));
        assert!(script.contains("GOSUB"));
        assert!(script.contains("RETURN"));
    }

    #[test]
    fn test_simple_echo_script_content() {
        let script = get_script("simple_echo").unwrap();
        assert!(script.contains("HEAR"));
        assert!(script.contains("TALK"));
        assert!(script.contains("GOTO"));
    }

    #[test]
    fn test_variables_script_content() {
        let script = get_script("variables").unwrap();
        assert!(script.contains("DIM"));
        assert!(script.contains("FOR"));
        assert!(script.contains("NEXT"));
        assert!(script.contains("UCASE$"));
    }
}
