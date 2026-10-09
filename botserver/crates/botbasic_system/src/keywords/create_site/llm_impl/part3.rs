use super::*;

/// Resolves the Drive bucket and key prefix for the calling bot.
///
/// The bot id arrives as a UUID string, so it is parsed back to a UUID and
/// resolved through the shared resolver: an org-hosted bot writes into the
/// org workspace bucket (`{slug}.gborg`), not into a `{bot}.gbai` bucket.
/// Failing to resolve is an error rather than a silent fallback to a bucket
/// nothing reads, which is how generated sites became unreachable.
#[cfg(all(not(feature = "vibe"), feature = "drive"))]
pub(super) fn resolve_site_location(
    pool: &botcore::shared::utils::DbPool,
    bot_id: &str,
) -> Result<botbasic_core::utils::BotDriveLocation, Box<dyn Error + Send + Sync>> {
    let bot_uuid = uuid::Uuid::parse_str(bot_id)
        .map_err(|e| format!("Invalid bot id '{bot_id}': {e}"))?;
    let mut conn = pool
        .get()
        .map_err(|e| format!("Failed to resolve Drive location for bot {bot_id}: {e}"))?;
    Ok(botbasic_core::utils::bot_drive_location_for(&mut conn, bot_uuid))
}
#[cfg(all(not(feature = "vibe"), feature = "drive"))]
pub(super) async fn store_to_drive(
    s3: Option<&Arc<dyn botlib::traits::DriveRepository>>,
    location: &botbasic_core::utils::BotDriveLocation,
    drive_path: &str,
    html_content: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(s3_client) = s3 else {
        debug!("S3 not configured, skipping drive storage");
        return Ok(());
    };
    let key = location.key_for(&format!("{drive_path}/index.html"));

    info!("Storing to drive: s3://{}/{key}", location.bucket);

    s3_client
        .put_object(&location.bucket, &key, html_content.as_bytes().to_vec(), None)
        .await
        .map_err(|e| format!("Failed to store to drive: {e}"))?;

    let schema_key = location.key_for(&format!("{drive_path}/schema.json"));
    let schema = r#"{"tables": {}, "version": 1}"#;

    s3_client
        .put_object(&location.bucket, &schema_key, schema.as_bytes().to_vec(), None)
        .await
        .map_err(|e| format!("Failed to store schema: {e}"))?;

    Ok(())
}
#[cfg(not(feature = "vibe"))]
pub(super) fn sync_to_serve_path(
    serve_path: &std::path::Path,
    html_content: &str,
    template_path: &std::path::Path,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    fs::create_dir_all(serve_path).map_err(|e| format!("Failed to create serve path: {e}"))?;

    let index_path = serve_path.join("index.html");
    fs::write(&index_path, html_content)
        .map_err(|e| format!("Failed to write index.html: {e}"))?;

    info!("Written: {}", index_path.display());

    let template_assets = template_path.join("_assets");
    let serve_assets = serve_path.join("_assets");

    if template_assets.exists() {
        copy_dir_recursive(&template_assets, &serve_assets)?;
        info!("Copied assets to: {}", serve_assets.display());
    } else {
        fs::create_dir_all(&serve_assets)
            .map_err(|e| format!("Failed to create assets dir: {e}"))?;

        let htmx_path = serve_assets.join("htmx.min.js");
        if !htmx_path.exists() {
            fs::write(&htmx_path, "/* HTMX - include from CDN or bundle */")
                .map_err(|e| format!("Failed to write htmx: {e}"))?;
        }

        let styles_path = serve_assets.join("styles.css");
        if !styles_path.exists() {
            fs::write(&styles_path, DEFAULT_STYLES)
                .map_err(|e| format!("Failed to write styles: {e}"))?;
        }

        let app_js_path = serve_assets.join("app.js");
        if !app_js_path.exists() {
            fs::write(&app_js_path, DEFAULT_APP_JS)
                .map_err(|e| format!("Failed to write app.js: {e}"))?;
        }
    }

    let schema_path = serve_path.join("schema.json");
    fs::write(&schema_path, r#"{"tables": {}, "version": 1}"#)
        .map_err(|e| format!("Failed to write schema.json: {e}"))?;

    Ok(())
}
#[cfg(not(feature = "vibe"))]
pub(super) fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> Result<(), Box<dyn Error + Send + Sync>> {
    fs::create_dir_all(dst)
        .map_err(|e| format!("Failed to create dir {}: {}", dst.display(), e))?;

    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy file {}: {}", src_path.display(), e))?;
        }
    }

    Ok(())
}
