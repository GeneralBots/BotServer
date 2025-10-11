use rhai::Dynamic;
use rhai::Engine;

pub fn first_keyword(engine: &mut Engine) {
    engine
        .register_custom_syntax(&["FIRST", "$expr$"], false, {
            move |context, inputs| {
                let input_string = context.eval_expression_tree(&inputs[0])?;
                let input_str = input_string.to_string();

                let first_word = input_str
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string();

                Ok(Dynamic::from(first_word))
            }
        })
        .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::Engine;

    fn setup_engine() -> Engine {
        let mut engine = Engine::new();
        first_keyword(&mut engine);
        engine
    }

    #[test]
    fn test_first_keyword_basic() {
        let engine = setup_engine();

        let result = engine
            .eval::<String>(
                r#"
            FIRST "hello world"
        "#,
            )
            .unwrap();

        assert_eq!(result, "hello");
    }

    #[test]
    fn test_first_keyword_single_word() {
        let engine = setup_engine();

        let result = engine
            .eval::<String>(
                r#"
            FIRST "single"
        "#,
            )
            .unwrap();

        assert_eq!(result, "single");
    }

    #[test]
    fn test_first_keyword_multiple_spaces() {
        let engine = setup_engine();

        let result = engine
            .eval::<String>(
                r#"
            FIRST "   leading spaces"
        "#,
            )
            .unwrap();

        assert_eq!(result, "leading");
    }

    #[test]
    fn test_first_keyword_empty_string() {
        let engine = setup_engine();

        let result = engine
            .eval::<String>(
                r#"
            FIRST ""
        "#,
            )
            .unwrap();

        assert_eq!(result, "");
    }

    #[test]
    fn test_first_keyword_whitespace_only() {
        let engine = setup_engine();

        let result = engine
            .eval::<String>(
                r#"
            FIRST "   "
        "#,
            )
            .unwrap();

        assert_eq!(result, "");
    }

    #[test]
    fn test_first_keyword_with_tabs() {
        let engine = setup_engine();

        let result = engine
            .eval::<String>(
                r#"
            FIRST "	tab	separated	words"
        "#,
            )
            .unwrap();

        assert_eq!(result, "tab");
    }

    #[test]
    fn test_first_keyword_with_variable() {
        let engine = setup_engine();

        let result = engine
            .eval::<String>(
                r#"
            let text = "variable test";
            FIRST text
        "#,
            )
            .unwrap();

        assert_eq!(result, "variable");
    }

    #[test]
    fn test_first_keyword_with_expression() {
        let engine = setup_engine();

        let result = engine
            .eval::<String>(
                r#"
            FIRST "one two " + "three four"
        "#,
            )
            .unwrap();

        assert_eq!(result, "one");
    }

    #[test]
    fn test_first_keyword_mixed_whitespace() {
        let engine = setup_engine();

        let result = engine
            .eval::<String>(
                r#"
            FIRST "  multiple   spaces   between   words  "
        "#,
            )
            .unwrap();

        assert_eq!(result, "multiple");
    }

    #[test]
    fn test_first_keyword_special_characters() {
        let engine = setup_engine();

        let result = engine
            .eval::<String>(
                r#"
            FIRST "hello-world example"
        "#,
            )
            .unwrap();

        assert_eq!(result, "hello-world");
    }
}
