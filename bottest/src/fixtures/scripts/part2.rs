//! Split from `mod.rs` — 277 lines.

pub const HTTP_INTEGRATION_SCRIPT: &str = r#"
' HTTP Integration Script
' Demonstrates POST, GET, GRAPHQL, SOAP calls

REM Simple GET request
weather = GET "https://api.weather.com/v1/current?location=NYC" HEADERS "Authorization: Bearer ${API_KEY}"
TALK "Current weather: " + weather.temperature + "°F"

REM POST request with JSON body
payload.name = "Test Order"
payload.items = ["item1", "item2"]
payload.total = 150.00

response = POST "https://api.example.com/orders" BODY payload HEADERS "Content-Type: application/json"

IF response.status = 200 THEN
    TALK "Order created: " + response.body.order_id
ELSE
    TALK "Failed to create order: " + response.error
END IF

REM GraphQL query
query$ = "query GetUser($id: ID!) { user(id: $id) { name email } }"
variables.id = "12345"

gql_response = GRAPHQL "https://api.example.com/graphql" QUERY query$ VARIABLES variables
TALK "User: " + gql_response.data.user.name

REM SOAP request
soap_body$ = "<GetProduct><SKU>ABC123</SKU></GetProduct>"
soap_response = SOAP "https://api.example.com/soap" ACTION "GetProduct" BODY soap_body$
TALK "Product: " + soap_response.ProductName
"#;

pub const MENU_FLOW_SCRIPT: &str = r#"
' Menu Flow Script
' Demonstrates interactive menu-based conversation

REM Show main menu
main_menu:
TALK "Please select an option:"
TALK "1. Check order status"
TALK "2. Track shipment"
TALK "3. Return an item"
TALK "4. Speak with an agent"
TALK "5. Exit"

HEAR choice$

SELECT CASE VAL(choice$)
    CASE 1
        GOSUB check_order
    CASE 2
        GOSUB track_shipment
    CASE 3
        GOSUB return_item
    CASE 4
        TRANSFER HUMAN
    CASE 5
        TALK "Thank you for using our service. Goodbye!"
        END
    CASE ELSE
        TALK "Invalid option. Please try again."
        GOTO main_menu
END SELECT

GOTO main_menu

check_order:
    TALK "Please enter your order number:"
    HEAR orderNum$
    order = FIND_ONE "orders" WHERE "order_number = '" + orderNum$ + "'"
    IF order THEN
        TALK "Order " + orderNum$ + " status: " + order.status
        TALK "Last updated: " + order.updated_at
    ELSE
        TALK "Order not found. Please check the number and try again."
    END IF
    RETURN

track_shipment:
    TALK "Please enter your tracking number:"
    HEAR trackingNum$
    tracking = GET "https://api.shipping.com/track/" + trackingNum$
    IF tracking.status = 200 THEN
        TALK "Your package is: " + tracking.body.status
        TALK "Location: " + tracking.body.location
    ELSE
        TALK "Could not find tracking information."
    END IF
    RETURN

return_item:
    TALK "Please enter the order number for the return:"
    HEAR returnOrder$
    TALK "What is the reason for return?"
    TALK "1. Defective"
    TALK "2. Wrong item"
    TALK "3. Changed mind"
    HEAR returnReason$

    return_request.order_number = returnOrder$
    return_request.reason = returnReason$
    return_request.status = "pending"
    SAVE "returns" return_request

    TALK "Return request created. Reference: " + return_request.id
    TALK "You'll receive a return label via email within 24 hours."
    RETURN
"#;

pub const SIMPLE_ECHO_SCRIPT: &str = r#"
' Simple Echo Script
' Echoes back whatever the user says

TALK "Echo Bot: I will repeat everything you say. Type 'quit' to exit."

echo_loop:
HEAR input$

IF UCASE$(input$) = "QUIT" THEN
    TALK "Goodbye!"
    END
END IF

TALK "You said: " + input$
GOTO echo_loop
"#;

pub const VARIABLES_SCRIPT: &str = r#"
' Variables and Expressions Script
' Demonstrates variable types and operations

REM String variables
firstName$ = "John"
lastName$ = "Doe"
fullName$ = firstName$ + " " + lastName$
TALK "Full name: " + fullName$

REM Numeric variables
price = 99.99
quantity = 3
subtotal = price * quantity
tax = subtotal * 0.08
total = subtotal + tax
TALK "Total: $" + STR$(total)

REM Arrays
DIM products$(5)
products$(0) = "Widget"
products$(1) = "Gadget"
products$(2) = "Gizmo"

FOR i = 0 TO 2
    TALK "Product " + STR$(i + 1) + ": " + products$(i)
NEXT i

REM Built-in functions
text$ = "  Hello World  "
TALK "Original: '" + text$ + "'"
TALK "Trimmed: '" + TRIM$(text$) + "'"
TALK "Upper: '" + UCASE$(text$) + "'"
TALK "Lower: '" + LCASE$(text$) + "'"
TALK "Length: " + STR$(LEN(TRIM$(text$)))

REM Date/time functions
today$ = DATE$
now$ = TIME$
TALK "Today is: " + today$ + " at " + now$
"#;

#[cfg(test)]
mod tests {
    use super::super::*;

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
