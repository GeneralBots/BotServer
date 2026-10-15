use botbasic_types::BasicRuntime;
use botkyc::KycRequest;
use botbasic_types::UserSession;
use log::trace;
use rhai::{Dynamic, Engine};
use std::sync::Arc;

/// KYC and identity BASIC keywords for issue #622.
///
/// Provides: VERIFY FACE, VALIDATE DOCUMENT, CAPTURE SIGNATURE, START KYC.
pub fn register_kyc_keywords(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) {
    register_verify_face(state.clone(), user.clone(), engine);
    register_validate_document(state.clone(), user.clone(), engine);
    register_capture_signature(state.clone(), user.clone(), engine);
    register_start_kyc(state.clone(), user.clone(), engine);
}

fn register_verify_face(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["VERIFY", "FACE", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let profile_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let selfie_url = context.eval_expression_tree(&inputs[1])?.to_string();
                trace!("VERIFY FACE: profile={profile_id} selfie={selfie_url}");
                let request = KycRequest::face(profile_id, selfie_url);
                Ok(serde_json_to_dynamic(&request))
            },
        )
    {
        log::error!("valid VERIFY FACE syntax failed: {e}");
    }
}

fn register_validate_document(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["VALIDATE", "DOCUMENT", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let profile_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let document_url = context.eval_expression_tree(&inputs[1])?.to_string();
                trace!("VALIDATE DOCUMENT: profile={profile_id} doc={document_url}");
                let request = KycRequest::document(profile_id, document_url);
                Ok(serde_json_to_dynamic(&request))
            },
        )
    {
        log::error!("valid VALIDATE DOCUMENT syntax failed: {e}");
    }
}

fn register_capture_signature(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["CAPTURE", "SIGNATURE", "$expr$", ",", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let profile_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let document_id = context.eval_expression_tree(&inputs[1])?.to_string();
                let signature_data = context.eval_expression_tree(&inputs[2])?.to_string();
                trace!("CAPTURE SIGNATURE: profile={profile_id} doc={document_id}");
                let request = KycRequest::signature(profile_id, document_id, signature_data);
                Ok(serde_json_to_dynamic(&request))
            },
        )
    {
        log::error!("valid CAPTURE SIGNATURE syntax failed: {e}");
    }
}

fn register_start_kyc(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["START", "KYC", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let profile_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let workflow_name = context.eval_expression_tree(&inputs[1])?.to_string();
                trace!("START KYC: profile={profile_id} workflow={workflow_name}");
                let request = KycRequest::workflow(profile_id, workflow_name);
                Ok(serde_json_to_dynamic(&request))
            },
        )
    {
        log::error!("valid START KYC syntax failed: {e}");
    }
}

fn serde_json_to_dynamic<T: serde::Serialize>(value: &T) -> Dynamic {
    match serde_json::to_value(value) {
        Ok(json) => Dynamic::from(json.to_string()),
        Err(e) => {
            log::error!("KYC request serialization failed: {e}");
            Dynamic::UNIT
        }
    }
}
