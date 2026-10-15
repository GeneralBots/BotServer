use botbasic_types::BasicRuntime;
use bottimeclock::TimeClockRequest;
use botbasic_types::UserSession;
use log::trace;
use rhai::{Dynamic, Engine};

use std::sync::Arc;

/// HR time clock BASIC keywords for issue #623.
///
/// Provides: CLOCK IN, CLOCK OUT, GET BANCO HORAS, FERIAS BALANCE, BANCO HORAS REPORT.
pub fn register_time_clock_keywords(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) {
    register_clock_in(state.clone(), user.clone(), engine);
    register_clock_out(state.clone(), user.clone(), engine);
    register_get_banco_horas(state.clone(), user.clone(), engine);
    register_ferias_balance(state.clone(), user.clone(), engine);
    register_banco_horas_report(state, user, engine);
}

fn register_clock_in(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(["CLOCK", "IN"], false, move |_context, _inputs| {
            trace!("CLOCK IN");
            Ok(serde_json_to_dynamic(&TimeClockRequest::clock_in()))
        })
    {
        log::error!("valid CLOCK IN syntax failed: {e}");
    }
}

fn register_clock_out(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(["CLOCK", "OUT"], false, move |_context, _inputs| {
            trace!("CLOCK OUT");
            Ok(serde_json_to_dynamic(&TimeClockRequest::clock_out()))
        })
    {
        log::error!("valid CLOCK OUT syntax failed: {e}");
    }
}

fn register_get_banco_horas(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["GET", "BANCO", "HORAS", "$expr$"],
            false,
            move |context, inputs| {
                let person_id = context.eval_expression_tree(&inputs[0])?.to_string();
                trace!("GET BANCO HORAS: {person_id}");
                Ok(serde_json_to_dynamic(&TimeClockRequest::banco_horas(person_id)))
            },
        )
    {
        log::error!("valid GET BANCO HORAS syntax failed: {e}");
    }
}

fn register_ferias_balance(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["FERIAS", "BALANCE", "$expr$"],
            false,
            move |context, inputs| {
                let person_id = context.eval_expression_tree(&inputs[0])?.to_string();
                trace!("FERIAS BALANCE: {person_id}");
                Ok(serde_json_to_dynamic(&TimeClockRequest::ferias_balance(person_id)))
            },
        )
    {
        log::error!("valid FERIAS BALANCE syntax failed: {e}");
    }
}

fn register_banco_horas_report(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["BANCO", "HORAS", "REPORT", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let person_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let period = context.eval_expression_tree(&inputs[1])?.to_string();
                trace!("BANCO HORAS REPORT: {person_id} period={period}");
                Ok(serde_json_to_dynamic(&TimeClockRequest::banco_horas_report(
                    person_id, period,
                )))
            },
        )
    {
        log::error!("valid BANCO HORAS REPORT syntax failed: {e}");
    }
}

fn serde_json_to_dynamic<T: serde::Serialize>(value: &T) -> Dynamic {
    match serde_json::to_value(value) {
        Ok(json) => Dynamic::from(json.to_string()),
        Err(e) => {
            log::error!("time-clock request serialization failed: {e}");
            Dynamic::UNIT
        }
    }
}
