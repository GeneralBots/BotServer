use super::*;

pub fn register_multimodal_keywords(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) {
    register_generate_image(state.clone(), user.clone(), engine);
    register_describe_image(state.clone(), user.clone(), engine);
    register_read_text(state.clone(), user.clone(), engine);
    register_scan_barcode(state.clone(), user.clone(), engine);
    register_detect_objects(state.clone(), user.clone(), engine);
    register_read_plate(state.clone(), user.clone(), engine);
    register_detect_damage(state.clone(), user.clone(), engine);
    register_generate_video(state.clone(), user.clone(), engine);
    register_speech_to_text(state.clone(), user.clone(), engine);
    register_text_to_speech(state.clone(), user.clone(), engine);
    register_compare_images(state.clone(), user.clone(), engine);
    register_classify_image(state.clone(), user.clone(), engine);
    register_detect_defects(state.clone(), user.clone(), engine);
    register_detect_faces(state.clone(), user.clone(), engine);
    register_extract_colors(state.clone(), user.clone(), engine);
    register_assess_image(state.clone(), user.clone(), engine);
    register_analyze_image(state.clone(), user.clone(), engine);
}
pub(super) fn register_generate_image(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine.register_custom_syntax(
        ["GENERATE", "IMAGE", "$expr$"],
        false,
        move |context, inputs| {
            let prompt = eval_string(context, &inputs[0])?;
            let runtime = Arc::clone(&state);
            let bot_id = user.bot_id;
            spawn_multimodal("generate-image", async move {
                let client = build_client(runtime.as_ref(), bot_id);
                if !client.is_enabled() {
                    return Err("BotModels is not enabled in bot configuration".into());
                }
                client.generate_image(&prompt).await
            })
        },
    ) {
        log::error!("GENERATE IMAGE registration failed: {e}");
    }
}
pub(super) fn register_describe_image(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    register_describe_image_fn(state.clone(), user.clone(), engine);
    if let Err(e) = engine.register_custom_syntax(
        ["DESCRIBE", "IMAGE", "$expr$"],
        false,
        move |context, inputs| {
            let source = eval_string(context, &inputs[0])?;
            let runtime = Arc::clone(&state);
            let bot_id = user.bot_id;
            spawn_multimodal("describe-image", async move {
                let client = build_client(runtime.as_ref(), bot_id);
                if !client.is_enabled() {
                    return Err("BotModels is not enabled in bot configuration".into());
                }
                let media = resolve_media_source(runtime.as_ref(), bot_id, &source).await?;
                let outcome = client.describe_image(media.reference()).await;
                media.cleanup();
                outcome
            })
        },
    ) {
        log::error!("DESCRIBE IMAGE registration failed: {e}");
    }
}
/// Function form `describe_image(source)`, emitted by
/// `convert_multiword_keywords`. The custom-syntax form is unreliable at
/// runtime because DESCRIBE VIDEO (and other DESCRIBE … forms) share the same
/// Rhai first-token key and only one registration survives.
pub(super) fn register_describe_image_fn(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) {
    engine.register_fn("describe_image", move |source: &str| -> Result<rhai::Dynamic, Box<EvalAltResult>> {
        let runtime = Arc::clone(&state);
        let bot_id = user.bot_id;
        let source = source.to_string();
        spawn_multimodal("describe-image", async move {
            let client = build_client(runtime.as_ref(), bot_id);
            if !client.is_enabled() {
                return Err("BotModels is not enabled in bot configuration".into());
            }
            let media = resolve_media_source(runtime.as_ref(), bot_id, &source).await?;
            let outcome = client.describe_image(media.reference()).await;
            media.cleanup();
            outcome
        })
    });
}
pub(super) fn register_read_text(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    if let Err(e) =
        engine.register_custom_syntax(["READ", "TEXT", "$expr$"], false, move |context, inputs| {
            let source = eval_string(context, &inputs[0])?;
            let runtime = Arc::clone(&state);
            let bot_id = user.bot_id;
            spawn_multimodal("read-text", async move {
                let client = build_client(runtime.as_ref(), bot_id);
                if !client.is_enabled() {
                    return Err("BotModels is not enabled in bot configuration".into());
                }
                let media = resolve_media_source(runtime.as_ref(), bot_id, &source).await?;
                let outcome = client.describe_image(media.reference()).await;
                media.cleanup();
                outcome
            })
        })
    {
        log::error!("READ TEXT registration failed: {e}");
    }
}
pub(super) fn register_scan_barcode(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine.register_custom_syntax(
        ["SCAN", "BARCODE", "$expr$"],
        false,
        move |context, inputs| {
            let source = eval_string(context, &inputs[0])?;
            let runtime = Arc::clone(&state);
            let bot_id = user.bot_id;
            spawn_multimodal("scan-barcode", async move {
                let client = build_client(runtime.as_ref(), bot_id);
                if !client.is_enabled() {
                    return Err("BotModels is not enabled in bot configuration".into());
                }
                let media = resolve_media_source(runtime.as_ref(), bot_id, &source).await?;
                let outcome = client.scan_barcode(media.reference()).await;
                media.cleanup();
                outcome
            })
        },
    ) {
        log::error!("SCAN BARCODE registration failed: {e}");
    }
}
pub(super) fn register_detect_objects(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine.register_custom_syntax(
        ["DETECT", "OBJECTS", "$expr$"],
        false,
        move |context, inputs| {
            let source = eval_string(context, &inputs[0])?;
            let runtime = Arc::clone(&state);
            let bot_id = user.bot_id;
            spawn_multimodal("detect-objects", async move {
                let client = build_client(runtime.as_ref(), bot_id);
                if !client.is_enabled() {
                    return Err("BotModels is not enabled in bot configuration".into());
                }
                let media = resolve_media_source(runtime.as_ref(), bot_id, &source).await?;
                let outcome = client.describe_image(media.reference()).await;
                media.cleanup();
                outcome
            })
        },
    ) {
        log::error!("DETECT OBJECTS registration failed: {e}");
    }
}
pub(super) fn register_read_plate(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine.register_custom_syntax(
        ["READ", "PLATE", "$expr$"],
        false,
        move |context, inputs| {
            let source = eval_string(context, &inputs[0])?;
            let runtime = Arc::clone(&state);
            let bot_id = user.bot_id;
            spawn_multimodal("read-plate", async move {
                let client = build_client(runtime.as_ref(), bot_id);
                if !client.is_enabled() {
                    return Err("BotModels is not enabled in bot configuration".into());
                }
                let media = resolve_media_source(runtime.as_ref(), bot_id, &source).await?;
                let raw = client.scan_barcode(media.reference()).await?;
                media.cleanup();
                Ok(format!("plate-scan:{raw}"))
            })
        },
    ) {
        log::error!("READ PLATE registration failed: {e}");
    }
}
pub(super) fn register_detect_damage(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine.register_custom_syntax(
        ["DETECT", "DAMAGE", "$expr$"],
        false,
        move |context, inputs| {
            let source = eval_string(context, &inputs[0])?;
            let runtime = Arc::clone(&state);
            let bot_id = user.bot_id;
            spawn_multimodal("detect-damage", async move {
                let client = build_client(runtime.as_ref(), bot_id);
                if !client.is_enabled() {
                    return Err("BotModels is not enabled in bot configuration".into());
                }
                let media = resolve_media_source(runtime.as_ref(), bot_id, &source).await?;
                let description = client.describe_image(media.reference()).await?;
                media.cleanup();
                Ok(format!("damage-assessment:{description}"))
            })
        },
    ) {
        log::error!("DETECT DAMAGE registration failed: {e}");
    }
}
pub(super) fn register_generate_video(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine.register_custom_syntax(
        ["GENERATE", "VIDEO", "$expr$"],
        false,
        move |context, inputs| {
            let prompt = eval_string(context, &inputs[0])?;
            let runtime = Arc::clone(&state);
            let bot_id = user.bot_id;
            spawn_multimodal("generate-video", async move {
                let client = build_client(runtime.as_ref(), bot_id);
                if !client.is_enabled() {
                    return Err("BotModels is not enabled in bot configuration".into());
                }
                client.generate_video(&prompt).await
            })
        },
    ) {
        log::error!("GENERATE VIDEO registration failed: {e}");
    }
}
pub(super) fn register_speech_to_text_fn(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    engine.register_fn("speech_to_text", move |source: &str| -> Result<rhai::Dynamic, Box<EvalAltResult>> {
        let runtime = Arc::clone(&state);
        let bot_id = user.bot_id;
        let source = source.to_string();
        spawn_multimodal("speech-to-text", async move {
            let client = build_client(runtime.as_ref(), bot_id);
            if !client.is_enabled() {
                return Err("BotModels is not enabled in bot configuration".into());
            }
            let media = resolve_media_source(runtime.as_ref(), bot_id, &source).await?;
            let outcome = client.speech_to_text(media.reference()).await;
            media.cleanup();
            outcome
        })
    });
}
pub(super) fn register_speech_to_text(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    register_speech_to_text_fn(state.clone(), user.clone(), engine);
    if let Err(e) = engine.register_custom_syntax(
        ["SPEECH", "TO", "TEXT", "$expr$"],
        false,
        move |context, inputs| {
            let source = eval_string(context, &inputs[0])?;
            let runtime = Arc::clone(&state);
            let bot_id = user.bot_id;
            spawn_multimodal("speech-to-text", async move {
                let client = build_client(runtime.as_ref(), bot_id);
                if !client.is_enabled() {
                    return Err("BotModels is not enabled in bot configuration".into());
                }
                let media = resolve_media_source(runtime.as_ref(), bot_id, &source).await?;
                let outcome = client.speech_to_text(media.reference()).await;
                media.cleanup();
                outcome
            })
        },
    ) {
        log::error!("SPEECH TO TEXT registration failed: {e}");
    }
}
pub(super) fn register_text_to_speech(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine.register_custom_syntax(
        ["TEXT", "TO", "SPEECH", "$expr$"],
        false,
        move |context, inputs| {
            let text = eval_string(context, &inputs[0])?;
            let runtime = Arc::clone(&state);
            let bot_id = user.bot_id;
            spawn_multimodal("text-to-speech", async move {
                let client = build_client(runtime.as_ref(), bot_id);
                if !client.is_enabled() {
                    return Err("BotModels is not enabled in bot configuration".into());
                }
                client.generate_audio(&text, None, Some("pt-BR")).await
            })
        },
    ) {
        log::error!("TEXT TO SPEECH registration failed: {e}");
    }
}
