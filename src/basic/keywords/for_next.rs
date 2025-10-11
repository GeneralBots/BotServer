use crate::shared::models::UserSession;
use crate::shared::state::AppState;
use log::info;
use rhai::Dynamic;
use rhai::Engine;

pub fn for_keyword(_state: &AppState, _user: UserSession, engine: &mut Engine) {
    engine
        .register_custom_syntax(&["EXIT", "FOR"], false, |_context, _inputs| {
            Err("EXIT FOR".into())
        })
        .unwrap();

    engine
        .register_custom_syntax(
            &[
                "FOR", "EACH", "$ident$", "IN", "$expr$", "$block$", "NEXT", "$ident$",
            ],
            true,
            |context, inputs| {
                let loop_var = inputs[0].get_string_value().unwrap();
                let next_var = inputs[3].get_string_value().unwrap();

                if loop_var != next_var {
                    return Err(format!(
                        "NEXT variable '{}' doesn't match FOR EACH variable '{}'",
                        next_var, loop_var
                    )
                    .into());
                }

                let collection = context.eval_expression_tree(&inputs[1])?;

                info!("Collection type: {}", collection.type_name());
                let ccc = collection.clone();
                let array = match collection.into_array() {
                    Ok(arr) => arr,
                    Err(err) => {
                        return Err(format!(
                            "foreach expected array, got {}: {}",
                            ccc.type_name(),
                            err
                        )
                        .into());
                    }
                };
                let block = &inputs[2];

                let orig_len = context.scope().len();

                for item in array {
                    context.scope_mut().push(loop_var, item);

                    match context.eval_expression_tree(block) {
                        Ok(_) => (),
                        Err(e) if e.to_string() == "EXIT FOR" => {
                            context.scope_mut().rewind(orig_len);
                            break;
                        }
                        Err(e) => {
                            context.scope_mut().rewind(orig_len);
                            return Err(e);
                        }
                    }

                    context.scope_mut().rewind(orig_len);
                }

                Ok(Dynamic::UNIT)
            },
        )
        .unwrap();
}
