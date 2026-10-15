use botbanking::BankingRequest;
use botbasic_types::BasicRuntime;
use botbasic_types::UserSession;
use log::trace;
use rhai::{Dynamic, Engine};
use std::sync::Arc;

/// Banking and reconciliation BASIC keywords for issue #618.
///
/// Provides: IMPORT BANK STATEMENT, IMPORT PLATFORM ORDERS, RECONCILE,
/// GET UNMATCHED DELIVERIES, ADD RECONCILE RULE, DELIVERY MARGINS,
/// REPORT REVENUE BY PLATFORM.
///
/// CSV parsing, reconciliation matching and the aggregations are built by
/// `botbanking::intent` (#1506) — this module only binds arguments.
pub fn register_banking_keywords(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) {
    register_import_bank_statement(state.clone(), user.clone(), engine);
    register_import_platform_orders(state.clone(), user.clone(), engine);
    register_reconcile(state.clone(), user.clone(), engine);
    register_get_unmatched_deliveries(state.clone(), user.clone(), engine);
    register_add_reconcile_rule(state.clone(), user.clone(), engine);
    register_delivery_margins(state.clone(), user.clone(), engine);
    register_revenue_by_platform(state, user, engine);
}

fn register_import_bank_statement(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine.register_custom_syntax(
        ["IMPORT", "BANK", "STATEMENT", "$expr$"],
        false,
        move |context, inputs| {
            let csv = context.eval_expression_tree(&inputs[0])?.to_string();
            trace!("IMPORT BANK STATEMENT ({} bytes)", csv.len());
            Ok(to_dynamic(&BankingRequest::import_bank_statement(&csv)))
        },
    ) {
        log::error!("valid IMPORT BANK STATEMENT syntax failed: {e}");
    }
}

fn register_import_platform_orders(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine.register_custom_syntax(
        ["IMPORT", "PLATFORM", "ORDERS", "$expr$"],
        false,
        move |context, inputs| {
            let csv = context.eval_expression_tree(&inputs[0])?.to_string();
            trace!("IMPORT PLATFORM ORDERS ({} bytes)", csv.len());
            Ok(to_dynamic(&BankingRequest::import_platform_orders(&csv)))
        },
    ) {
        log::error!("valid IMPORT PLATFORM ORDERS syntax failed: {e}");
    }
}

fn register_reconcile(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine.register_custom_syntax(
        ["RECONCILE", "$expr$", "$expr$"],
        false,
        move |context, inputs| {
            let deliveries_json = context.eval_expression_tree(&inputs[0])?.to_string();
            let bank_json = context.eval_expression_tree(&inputs[1])?.to_string();
            trace!("RECONCILE: matching deliveries to bank txns");
            Ok(to_dynamic(&BankingRequest::reconcile(
                &deliveries_json,
                &bank_json,
            )))
        },
    ) {
        log::error!("valid RECONCILE syntax failed: {e}");
    }
}

fn register_get_unmatched_deliveries(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine.register_custom_syntax(
        ["GET", "UNMATCHED", "DELIVERIES", "$expr$"],
        false,
        move |context, inputs| {
            let reconciled_ids_json = context.eval_expression_tree(&inputs[0])?.to_string();
            trace!("GET UNMATCHED DELIVERIES");
            Ok(to_dynamic(&BankingRequest::unmatched_deliveries(
                &reconciled_ids_json,
            )))
        },
    ) {
        log::error!("valid GET UNMATCHED DELIVERIES syntax failed: {e}");
    }
}

fn register_add_reconcile_rule(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine.register_custom_syntax(
        ["ADD", "RECONCILE", "RULE", "$expr$", ",", "$expr$", ",", "$expr$"],
        false,
        move |context, inputs| {
            let name = context.eval_expression_tree(&inputs[0])?.to_string();
            let match_field = context.eval_expression_tree(&inputs[1])?.to_string();
            let match_value = context.eval_expression_tree(&inputs[2])?.to_string();
            trace!("ADD RECONCILE RULE: {name} {match_field}={match_value}");
            Ok(to_dynamic(&BankingRequest::add_reconcile_rule(
                &name,
                &match_field,
                &match_value,
            )))
        },
    ) {
        log::error!("valid ADD RECONCILE RULE syntax failed: {e}");
    }
}

fn register_delivery_margins(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine.register_custom_syntax(
        ["DELIVERY", "MARGINS", "$expr$"],
        false,
        move |context, inputs| {
            let deliveries_json = context.eval_expression_tree(&inputs[0])?.to_string();
            trace!("DELIVERY MARGINS");
            Ok(to_dynamic(&BankingRequest::delivery_margins(
                &deliveries_json,
            )))
        },
    ) {
        log::error!("valid DELIVERY MARGINS syntax failed: {e}");
    }
}

fn register_revenue_by_platform(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine.register_custom_syntax(
        ["REPORT", "REVENUE", "BY", "PLATFORM", "$expr$"],
        false,
        move |context, inputs| {
            let deliveries_json = context.eval_expression_tree(&inputs[0])?.to_string();
            trace!("REPORT REVENUE BY PLATFORM");
            Ok(to_dynamic(&BankingRequest::revenue_by_platform(
                &deliveries_json,
            )))
        },
    ) {
        log::error!("valid REPORT REVENUE BY PLATFORM syntax failed: {e}");
    }
}

fn to_dynamic(request: &BankingRequest) -> Dynamic {
    match serde_json::to_value(request) {
        Ok(json) => Dynamic::from(json.to_string()),
        Err(e) => {
            log::error!("banking request serialization failed: {e}");
            Dynamic::UNIT
        }
    }
}
