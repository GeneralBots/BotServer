use botbasic_types::BasicRuntime;
use botbasic_types::UserSession;
use botm365::M365Request;
use log::trace;
use rhai::{Dynamic, Engine};
use std::sync::Arc;

/// Microsoft 365 / SharePoint / Teams BASIC keywords for issue #624.
///
/// Provides: GET SHAREPOINT LISTS, CREATE SHAREPOINT ITEM, SEND TEAMS MESSAGE.
///
/// The Graph endpoints and payload shapes are built by `botm365::intent`
/// (#1506) — this module only binds arguments.
pub fn register_m365_keywords(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) {
    register_get_sharepoint_lists(state.clone(), user.clone(), engine);
    register_create_sharepoint_item(state.clone(), user.clone(), engine);
    register_send_teams_message(state, user, engine);
}

fn register_get_sharepoint_lists(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine.register_custom_syntax(
        ["GET", "SHAREPOINT", "LISTS", "$expr$"],
        false,
        move |context, inputs| {
            let site_id = context.eval_expression_tree(&inputs[0])?.to_string();
            trace!("GET SHAREPOINT LISTS: site={site_id}");
            Ok(to_dynamic(&M365Request::sharepoint_lists(&site_id)))
        },
    ) {
        log::error!("valid GET SHAREPOINT LISTS syntax failed: {e}");
    }
}

fn register_create_sharepoint_item(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine.register_custom_syntax(
        [
            "CREATE", "SHAREPOINT", "ITEM", "$expr$", ",", "$expr$", ",", "$expr$",
        ],
        false,
        move |context, inputs| {
            let site_id = context.eval_expression_tree(&inputs[0])?.to_string();
            let list_id = context.eval_expression_tree(&inputs[1])?.to_string();
            let fields_json = context.eval_expression_tree(&inputs[2])?.to_string();
            trace!("CREATE SHAREPOINT ITEM: site={site_id} list={list_id}");
            Ok(to_dynamic(&M365Request::sharepoint_item(
                &site_id,
                &list_id,
                &fields_json,
            )))
        },
    ) {
        log::error!("valid CREATE SHAREPOINT ITEM syntax failed: {e}");
    }
}

fn register_send_teams_message(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    engine.register_fn("send_teams_message", |chat_id: String, message: String| {
        trace!("send_teams_message: chat={chat_id}");
        to_dynamic(&M365Request::teams_message(&chat_id, &message))
    });
}

fn to_dynamic<T: serde::Serialize>(value: &T) -> Dynamic {
    match serde_json::to_value(value) {
        Ok(json) => Dynamic::from(json.to_string()),
        Err(e) => {
            log::error!("Microsoft 365 request serialization failed: {e}");
            Dynamic::UNIT
        }
    }
}
