use crate::shared::state::AppState;
use log::info;
use rhai::{Dynamic, Engine};
use std::thread;
use std::time::Duration;

pub fn wait_keyword(_state: &AppState, engine: &mut Engine) {
    engine
        .register_custom_syntax(
            &["WAIT", "$expr$"],
            false, // Expression, not statement
            move |context, inputs| {
                let seconds = context.eval_expression_tree(&inputs[0])?;

                // Convert to number (handle both int and float)
                let duration_secs = if seconds.is::<i64>() {
                    seconds.cast::<i64>() as f64
                } else if seconds.is::<f64>() {
                    seconds.cast::<f64>()
                } else {
                    return Err(format!("WAIT expects a number, got: {}", seconds).into());
                };

                if duration_secs < 0.0 {
                    return Err("WAIT duration cannot be negative".into());
                }

                // Cap maximum wait time to prevent abuse (e.g., 5 minutes max)
                let capped_duration = if duration_secs > 300.0 {
                    300.0
                } else {
                    duration_secs
                };

                info!("WAIT {} seconds (thread sleep)", capped_duration);

                // Use thread::sleep to block only the current thread, not the entire server
                let duration = Duration::from_secs_f64(capped_duration);
                thread::sleep(duration);

                info!("WAIT completed after {} seconds", capped_duration);
                Ok(Dynamic::from(format!("Waited {} seconds", capped_duration)))
            },
        )
        .unwrap();
}
