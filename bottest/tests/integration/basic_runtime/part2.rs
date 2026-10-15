//! Split from `basic_runtime.rs` — 324 lines.

use super::*;

#[test]
pub(crate) fn test_conditional_response() {
    let output = OutputCollector::new();
    let input = InputProvider::new(vec!["yes".to_string()]);
    let engine = create_conversation_engine(output.clone(), input);

    engine
        .eval::<()>(
            r#"
        TALK("Do you want to continue? (yes/no)");
        let response = HEAR();
        if UPPER(response) == "YES" {
            TALK("Great, let's continue!");
        } else {
            TALK("Goodbye!");
        }
    "#,
        )
        .unwrap();

    let messages = output.get_messages();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[1], "Great, let's continue!");
}

#[test]
pub(crate) fn test_keyword_detection() {
    let output = OutputCollector::new();
    let input = InputProvider::new(vec!["I need help with my order".to_string()]);
    let engine = create_conversation_engine(output.clone(), input);

    engine
        .eval::<()>(
            r#"
        let message = HEAR();
        let upper_msg = UPPER(message);

        if INSTR(upper_msg, "HELP") > 0 {
            TALK("I can help you! What do you need?");
        } else if INSTR(upper_msg, "ORDER") > 0 {
            TALK("Let me look up your order.");
        } else {
            TALK("How can I assist you today?");
        }
    "#,
        )
        .unwrap();

    let messages = output.get_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0], "I can help you! What do you need?");
}

#[test]
pub(crate) fn test_variable_assignment() {
    let engine = create_basic_engine();

    let result: i64 = engine
        .eval(
            r"
        let x = 10;
        let y = 20;
        let z = x + y;
        z
    ",
        )
        .unwrap();
    assert_eq!(result, 30);
}

#[test]
pub(crate) fn test_string_variables() {
    let engine = create_basic_engine();

    let result: String = engine
        .eval(
            r#"
        let first_name = "John";
        let last_name = "Doe";
        let full_name = first_name + " " + last_name;
        UPPER(full_name)
    "#,
        )
        .unwrap();
    assert_eq!(result, "JOHN DOE");
}

#[test]
pub(crate) fn test_numeric_expressions() {
    let engine = create_basic_engine();

    let result: i64 = engine.eval("2 + 3 * 4").unwrap();
    assert_eq!(result, 14);

    let result: i64 = engine.eval("(2 + 3) * 4").unwrap();
    assert_eq!(result, 20);

    let result: i64 = engine.eval("ABS(-5) + MAX(3, 7)").unwrap();
    assert_eq!(result, 12);
}

#[test]
pub(crate) fn test_for_loop() {
    let output = OutputCollector::new();
    let input = InputProvider::new(vec![]);
    let engine = create_conversation_engine(output.clone(), input);

    engine
        .eval::<()>(
            r#"
        for i in 1..4 {
            TALK("Count: " + i.to_string());
        }
    "#,
        )
        .unwrap();

    let messages = output.get_messages();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0], "Count: 1");
    assert_eq!(messages[1], "Count: 2");
    assert_eq!(messages[2], "Count: 3");
}

#[test]
pub(crate) fn test_while_loop() {
    let engine = create_basic_engine();

    let result: i64 = engine
        .eval(
            r"
        let count = 0;
        let sum = 0;
        while count < 5 {
            sum = sum + count;
            count = count + 1;
        }
        sum
    ",
        )
        .unwrap();
    assert_eq!(result, 10);
}

#[test]
pub(crate) fn test_division_by_zero() {
    let engine = create_basic_engine();

    let result = engine.eval::<f64>("10.0 / 0.0");
    if let Ok(val) = result {
        assert!(val.is_infinite() || val.is_nan());
    }
}

#[test]
pub(crate) fn test_invalid_function_call() {
    let engine = create_basic_engine();

    let result = engine.eval::<String>(r#"UNDEFINED_FUNCTION("test")"#);
    assert!(result.is_err());
}

#[test]
pub(crate) fn test_type_mismatch() {
    let engine = create_basic_engine();

    let result = engine.eval::<i64>(r#"ABS("not a number")"#);
    assert!(result.is_err());
}

#[test]
pub(crate) fn test_greeting_script_logic() {
    let output = OutputCollector::new();
    let input = InputProvider::new(vec!["HELP".to_string()]);
    let engine = create_conversation_engine(output.clone(), input);

    engine
        .eval::<()>(
            r#"
        let greeting = "Hello! Welcome to our service.";
        TALK(greeting);

        let user_input = HEAR();

        if INSTR(UPPER(user_input), "HELP") > 0 {
            TALK("I can help you with: Products, Support, or Billing.");
        } else if INSTR(UPPER(user_input), "BYE") > 0 {
            TALK("Goodbye! Have a great day!");
        } else {
            TALK("How can I assist you today?");
        }
    "#,
        )
        .unwrap();

    let messages = output.get_messages();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0], "Hello! Welcome to our service.");
    assert!(messages[1].contains("help"));
}

#[test]
pub(crate) fn test_menu_flow_logic() {
    let output = OutputCollector::new();
    let input = InputProvider::new(vec!["1".to_string()]);
    let engine = create_conversation_engine(output.clone(), input);

    engine
        .eval::<()>(
            r#"
        TALK("Please select an option:");
        TALK("1. Check order status");
        TALK("2. Track shipment");
        TALK("3. Contact support");

        let choice = HEAR();
        let choice_num = VAL(choice);

        if choice_num == 1.0 {
            TALK("Please enter your order number.");
        } else if choice_num == 2.0 {
            TALK("Please enter your tracking number.");
        } else if choice_num == 3.0 {
            TALK("Connecting you to support...");
        } else {
            TALK("Invalid option. Please try again.");
        }
    "#,
        )
        .unwrap();

    let messages = output.get_messages();
    assert_eq!(messages.len(), 5);
    assert_eq!(messages[4], "Please enter your order number.");
}

#[test]
pub(crate) fn test_echo_bot_logic() {
    let output = OutputCollector::new();
    let input = InputProvider::new(vec!["Hello".to_string(), "How are you?".to_string()]);
    let engine = create_conversation_engine(output.clone(), input);

    engine
        .eval::<()>(
            r#"
        TALK("Echo Bot: I will repeat what you say.");

        let input1 = HEAR();
        TALK("You said: " + input1);

        let input2 = HEAR();
        TALK("You said: " + input2);
    "#,
        )
        .unwrap();

    let messages = output.get_messages();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0], "Echo Bot: I will repeat what you say.");
    assert_eq!(messages[1], "You said: Hello");
    assert_eq!(messages[2], "You said: How are you?");
}

#[test]
pub(crate) fn test_order_lookup_simulation() {
    let output = OutputCollector::new();
    let input = InputProvider::new(vec!["ORD-12345".to_string()]);
    let engine = create_conversation_engine(output.clone(), input);

    engine
        .eval::<()>(
            r#"
        TALK("Please enter your order number:");
        let order_num = HEAR();

        let is_valid = INSTR(order_num, "ORD-") == 1 && LEN(order_num) >= 9;

        if is_valid {
            TALK("Looking up order " + order_num + "...");
            TALK("Order Status: Shipped");
            TALK("Estimated delivery: 3-5 business days");
        } else {
            TALK("Invalid order number format. Please use ORD-XXXXX format.");
        }
    "#,
        )
        .unwrap();

    let messages = output.get_messages();
    assert_eq!(messages.len(), 4);
    assert!(messages[1].contains("ORD-12345"));
    assert!(messages[2].contains("Shipped"));
}

#[test]
pub(crate) fn test_price_calculation() {
    let output = OutputCollector::new();
    let input = InputProvider::new(vec!["3".to_string()]);
    let engine = create_conversation_engine(output.clone(), input);

    engine
        .eval::<()>(
            r#"
        let price = 29.99;
        TALK("Each widget costs $" + price.to_string());
        TALK("How many would you like?");

        let quantity = VAL(HEAR());
        let subtotal = price * quantity;
        let tax = subtotal * 0.08;
        let total = subtotal + tax;

        TALK("Subtotal: $" + subtotal.to_string());
        TALK("Tax (8%): $" + ROUND(tax * 100.0).to_string());
        TALK("Total: $" + ROUND(total * 100.0).to_string());
    "#,
        )
        .unwrap();

    let messages = output.get_messages();
    assert_eq!(messages.len(), 5);
    assert!(messages[0].contains("29.99"));
    assert!(messages[2].contains("89.97"));
}
