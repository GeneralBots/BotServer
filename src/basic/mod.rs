use crate::shared::models::UserSession;
use crate::shared::state::AppState;
use log::info;
use rhai::{Dynamic, Engine, EvalAltResult};
use std::sync::Arc;

pub mod keywords;

use self::keywords::create_site::create_site_keyword;
use self::keywords::find::find_keyword;
use self::keywords::first::first_keyword;
use self::keywords::for_next::for_keyword;
use self::keywords::format::format_keyword;
use self::keywords::get::get_keyword;
use self::keywords::hear_talk::{
    hear_keyword, set_context_keyword, set_user_keyword, talk_keyword,
};
use self::keywords::last::last_keyword;
use self::keywords::llm_keyword::llm_keyword;
use self::keywords::on::on_keyword;
use self::keywords::print::print_keyword;
use self::keywords::set::set_keyword;
use self::keywords::set_schedule::set_schedule_keyword;
use self::keywords::wait::wait_keyword;

#[cfg(feature = "email")]
use self::keywords::create_draft_keyword;

#[cfg(feature = "web_automation")]
use self::keywords::get_website::get_website_keyword;

pub struct ScriptService {
    pub engine: Engine,
    state: Arc<AppState>,
    user: UserSession,
}

impl ScriptService {
    pub fn new(state: Arc<AppState>, user: UserSession) -> Self {
        let mut engine = Engine::new();

        engine.set_allow_anonymous_fn(true);
        engine.set_allow_looping(true);

        #[cfg(feature = "email")]
        create_draft_keyword(&state, user.clone(), &mut engine);

        create_site_keyword(&state, user.clone(), &mut engine);
        find_keyword(&state, user.clone(), &mut engine);
        for_keyword(&state, user.clone(), &mut engine);
        first_keyword(&mut engine);
        last_keyword(&mut engine);
        format_keyword(&mut engine);
        llm_keyword(&state, user.clone(), &mut engine);
        get_keyword(&state, user.clone(), &mut engine);
        set_keyword(&state, user.clone(), &mut engine);
        wait_keyword(&state, user.clone(), &mut engine);
        print_keyword(&state, user.clone(), &mut engine);
        on_keyword(&state, user.clone(), &mut engine);
        set_schedule_keyword(&state, user.clone(), &mut engine);
        hear_keyword(state.clone(), user.clone(), &mut engine);
        talk_keyword(state.clone(), user.clone(), &mut engine);
        set_context_keyword(&state, user.clone(), &mut engine);
        set_user_keyword(state.clone(), user.clone(), &mut engine);

        #[cfg(feature = "web_automation")]
        get_website_keyword(&state, user.clone(), &mut engine);

        ScriptService {
            engine,
            state,
            user,
        }
    }

    fn preprocess_basic_script(&self, script: &str) -> String {
        let mut result = String::new();
        let mut for_stack: Vec<usize> = Vec::new();
        let mut current_indent = 0;

        for line in script.lines() {
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("REM") {
                result.push_str(line);
                result.push('\n');
                continue;
            }

            if trimmed.starts_with("FOR EACH") {
                for_stack.push(current_indent);
                result.push_str(&" ".repeat(current_indent));
                result.push_str(trimmed);
                result.push_str("{\n");
                current_indent += 4;
                result.push_str(&" ".repeat(current_indent));
                result.push('\n');
                continue;
            }

            if trimmed.starts_with("NEXT") {
                if let Some(expected_indent) = for_stack.pop() {
                    if (current_indent - 4) != expected_indent {
                        panic!("NEXT without matching FOR EACH");
                    }
                    current_indent = current_indent - 4;
                    result.push_str(&" ".repeat(current_indent));
                    result.push_str("}\n");
                    result.push_str(&" ".repeat(current_indent));
                    result.push_str(trimmed);
                    result.push(';');
                    result.push('\n');
                    continue;
                } else {
                    panic!("NEXT without matching FOR EACH");
                }
            }

            if trimmed == "EXIT FOR" {
                result.push_str(&" ".repeat(current_indent));
                result.push_str(trimmed);
                result.push('\n');
                continue;
            }

            result.push_str(&" ".repeat(current_indent));

            let basic_commands = [
                "SET",
                "CREATE",
                "PRINT",
                "FOR",
                "FIND",
                "GET",
                "EXIT",
                "IF",
                "THEN",
                "ELSE",
                "END IF",
                "WHILE",
                "WEND",
                "DO",
                "LOOP",
                "HEAR",
                "TALK",
                "SET CONTEXT",
                "SET USER",
            ];

            let is_basic_command = basic_commands.iter().any(|&cmd| trimmed.starts_with(cmd));
            let is_control_flow = trimmed.starts_with("IF")
                || trimmed.starts_with("ELSE")
                || trimmed.starts_with("END IF");

            if is_basic_command || !for_stack.is_empty() || is_control_flow {
                result.push_str(trimmed);
                result.push(';');
            } else {
                result.push_str(trimmed);
                if !trimmed.ends_with(';') && !trimmed.ends_with('{') && !trimmed.ends_with('}') {
                    result.push(';');
                }
            }
            result.push('\n');
        }

        if !for_stack.is_empty() {
            panic!("Unclosed FOR EACH loop");
        }

        result
    }

    pub fn compile(&self, script: &str) -> Result<rhai::AST, Box<EvalAltResult>> {
        let processed_script = self.preprocess_basic_script(script);
        info!("Processed Script:\n{}", processed_script);
        match self.engine.compile(&processed_script) {
            Ok(ast) => Ok(ast),
            Err(parse_error) => Err(Box::new(parse_error.into())),
        }
    }

    pub fn run(&self, ast: &rhai::AST) -> Result<Dynamic, Box<EvalAltResult>> {
        self.engine.eval_ast(ast)
    }
}
