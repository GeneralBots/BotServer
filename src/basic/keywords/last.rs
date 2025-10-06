use rhai::Dynamic;
use rhai::Engine;

pub fn last_keyword(engine: &mut Engine) {
    engine
        .register_custom_syntax(&["LAST", "(", "$expr$", ")"], false, {
            move |context, inputs| {
                let input_string = context.eval_expression_tree(&inputs[0])?;
                let input_str = input_string.to_string();

                // Extrai a última palavra dividindo por espaço
                let last_word = input_str
                    .split_whitespace()
                    .last()
                    .unwrap_or("")
                    .to_string();

                Ok(Dynamic::from(last_word))
            }
        })
        .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::{Engine, Scope};

    #[test]
    fn test_last_keyword_basic() {
        let mut engine = Engine::new();
        last_keyword(&mut engine);
        
        let result: String = engine.eval("LAST(\"hello world\")").unwrap();
        assert_eq!(result, "world");
    }

    #[test]
    fn test_last_keyword_single_word() {
        let mut engine = Engine::new();
        last_keyword(&mut engine);
        
        let result: String = engine.eval("LAST(\"hello\")").unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_last_keyword_empty_string() {
        let mut engine = Engine::new();
        last_keyword(&mut engine);
        
        let result: String = engine.eval("LAST(\"\")").unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_last_keyword_multiple_spaces() {
        let mut engine = Engine::new();
        last_keyword(&mut engine);
        
        let result: String = engine.eval("LAST(\"hello    world    \")").unwrap();
        assert_eq!(result, "world");
    }

    #[test]
    fn test_last_keyword_tabs_and_newlines() {
        let mut engine = Engine::new();
        last_keyword(&mut engine);
        
        let result: String = engine.eval("LAST(\"hello\tworld\n\")").unwrap();
        assert_eq!(result, "world");
    }

    #[test]
    fn test_last_keyword_with_variable() {
        let mut engine = Engine::new();
        last_keyword(&mut engine);
        let mut scope = Scope::new();
        
        scope.push("text", "this is a test");
        let result: String = engine.eval_with_scope(&mut scope, "LAST(text)").unwrap();
        
        assert_eq!(result, "test");
    }

    #[test]
    fn test_last_keyword_whitespace_only() {
        let mut engine = Engine::new();
        last_keyword(&mut engine);
        
        let result: String = engine.eval("LAST(\"   \")").unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_last_keyword_mixed_whitespace() {
        let mut engine = Engine::new();
        last_keyword(&mut engine);
        
        let result: String = engine.eval("LAST(\"hello\t \n world  \t final\")").unwrap();
        assert_eq!(result, "final");
    }

    #[test]
    fn test_last_keyword_expression() {
        let mut engine = Engine::new();
        last_keyword(&mut engine);
        
        // Test with string concatenation
        let result: String = engine.eval("LAST(\"hello\" + \" \" + \"world\")").unwrap();
        assert_eq!(result, "world");
    }

    #[test]
    fn test_last_keyword_unicode() {
        let mut engine = Engine::new();
        last_keyword(&mut engine);
        
        let result: String = engine.eval("LAST(\"hello 世界 мир world\")").unwrap();
        assert_eq!(result, "world");
    }

    #[test]
    fn test_last_keyword_in_expression() {
        let mut engine = Engine::new();
        last_keyword(&mut engine);
        
        // Test using the result in another expression
        let result: bool = engine.eval("LAST(\"hello world\") == \"world\"").unwrap();
        assert!(result);
    }

    #[test]
    fn test_last_keyword_complex_scenario() {
        let mut engine = Engine::new();
        last_keyword(&mut engine);
        let mut scope = Scope::new();
        
        scope.push("sentence", "The quick brown fox jumps over the lazy dog");
        let result: String = engine.eval_with_scope(&mut scope, "LAST(sentence)").unwrap();
        
        assert_eq!(result, "dog");
    }

    #[test]
    #[should_panic] // This should fail because the syntax expects parentheses
    fn test_last_keyword_missing_parentheses() {
        let mut engine = Engine::new();
        last_keyword(&mut engine);
        
        // This should fail - missing parentheses
        let _: String = engine.eval("LAST \"hello world\"").unwrap();
    }

    #[test]
    #[should_panic] // This should fail because of incomplete syntax
    fn test_last_keyword_missing_closing_parenthesis() {
        let mut engine = Engine::new();
        last_keyword(&mut engine);
        
        // This should fail - missing closing parenthesis
        let _: String = engine.eval("LAST(\"hello world\"").unwrap();
    }

    #[test]
    #[should_panic] // This should fail because of incomplete syntax
    fn test_last_keyword_missing_opening_parenthesis() {
        let mut engine = Engine::new();
        last_keyword(&mut engine);
        
        // This should fail - missing opening parenthesis
        let _: String = engine.eval("LAST \"hello world\")").unwrap();
    }

    #[test]
    fn test_last_keyword_dynamic_type() {
        let mut engine = Engine::new();
        last_keyword(&mut engine);
        
        // Test that the function returns the correct Dynamic type
        let result = engine.eval::<Dynamic>("LAST(\"test string\")").unwrap();
        assert!(result.is::<String>());
        assert_eq!(result.to_string(), "string");
    }

    #[test]
    fn test_last_keyword_nested_expression() {
        let mut engine = Engine::new();
        last_keyword(&mut engine);
        
        // Test with a more complex nested expression
        let result: String = engine.eval("LAST(\"The result is: \" + \"hello world\")").unwrap();
        assert_eq!(result, "world");
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_last_keyword_in_script() {
        let mut engine = Engine::new();
        last_keyword(&mut engine);
        
        let script = r#"
            let sentence1 = "first second third";
            let sentence2 = "alpha beta gamma";
            
            let last1 = LAST(sentence1);
            let last2 = LAST(sentence2);
            
            last1 + " and " + last2
        "#;
        
        let result: String = engine.eval(script).unwrap();
        assert_eq!(result, "third and gamma");
    }

    #[test]
    fn test_last_keyword_with_function() {
        let mut engine = Engine::new();
        last_keyword(&mut engine);
        
        // Register a function that returns a string
        engine.register_fn("get_name", || -> String { "john doe".to_string() });
        
        let result: String = engine.eval("LAST(get_name())").unwrap();
        assert_eq!(result, "doe");
    }

    #[test]
    fn test_last_keyword_multiple_calls() {
        let mut engine = Engine::new();
        last_keyword(&mut engine);
        
        let script = r#"
            let text1 = "apple banana cherry";
            let text2 = "cat dog elephant";
            
            let result1 = LAST(text1);
            let result2 = LAST(text2);
            
            result1 + "-" + result2
        "#;
        
        let result: String = engine.eval(script).unwrap();
        assert_eq!(result, "cherry-elephant");
    }
}