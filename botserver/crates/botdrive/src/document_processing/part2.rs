use super::*;

pub async fn export_document(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExportDocumentRequest>,
) -> Result<Json<DocumentResponse>, (StatusCode, Json<serde_json::Value>)> {
    let s3_client = state.drive.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "S3 service not available" })),
        )
    })?;

    let bytes = s3_client
        .get_object(&req.bucket, &req.source_path)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdrive::document_processing::part2", None) })),
            )
})?
    ;

    let content = String::from_utf8(bytes.to_vec()).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdrive::document_processing::part2", None) })),
        )
    })?;

    let exported_content = match req.format.as_str() {
        "pdf" => {
            format!("PDF Export:\n{}", content)
        }
        "docx" => {
            format!("DOCX Export:\n{}", content)
        }
        "html" => {
            format!("<!DOCTYPE html>\n<html>\n<head><title>Exported Document</title></head>\n<body>\n<pre>{}</pre>\n</body>\n</html>", content)
        }
        "xml" => {
            format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<document>\n<content><![CDATA[{}]]></content>\n</document>", content)
        }
        _ => content,
    };

    Ok(Json(DocumentResponse {
        success: true,
        output_path: None,
        message: Some(format!("Document exported as {}", req.format)),
        metadata: Some(serde_json::json!({
            "format": req.format,
            "size": exported_content.len(),
            "content": exported_content
        })),
    }))
}
pub async fn import_document(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ImportDocumentRequest>,
) -> Result<Json<DocumentResponse>, (StatusCode, Json<serde_json::Value>)> {
    let s3_client = state.drive.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "S3 service not available" })),
        )
    })?;

    let content = if let Some(url) = &req.source_url {
        let client = reqwest::Client::new();
        let response = client.get(url).send().await.map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdrive::document_processing::part2", None) })),
            )
        })?;

        response.text().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdrive::document_processing::part2", None) })),
            )
        })?
    } else if let Some(data) = &req.source_data {
        data.clone()
    } else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({ "error": "Either source_url or source_data must be provided" }),
            ),
        ));
    };

    let processed_content = match req.format.as_str() {
        "json" => {
            let parsed: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdrive::document_processing::part2", None) })),
                )
            })?;
            serde_json::to_string_pretty(&parsed).unwrap_or(content)
        }
        // "xml", "csv", and any other format pass through unchanged
        _ => content,
    };

    s3_client
        .put_object(&req.bucket, &req.output_path, processed_content.into_bytes(), None)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdrive::document_processing::part2", None) })),
            )
        })?;

    Ok(Json(DocumentResponse {
        success: true,
        output_path: Some(req.output_path),
        message: Some("Document imported successfully".to_string()),
        metadata: Some(serde_json::json!({
            "format": req.format,
            "source_type": if req.source_url.is_some() { "url" } else { "data" }
        })),
    }))
}
