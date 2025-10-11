use log::info;
use rhai::Dynamic;
use rhai::Engine;

use crate::shared::state::AppState;
use crate::shared::models::UserSession;

pub fn print_keyword(_state: &AppState, _user: UserSession, engine: &mut Engine) {
    engine
        .register_custom_syntax(
            &["PRINT", "$expr$"],
            true,
            |context, inputs| {
                let value = context.eval_expression_tree(&inputs[0])?;
                info!("{}", value);
                Ok(Dynamic::UNIT)
            },
        )
        .unwrap();
}
