use super::*;

pub fn register_procedure_keywords(state: &Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    register_while_wend(engine);
    register_do_loop(engine);
    register_call_keyword(state, user, engine);
    register_return_keyword(engine);
}
pub(super) fn register_while_wend(engine: &mut Engine) {
    if let Err(e) = engine
        .register_custom_syntax(
            ["WHILE", "$expr$", "$block$", "WEND"],
            true,
            |context, inputs| {
                let condition_expr = &inputs[0];
                let block = &inputs[1];

                let max_iterations = MAX_LOOP_ITERATIONS;
                let mut iterations = 0;

                loop {
                    let condition = context.eval_expression_tree(condition_expr)?;
                    let should_continue = match condition.as_bool() {
                        Ok(b) => b,
                        Err(_) => {
                            if let Ok(n) = condition.as_int() {
                                n != 0
                            } else if let Ok(f) = condition.as_float() {
                                f != 0.0
                            } else if let Ok(s) = condition.clone().into_string() {
                                !s.is_empty() && s.to_lowercase() != "false"
                            } else {
                                !condition.is_unit()
                            }
                        }
                    };

                    if !should_continue {
                        break;
                    }

                    match context.eval_expression_tree(block) {
                        Ok(_) => (),
                        Err(e) => {
                            let err_str = e.to_string();
                            if err_str == "EXIT WHILE" || err_str == "EXIT DO" {
                                break;
                            }
                            return Err(e);
                        }
                    }

                    iterations += 1;
                    if iterations >= max_iterations {
                        return Err(format!(
                            "WHILE loop exceeded maximum iterations ({max_iterations}). Possible infinite loop."
                        )
                        .into());
                    }
                }

                Ok(Dynamic::UNIT)
            },
        )
    {
        log::error!("Failed to register WHILE/WEND syntax: {e}");
    }

    if let Err(e) = engine
        .register_custom_syntax(["EXIT", "WHILE"], false, |_context, _inputs| {
            Err("EXIT WHILE".into())
        })
    {
        log::error!("Failed to register EXIT WHILE syntax: {e}");
    }
}
pub(super) fn register_do_loop(engine: &mut Engine) {
    if let Err(e) = engine
        .register_custom_syntax(
            ["DO", "WHILE", "$expr$", "$block$", "LOOP"],
            true,
            |context, inputs| {
                let condition_expr = &inputs[0];
                let block = &inputs[1];

                let max_iterations = MAX_LOOP_ITERATIONS;
                let mut iterations = 0;

                loop {
                    let condition = context.eval_expression_tree(condition_expr)?;
                    let should_continue = eval_bool_condition(&condition);

                    if !should_continue {
                        break;
                    }

                    match context.eval_expression_tree(block) {
                        Ok(_) => (),
                        Err(e) if e.to_string() == "EXIT DO" => break,
                        Err(e) => return Err(e),
                    }

                    iterations += 1;
                    if iterations >= max_iterations {
                        return Err("DO WHILE loop exceeded maximum iterations".into());
                    }
                }

                Ok(Dynamic::UNIT)
            },
        )
    {
        log::error!("Failed to register DO WHILE syntax: {e}");
    }

    if let Err(e) = engine
        .register_custom_syntax(
            ["DO", "UNTIL", "$expr$", "$block$", "LOOP"],
            true,
            |context, inputs| {
                let condition_expr = &inputs[0];
                let block = &inputs[1];

                let max_iterations = MAX_LOOP_ITERATIONS;
                let mut iterations = 0;

                loop {
                    let condition = context.eval_expression_tree(condition_expr)?;
                    let should_stop = eval_bool_condition(&condition);

                    if should_stop {
                        break;
                    }

                    match context.eval_expression_tree(block) {
                        Ok(_) => (),
                        Err(e) if e.to_string() == "EXIT DO" => break,
                        Err(e) => return Err(e),
                    }

                    iterations += 1;
                    if iterations >= max_iterations {
                        return Err("DO UNTIL loop exceeded maximum iterations".into());
                    }
                }

                Ok(Dynamic::UNIT)
            },
        )
    {
        log::error!("Failed to register DO UNTIL syntax: {e}");
    }

    if let Err(e) = engine
        .register_custom_syntax(
            ["DO", "$block$", "LOOP", "WHILE", "$expr$"],
            true,
            |context, inputs| {
                let block = &inputs[0];
                let condition_expr = &inputs[1];

                let max_iterations = MAX_LOOP_ITERATIONS;
                let mut iterations = 0;

                loop {
                    match context.eval_expression_tree(block) {
                        Ok(_) => (),
                        Err(e) if e.to_string() == "EXIT DO" => break,
                        Err(e) => return Err(e),
                    }

                    let condition = context.eval_expression_tree(condition_expr)?;
                    let should_continue = eval_bool_condition(&condition);

                    if !should_continue {
                        break;
                    }

                    iterations += 1;
                    if iterations >= max_iterations {
                        return Err("DO...LOOP WHILE exceeded maximum iterations".into());
                    }
                }

                Ok(Dynamic::UNIT)
            },
        )
    {
        log::error!("Failed to register DO...LOOP WHILE syntax: {e}");
    }

    if let Err(e) = engine
        .register_custom_syntax(
            ["DO", "$block$", "LOOP", "UNTIL", "$expr$"],
            true,
            |context, inputs| {
                let block = &inputs[0];
                let condition_expr = &inputs[1];

                let max_iterations = MAX_LOOP_ITERATIONS;
                let mut iterations = 0;

                loop {
                    match context.eval_expression_tree(block) {
                        Ok(_) => (),
                        Err(e) if e.to_string() == "EXIT DO" => break,
                        Err(e) => return Err(e),
                    }

                    let condition = context.eval_expression_tree(condition_expr)?;
                    let should_stop = eval_bool_condition(&condition);

                    if should_stop {
                        break;
                    }

                    iterations += 1;
                    if iterations >= max_iterations {
                        return Err("DO...LOOP UNTIL exceeded maximum iterations".into());
                    }
                }

                Ok(Dynamic::UNIT)
            },
        )
    {
        log::error!("Failed to register DO...LOOP UNTIL syntax: {e}");
    }

    if let Err(e) = engine
        .register_custom_syntax(["EXIT", "DO"], false, |_context, _inputs| {
            Err("EXIT DO".into())
        })
    {
        log::error!("Failed to register EXIT DO syntax: {e}");
    }
}
pub(super) fn eval_bool_condition(value: &Dynamic) -> bool {
    match value.as_bool() {
        Ok(b) => b,
        Err(_) => {
            if let Ok(n) = value.as_int() {
                n != 0
            } else if let Ok(f) = value.as_float() {
                f != 0.0
            } else if let Ok(s) = value.clone().into_string() {
                !s.is_empty() && s.to_lowercase() != "false" && s != "0"
            } else {
                !value.is_unit()
            }
        }
    }
}
