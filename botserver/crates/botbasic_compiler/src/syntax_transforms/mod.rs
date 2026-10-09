pub(crate) use regex::Regex;
pub(crate) use std::collections::BTreeSet;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn for_each_next_becomes_rhai_native_for() {
        let script = "FOR EACH candidate IN allowed\nPRINT candidate\nNEXT\n";
        let out = convert_multiword_keywords(script);
        assert!(out.contains("for candidate in (allowed) {"), "got: {out}");
        assert!(out.contains("}\n"), "closing brace missing: {out}");
        assert!(!out.to_uppercase().contains("FOR_EACH"), "unconverted form leaked: {out}");
    }

    #[test]
    fn underscored_for_each_from_ast_self_heals_at_runtime() {
        // Exactly the shape stored in compiled .ast files.
        let script = "allowed = SPLIT(TAXONOMY, \",\");\nFOR_EACH candidate in allowed;\nif candidate == answer {\n  category = candidate;\n}\nNEXT;\n";
        let out = convert_multiword_keywords(script);
        assert!(out.contains("for candidate in (allowed) {"), "got: {out}");
        assert!(!out.contains("NEXT"), "bare NEXT leaked: {out}");
    }

    #[test]
    fn nested_for_each_pairs_with_named_next() {
        let script = "FOR EACH a IN xs\nFOR EACH b IN ys\nPRINT a + b\nNEXT b\nNEXT a\n";
        let out = convert_multiword_keywords(script);
        assert_eq!(out.matches("for ").count(), 2, "got: {out}");
        assert_eq!(out.matches("}\n").count(), 2, "got: {out}");
    }

    #[test]
    fn get_path_rewrite_and_protected_forms() {
        let out = convert_multiword_keywords("GET \"/gbdialog/config.txt\"\n");
        assert!(out.contains("get_file("), "got: {out}");
        let out = convert_multiword_keywords("GET FROM customers WHERE id = 1\n");
        assert!(!out.contains("get_file("), "GET FROM must not be rewritten: {out}");
    }

    #[test]
    fn create_file_expression_path_becomes_create_file_call() {
        // classify_media.bas shape: path is an expression, data is an expression.
        let out = convert_multiword_keywords(
            "CREATE FILE destination + \".meta.txt\" WITH \"category=\" + category\n",
        );
        assert!(
            out.contains("create_file(destination + \".meta.txt\", \"category=\" + category);"),
            "got: {out}"
        );
        assert!(!out.to_uppercase().contains("CREATE FILE"), "unconverted form leaked: {out}");
    }

    #[test]
    fn create_file_literal_path_becomes_create_file_call() {
        let out = convert_multiword_keywords("CREATE FILE \"x.txt\" WITH \"hello\"\n");
        assert!(out.contains("create_file(\"x.txt\", \"hello\");"), "got: {out}");
    }

    #[test]
    fn create_file_call_with_equals_in_string_is_not_let_prefixed() {
        // classify_media.bas: the data argument contains `=` inside a string;
        // the assignment heuristic must not mistake the call for `let x = ...`.
        let script = "create_file(destination + \".meta.txt\", \"category=\" + category);\n";
        let out = convert_if_then_syntax(script);
        assert!(!out.contains("let create_file"), "got: {out}");
        assert!(out.contains("create_file(destination + \".meta.txt\""), "got: {out}");
    }

    #[test]
    fn bare_today_and_now_become_calls() {
        // classify_media.bas shape: `let today = TODAY;` — TODAY is registered
        // as a function at runtime, so the bare form must become a call.
        let out = convert_multiword_keywords("let today = TODAY;\nlet stamp = NOW;\n");
        assert!(out.contains("TODAY()"), "got: {out}");
        assert!(out.contains("NOW()"), "got: {out}");
        let out = convert_multiword_keywords("let a = TODAY();\n");
        assert_eq!(out.matches("TODAY()").count(), 1, "no double call: {out}");
        // Idempotency: an AST that already contains TODAY() must survive the
        // runtime self-heal unchanged (the lookahead-less backtracking bug
        // produced TODAY()()).
        let out = convert_multiword_keywords("let today = TODAY();\n");
        assert!(out.contains("TODAY();"), "double call: {out}");
        assert!(!out.contains("TODAY()()"), "double call: {out}");
    }

    #[test]
    fn today_inside_string_literal_is_not_rewritten() {
        let out = convert_multiword_keywords("TALK \"TODAY is a good day\"\n");
        assert!(out.contains("TODAY is a good day"), "string mangled: {out}");
        assert!(!out.contains("TODAY()"), "got: {out}");
    }

    #[test]
    fn perception_keywords_use_function_forms() {
        let out = convert_multiword_keywords("DESCRIBE VIDEO path\nON ERROR RESUME NEXT\n");
        assert!(out.contains("describe_video("), "got: {out}");
        assert!(out.contains("on_error_resume_next("), "got: {out}");
    }

    #[test]
    fn on_event_declaration_collapses_to_a_noop_call() {
        // The tool being compiled IS the tool that runs, so the declaration
        // names only the event; the compiled form must be the no-op runtime
        // call, with the quotes stripped exactly once.
        let out = convert_multiword_keywords("ON EVENT \"media_uploaded\"\nTALK \"done\"\n");
        assert!(out.contains("on_event(\"media_uploaded\")"), "got: {out}");
        assert!(!out.contains("CALL"), "tool name must not appear: {out}");
        assert!(!out.to_uppercase().contains("ON EVENT"), "unconverted form leaked: {out}");
    }

    #[test]
    fn on_event_accepts_a_bare_identifier() {
        let out = convert_multiword_keywords("ON EVENT media_uploaded\n");
        assert!(out.contains("on_event(\"media_uploaded\")"), "got: {out}");
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

pub use part1::*;
pub use part2::*;
pub use part3::*;
pub use part4::*;
pub use part5::*;
