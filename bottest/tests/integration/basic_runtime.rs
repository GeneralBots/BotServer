use rhai::Engine;
use std::sync::{Arc, Mutex};

fn create_basic_engine() -> Engine {
    let mut engine = Engine::new();

    engine.register_fn("INSTR", |haystack: &str, needle: &str| -> i64 {
        if haystack.is_empty() || needle.is_empty() {
            return 0;
        }
        match haystack.find(needle) {
            Some(pos) => (pos + 1) as i64,
            None => 0,
        }
    });
    engine.register_fn("UPPER", |s: &str| -> String { s.to_uppercase() });
    engine.register_fn("UCASE", |s: &str| -> String { s.to_uppercase() });
    engine.register_fn("LOWER", |s: &str| -> String { s.to_lowercase() });
    engine.register_fn("LCASE", |s: &str| -> String { s.to_lowercase() });
    engine.register_fn("LEN", |s: &str| -> i64 { s.len() as i64 });
    engine.register_fn("TRIM", |s: &str| -> String { s.trim().to_string() });
    engine.register_fn("LTRIM", |s: &str| -> String { s.trim_start().to_string() });
    engine.register_fn("RTRIM", |s: &str| -> String { s.trim_end().to_string() });
    engine.register_fn("LEFT", |s: &str, count: i64| -> String {
        let count = count.max(0) as usize;
        s.chars().take(count).collect()
    });
    engine.register_fn("RIGHT", |s: &str, count: i64| -> String {
        let count = count.max(0) as usize;
        let len = s.chars().count();
        if count >= len {
            s.to_string()
        } else {
            s.chars().skip(len - count).collect()
        }
    });
    engine.register_fn("MID", |s: &str, start: i64, length: i64| -> String {
        let start_idx = if start < 1 { 0 } else { (start - 1) as usize };
        let len = length.max(0) as usize;
        s.chars().skip(start_idx).take(len).collect()
    });
    engine.register_fn("REPLACE", |s: &str, find: &str, replace: &str| -> String {
        s.replace(find, replace)
    });

    engine.register_fn("ABS", |n: i64| -> i64 { n.abs() });
    engine.register_fn("ABS", |n: f64| -> f64 { n.abs() });
    engine.register_fn("ROUND", |n: f64| -> i64 { n.round() as i64 });
    engine.register_fn("INT", |n: f64| -> i64 { n.trunc() as i64 });
    engine.register_fn("FIX", |n: f64| -> i64 { n.trunc() as i64 });
    engine.register_fn("FLOOR", |n: f64| -> i64 { n.floor() as i64 });
    engine.register_fn("CEIL", |n: f64| -> i64 { n.ceil() as i64 });
    engine.register_fn("MAX", |a: i64, b: i64| -> i64 { a.max(b) });
    engine.register_fn("MIN", |a: i64, b: i64| -> i64 { a.min(b) });
    engine.register_fn("MOD", |a: i64, b: i64| -> i64 { a % b });
    engine.register_fn("SGN", |n: i64| -> i64 { n.signum() });
    engine.register_fn("SQRT", |n: f64| -> f64 { n.sqrt() });
    engine.register_fn("SQR", |n: f64| -> f64 { n.sqrt() });
    engine.register_fn("POW", |base: f64, exp: f64| -> f64 { base.powf(exp) });
    engine.register_fn("LOG", |n: f64| -> f64 { n.ln() });
    engine.register_fn("LOG10", |n: f64| -> f64 { n.log10() });
    engine.register_fn("EXP", |n: f64| -> f64 { n.exp() });
    engine.register_fn("SIN", |n: f64| -> f64 { n.sin() });
    engine.register_fn("COS", |n: f64| -> f64 { n.cos() });
    engine.register_fn("TAN", |n: f64| -> f64 { n.tan() });
    engine.register_fn("PI", || -> f64 { std::f64::consts::PI });

    engine.register_fn("VAL", |s: &str| -> f64 {
        s.trim().parse::<f64>().unwrap_or(0.0)
    });
    engine.register_fn("STR", |n: i64| -> String { n.to_string() });
    engine.register_fn("STR", |n: f64| -> String { n.to_string() });

    engine.register_fn("IS_NUMERIC", |value: &str| -> bool {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return false;
        }
        trimmed.parse::<i64>().is_ok() || trimmed.parse::<f64>().is_ok()
    });

    engine
}

#[derive(Clone, Default)]
struct OutputCollector {
    messages: Arc<Mutex<Vec<String>>>,
}

impl OutputCollector {
    fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn add_message(&self, msg: String) {
        let mut messages = self.messages.lock().unwrap();
        messages.push(msg);
    }

    fn get_messages(&self) -> Vec<String> {
        self.messages.lock().unwrap().clone()
    }
}

#[derive(Clone)]
struct InputProvider {
    inputs: Arc<Mutex<Vec<String>>>,
    index: Arc<Mutex<usize>>,
}

impl InputProvider {
    fn new(inputs: Vec<String>) -> Self {
        Self {
            inputs: Arc::new(Mutex::new(inputs)),
            index: Arc::new(Mutex::new(0)),
        }
    }

    fn next_input(&self) -> String {
        let inputs = self.inputs.lock().unwrap();
        let mut index = self.index.lock().unwrap();
        if *index < inputs.len() {
            let input = inputs[*index].clone();
            *index += 1;
            input
        } else {
            String::new()
        }
    }
}

fn create_conversation_engine(output: OutputCollector, input: InputProvider) -> Engine {
    let mut engine = create_basic_engine();

    engine.register_fn("TALK", move |msg: &str| {
        output.add_message(msg.to_string());
    });

    engine.register_fn("HEAR", move || -> String { input.next_input() });

    engine
}

#[test]
fn test_string_concatenation_in_engine() {
    let engine = create_basic_engine();

    let result: String = engine
        .eval(r#"let a = "Hello"; let b = " World"; a + b"#)
        .unwrap();
    assert_eq!(result, "Hello World");
}

#[test]
fn test_string_functions_chain() {
    let engine = create_basic_engine();

    let result: String = engine.eval(r#"UPPER(TRIM("  hello  "))"#).unwrap();
    assert_eq!(result, "HELLO");

    let result: i64 = engine.eval(r#"LEN(TRIM("  test  "))"#).unwrap();
    assert_eq!(result, 4);
}

#[test]
fn test_substring_extraction() {
    let engine = create_basic_engine();

    let result: String = engine.eval(r#"LEFT("Hello World", 5)"#).unwrap();
    assert_eq!(result, "Hello");

    let result: String = engine.eval(r#"RIGHT("Hello World", 5)"#).unwrap();
    assert_eq!(result, "World");

    let result: String = engine.eval(r#"MID("Hello World", 7, 5)"#).unwrap();
    assert_eq!(result, "World");
}

#[test]
fn test_instr_function() {
    let engine = create_basic_engine();

    let result: i64 = engine.eval(r#"INSTR("Hello World", "World")"#).unwrap();
    assert_eq!(result, 7);

    let result: i64 = engine.eval(r#"INSTR("Hello World", "xyz")"#).unwrap();
    assert_eq!(result, 0);

    let result: i64 = engine.eval(r#"INSTR("Hello World", "o")"#).unwrap();
    assert_eq!(result, 5);
}

#[test]
fn test_replace_function() {
    let engine = create_basic_engine();

    let result: String = engine
        .eval(r#"REPLACE("Hello World", "World", "Rust")"#)
        .unwrap();
    assert_eq!(result, "Hello Rust");

    let result: String = engine.eval(r#"REPLACE("aaa", "a", "b")"#).unwrap();
    assert_eq!(result, "bbb");
}

#[test]
fn test_math_operations_chain() {
    let engine = create_basic_engine();

    let result: f64 = engine.eval("SQRT(ABS(-16.0))").unwrap();
    assert!((result - 4.0).abs() < f64::EPSILON);

    let result: i64 = engine.eval("MAX(ABS(-5), ABS(-10))").unwrap();
    assert_eq!(result, 10);
}

#[test]
fn test_rounding_functions() {
    let engine = create_basic_engine();

    let result: i64 = engine.eval("ROUND(3.7)").unwrap();
    assert_eq!(result, 4);

    let result: i64 = engine.eval("ROUND(3.2)").unwrap();
    assert_eq!(result, 3);

    let result: i64 = engine.eval("FLOOR(3.9)").unwrap();
    assert_eq!(result, 3);

    let result: i64 = engine.eval("FLOOR(-3.1)").unwrap();
    assert_eq!(result, -4);

    let result: i64 = engine.eval("CEIL(3.1)").unwrap();
    assert_eq!(result, 4);

    let result: i64 = engine.eval("CEIL(-3.9)").unwrap();
    assert_eq!(result, -3);
}

#[test]
fn test_trigonometric_functions() {
    let engine = create_basic_engine();

    let result: f64 = engine.eval("SIN(0.0)").unwrap();
    assert!((result - 0.0).abs() < f64::EPSILON);

    let result: f64 = engine.eval("COS(0.0)").unwrap();
    assert!((result - 1.0).abs() < f64::EPSILON);

    let pi: f64 = engine.eval("PI()").unwrap();
    assert!((pi - std::f64::consts::PI).abs() < f64::EPSILON);
}

#[test]
fn test_val_function() {
    let engine = create_basic_engine();

    let result: f64 = engine.eval(r#"VAL("42")"#).unwrap();
    assert!((result - 42.0).abs() < f64::EPSILON);

    let result: f64 = engine.eval(r#"VAL("3.5")"#).unwrap();
    assert!((result - 3.5).abs() < f64::EPSILON);

    let result: f64 = engine.eval(r#"VAL("invalid")"#).unwrap();
    assert!((result - 0.0).abs() < f64::EPSILON);
}

#[test]
fn test_talk_output() {
    let output = OutputCollector::new();
    let input = InputProvider::new(vec![]);
    let engine = create_conversation_engine(output.clone(), input);

    engine.eval::<()>(r#"TALK("Hello, World!")"#).unwrap();

    let messages = output.get_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0], "Hello, World!");
}

#[test]
fn test_talk_multiple_messages() {
    let output = OutputCollector::new();
    let input = InputProvider::new(vec![]);
    let engine = create_conversation_engine(output.clone(), input);

    engine
        .eval::<()>(
            r#"
        TALK("Line 1");
        TALK("Line 2");
        TALK("Line 3");
    "#,
        )
        .unwrap();

    let messages = output.get_messages();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0], "Line 1");
    assert_eq!(messages[1], "Line 2");
    assert_eq!(messages[2], "Line 3");
}

#[test]
fn test_hear_input() {
    let output = OutputCollector::new();
    let input = InputProvider::new(vec!["Hello from user".to_string()]);
    let engine = create_conversation_engine(output, input);

    let result: String = engine.eval("HEAR()").unwrap();
    assert_eq!(result, "Hello from user");
}

#[test]
fn test_talk_hear_conversation() {
    let output = OutputCollector::new();
    let input = InputProvider::new(vec!["John".to_string()]);
    let engine = create_conversation_engine(output.clone(), input);

    engine
        .eval::<()>(
            r#"
        TALK("What is your name?");
        let name = HEAR();
        TALK("Hello, " + name + "!");
    "#,
        )
        .unwrap();

    let messages = output.get_messages();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0], "What is your name?");
    assert_eq!(messages[1], "Hello, John!");
}

mod part2;
