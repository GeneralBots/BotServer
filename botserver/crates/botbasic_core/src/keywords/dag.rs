use botbasic_types::UserSession;
use botbasic_types::BasicRuntime;
use rhai::{Dynamic, Engine, EvalAltResult, Scope};
use std::sync::Arc;

pub fn register_dag_keywords(
    _state: &Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    if let Err(e) = engine
        .register_custom_syntax(
            ["IF", "$expr$", "THEN", "$block$", "ELSE", "$block$", "END", "IF"],
            true,
            |context, inputs| {
                let cond = context
                    .eval_expression_tree(&inputs[0])?
                    .as_bool()
                    .map_err(|e| EvalAltResult::ErrorRuntime(e.to_string().into(), rhai::Position::NONE))?;
                if cond {
                    context.eval_expression_tree(&inputs[1])
                } else {
                    context.eval_expression_tree(&inputs[2])
                }
            },
        )
    {
        log::error!("Failed to register IF/THEN/ELSE: {e}");
    }

    if let Err(e) = engine
        .register_custom_syntax(
            ["PARALLEL", "$block$", "AND", "$block$", "END", "PARALLEL"],
            true,
            |_context, inputs| {
                let mut handles = Vec::new();
                // One worker per parallel branch keeps the statement arity intact;
                // the branches themselves are not evaluated yet.
                for _ in inputs {
                    handles.push(std::thread::spawn(|| -> Result<Dynamic, Box<EvalAltResult>> {
                        Ok(Dynamic::UNIT)
                    }));
                }
                let mut results: Vec<Dynamic> = Vec::new();
                for h in handles {
                    match h.join() {
                        Ok(Ok(dyn_val)) => results.push(dyn_val),
                        _ => {
                            return Err(EvalAltResult::ErrorRuntime(
                                "Parallel thread panic".into(),
                                rhai::Position::NONE,
                            )
                            .into());
                        }
                    }
                }
                Ok(Dynamic::from(results))
            },
        )
    {
        log::error!("Failed to register PARALLEL/AND: {e}");
    }

    if let Err(e) = engine
        .register_custom_syntax(
            ["ON", "ERROR", "$block$"],
            true,
            |context, inputs| {
                let handler = inputs[0].clone();
                match context.eval_expression_tree(&handler) {
                    Ok(r) => Ok(r),
                    Err(e) => {
                        let mut error_scope = Scope::new();
                        let _ = error_scope.push_dynamic("error", Dynamic::from(e.to_string()));
                        let _ = context.eval_expression_tree(&handler)
                            .map_err(|_| -> Box<EvalAltResult> {
                                Box::new(EvalAltResult::ErrorRuntime(
                                    "ON ERROR handler re-raised".into(),
                                    rhai::Position::NONE,
                                ))
                            })?;
                        Ok(Dynamic::UNIT)
                    }
                }
            },
        )
    {
        log::error!("Failed to register ON ERROR: {e}");
    }
}
