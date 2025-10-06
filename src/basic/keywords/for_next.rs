use crate::shared::state::AppState;
use log::info;
use rhai::Dynamic;
use rhai::Engine;

pub fn for_keyword(_state: &AppState, engine: &mut Engine) {
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
            true, // We're modifying the scope by adding the loop variable
            |context, inputs| {
                // Get the iterator variable names
                let loop_var = inputs[0].get_string_value().unwrap();
                let next_var = inputs[3].get_string_value().unwrap();

                // Verify variable names match
                if loop_var != next_var {
                    return Err(format!(
                        "NEXT variable '{}' doesn't match FOR EACH variable '{}'",
                        next_var, loop_var
                    )
                    .into());
                }

                // Evaluate the collection expression
                let collection = context.eval_expression_tree(&inputs[1])?;

                // Debug: Print the collection type
                info!("Collection type: {}", collection.type_name());
                let ccc = collection.clone();
                // Convert to array - with proper error handling
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
                // Get the block as an expression tree
                let block = &inputs[2];

                // Remember original scope length
                let orig_len = context.scope().len();

                for item in array {
                    // Push the loop variable into the scope
                    context.scope_mut().push(loop_var, item);

                    // Evaluate the block with the current scope
                    match context.eval_expression_tree(block) {
                        Ok(_) => (),
                        Err(e) if e.to_string() == "EXIT FOR" => {
                            context.scope_mut().rewind(orig_len);
                            break;
                        }
                        Err(e) => {
                            // Rewind the scope before returning error
                            context.scope_mut().rewind(orig_len);
                            return Err(e);
                        }
                    }

                    // Remove the loop variable for next iteration
                    context.scope_mut().rewind(orig_len);
                }

                Ok(Dynamic::UNIT)
            },
        )
        .unwrap();
}
