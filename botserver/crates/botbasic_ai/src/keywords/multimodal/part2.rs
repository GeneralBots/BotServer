use super::*;

pub(super) fn register_analyze_image(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine.register_custom_syntax(
        ["ANALYZE", "IMAGE", "$expr$"],
        false,
        move |context, inputs| {
            let source = eval_string(context, &inputs[0])?;
            let runtime = Arc::clone(&state);
            let bot_id = user.bot_id;
            spawn_multimodal("analyze-image", async move {
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
        log::error!("ANALYZE IMAGE registration failed: {e}");
    }
}
pub(super) fn register_compare_images(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine.register_custom_syntax(
        ["COMPARE", "IMAGES", "$expr$", "WITH", "$expr$"],
        false,
        move |context, inputs| {
            let a = eval_string(context, &inputs[0])?;
            let b = eval_string(context, &inputs[1])?;
            let combined = format!("{a}|{b}");
            let runtime = Arc::clone(&state);
            let bot_id = user.bot_id;
            spawn_multimodal("compare-images", async move {
                let client = build_client(runtime.as_ref(), bot_id);
                if !client.is_enabled() {
                    return Err("BotModels is not enabled in bot configuration".into());
                }
                client.describe_image(&combined).await
            })
        },
    ) {
        log::error!("COMPARE IMAGES registration failed: {e}");
    }
}
pub(super) fn register_classify_image(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine.register_custom_syntax(
        ["CLASSIFY", "IMAGE", "$expr$"],
        false,
        move |context, inputs| {
            let source = eval_string(context, &inputs[0])?;
            let runtime = Arc::clone(&state);
            let bot_id = user.bot_id;
            spawn_multimodal("classify-image", async move {
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
        log::error!("CLASSIFY IMAGE registration failed: {e}");
    }
}
pub(super) fn register_detect_defects(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine.register_custom_syntax(
        ["DETECT", "DEFECTS", "$expr$"],
        false,
        move |context, inputs| {
            let source = eval_string(context, &inputs[0])?;
            let runtime = Arc::clone(&state);
            let bot_id = user.bot_id;
            spawn_multimodal("detect-defects", async move {
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
        log::error!("DETECT DEFECTS registration failed: {e}");
    }
}
pub(super) fn register_detect_faces(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine.register_custom_syntax(
        ["DETECT", "FACES", "$expr$"],
        false,
        move |context, inputs| {
            let source = eval_string(context, &inputs[0])?;
            let runtime = Arc::clone(&state);
            let bot_id = user.bot_id;
            spawn_multimodal("detect-faces", async move {
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
        log::error!("DETECT FACES registration failed: {e}");
    }
}
pub(super) fn register_extract_colors(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine.register_custom_syntax(
        ["EXTRACT", "COLORS", "$expr$"],
        false,
        move |context, inputs| {
            let source = eval_string(context, &inputs[0])?;
            let runtime = Arc::clone(&state);
            let bot_id = user.bot_id;
            spawn_multimodal("extract-colors", async move {
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
        log::error!("EXTRACT COLORS registration failed: {e}");
    }
}
pub(super) fn register_assess_image(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine.register_custom_syntax(
        ["ASSESS", "IMAGE", "$expr$"],
        false,
        move |context, inputs| {
            let source = eval_string(context, &inputs[0])?;
            let runtime = Arc::clone(&state);
            let bot_id = user.bot_id;
            spawn_multimodal("assess-image", async move {
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
        log::error!("ASSESS IMAGE registration failed: {e}");
    }
}
