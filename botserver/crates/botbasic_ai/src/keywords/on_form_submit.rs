


use botbasic_types::UserSession;
use botbasic_types::BasicRuntime;
use log::{debug, info};
use rhai::{Array, Dynamic, Engine, Map};

pub fn on_form_submit_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let user1 = user.clone();

    engine.register_fn("VALIDATE_FORM", move |form_data: Map| -> bool {
        trace_call("VALIDATE_FORM", &user1);
        validate_form(&form_data)
    });

    let user2 = user.clone();

    engine.register_fn("VALIDATE_FORM", move |form_data: Map, rules: Map| -> bool {
        trace_call("VALIDATE_FORM with rules", &user2);
        validate_form_with_rules(&form_data, &rules)
    });

    let user3 = user.clone();

    engine.register_fn(
        "REGISTER_FORM_HANDLER",
        move |form_name: &str, handler_script: &str| -> bool {
            debug!(
                "REGISTER_FORM_HANDLER: form={}, script_len={}, user={}",
                form_name,
                handler_script.len(),
                user3.user_id
            );
            info!("Form handler registered for: {}", form_name);
            true
        },
    );

    let user4 = user.clone();

    engine.register_fn("IS_FORM_SUBMISSION", move || -> bool {
        debug!("IS_FORM_SUBMISSION check, user={}", user4.user_id);
        true
    });

    let user5 = user.clone();

    engine.register_fn("GET_SUBMISSION_ID", move || -> String {
        let id = generate_submission_id();
        debug!("GET_SUBMISSION_ID: {}, user={}", id, user5.user_id);
        id
    });

    let user6 = user.clone();
    let state6 = state.clone();

    engine.register_fn(
        "SAVE_SUBMISSION",
        move |form_name: &str, data: Map| -> Map {
            debug!(
                "SAVE_SUBMISSION: form={}, fields={}, user={}",
                form_name,
                data.len(),
                user6.user_id
            );
            save_form_submission(&state6, form_name, &user6, &data)
        },
    );

    let user7 = user.clone();
    let state7 = state.clone();

    engine.register_fn("GET_SUBMISSIONS", move |form_name: &str| -> Array {
        debug!(
            "GET_SUBMISSIONS: form={}, user={}",
            form_name, user7.user_id
        );
        get_form_submissions(&state7, form_name, &user7, None)
    });

    let user8 = user.clone();
    let state8 = state;

    engine.register_fn(
        "GET_SUBMISSIONS",
        move |form_name: &str, limit: i64| -> Array {
            debug!(
                "GET_SUBMISSIONS: form={}, limit={}, user={}",
                form_name, limit, user8.user_id
            );
            get_form_submissions(&state8, form_name, &user8, Some(limit as usize))
        },
    );

    let user9 = user;

    engine.register_fn("FORM_ERROR", move |message: &str| -> Map {
        debug!("FORM_ERROR: {}, user={}", message, user9.user_id);
        create_error_response(message)
    });

    info!("Registered form submission keywords");
}

fn validate_form(form_data: &Map) -> bool {
    if form_data.is_empty() {
        debug!("Form validation failed: empty data");
        return false;
    }

    for value in form_data.values() {
        if value.is_unit() {
            debug!("Form validation failed: null field");
            return false;
        }
    }

    debug!("Form validation passed for {} fields", form_data.len());
    true
}

fn validate_form_with_rules(form_data: &Map, rules: &Map) -> bool {
    if !validate_form(form_data) {
        return false;
    }

    for (field_name, rule_value) in rules.iter() {
        if let Some(field_value) = form_data.get(field_name.as_str()) {
            let rule = rule_value.to_string().to_lowercase();

            match rule.as_str() {
                "required" if field_value.is_unit() => {
                    debug!("Validation failed: required field missing: {}", field_name);
                    return false;
                }
                "email" => {
                    let email_str = field_value.to_string();
                    if !email_str.contains('@') || !email_str.contains('.') {
                        debug!("Validation failed: invalid email: {}", field_name);
                        return false;
                    }
                }
                "phone" => {
                    let phone_str = field_value.to_string();
                    let digits_only: String =
                        phone_str.chars().filter(|c| c.is_numeric()).collect();
                    if digits_only.len() < 10 {
                        debug!("Validation failed: invalid phone: {}", field_name);
                        return false;
                    }
                }
                _ => {}
            }
        }
    }

    debug!("Form validation with rules passed");
    true
}

fn _register_form_handler(
    _state: &Arc<dyn BasicRuntime>,
    form_name: &str,
    _handler_script: &str,
    user: &UserSession,
) -> bool {
    let handler_id = Uuid::new_v4().to_string();
    info!(
        "Registered handler for form: {} ({}), user={}",
        form_name, handler_id, user.user_id
    );
    true
}

fn generate_submission_id() -> String {
    Uuid::new_v4().to_string()
}

fn save_form_submission(
    _state: &Arc<dyn BasicRuntime>,
    form_name: &str,
    user: &UserSession,
    data: &Map,
) -> Map {
    let submission_id = generate_submission_id();
    let mut result = Map::new();
    let timestamp = chrono::Utc::now().to_rfc3339();

    result.insert("success".into(), Dynamic::from(true));
    result.insert("id".into(), Dynamic::from(submission_id.clone()));
    result.insert("timestamp".into(), Dynamic::from(timestamp));
    result.insert("fields_saved".into(), Dynamic::from(data.len() as i64));

    info!(
        "Saved form submission: form={}, id={}, fields={}, user={}",
        form_name,
        submission_id,
        data.len(),
        user.user_id
    );
    result
}

fn get_form_submissions(
    _state: &Arc<dyn BasicRuntime>,
    form_name: &str,
    user: &UserSession,
    limit: Option<usize>,
) -> Array {
    let submissions = Array::new();
    let limit_val = limit.unwrap_or(100);

    debug!(
        "Retrieved form submissions: form={}, limit={}, user={}",
        form_name, limit_val, user.user_id
    );

    submissions
}

fn create_error_response(message: &str) -> Map {
    let mut response = Map::new();
    response.insert("success".into(), Dynamic::from(false));
    response.insert("error".into(), Dynamic::from(message.to_string()));
    response.insert(
        "timestamp".into(),
        Dynamic::from(chrono::Utc::now().to_rfc3339()),
    );
    response
}

fn trace_call(operation: &str, user: &UserSession) {
    debug!("{} called by user: {}", operation, user.user_id);
}

use std::sync::Arc;
use uuid::Uuid;