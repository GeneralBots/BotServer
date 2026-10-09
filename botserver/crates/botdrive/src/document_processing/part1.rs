use super::*;

pub async fn merge_documents(
    State(state): State<Arc<AppState>>,
    Json(req): Json<MergeDocumentsRequest>,
) -> Result<Json<DocumentResponse>, (StatusCode, Json<serde_json::Value>)> {
    let s3_client = state.drive.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "S3 service not available" })),
        )
    })?;

    if req.source_paths.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "No source documents provided" })),
        ));
    }

    let mut merged_content = String::new();
    let format = req.format.as_deref().unwrap_or("txt");

for (idx, path) in req.source_paths.iter().enumerate() {
let bytes = s3_client
.get_object(&req.bucket, path)
.await
.map_err(|e| {
(
StatusCode::INTERNAL_SERVER_ERROR,
Json(serde_json::json!({ "error": format!("Failed to read document {}: {}", path, e) })),
)
})?;

let content = String::from_utf8(bytes).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Document is not valid UTF-8: {}", e) })),
            )
        })?;

        if idx > 0 {
            merged_content.push_str("\n\n");
            if format == "md" || format == "markdown" {
                merged_content.push_str("---\n\n");
            }
        }
        merged_content.push_str(&content);
    }

    s3_client
        .put_object(&req.bucket, &req.output_path, merged_content.into_bytes(), None)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to write merged document: {}", e) })),
            )
        })?;

    Ok(Json(DocumentResponse {
        success: true,
        output_path: Some(req.output_path),
        message: Some(format!(
            "Successfully merged {} documents",
            req.source_paths.len()
        )),
        metadata: Some(serde_json::json!({
            "source_count": req.source_paths.len(),
            "format": format
        })),
    }))
}
pub async fn convert_document(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ConvertDocumentRequest>,
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
                Json(serde_json::json!({ "error": format!("Failed to read source document: {}", e) })),
            )
        })?;

    let source_content = String::from_utf8(bytes.to_vec()).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Document is not valid UTF-8: {}", e) })),
        )
    })?;

    let converted_content = match (req.from_format.as_str(), req.to_format.as_str()) {
        ("txt", "md") | ("text", "markdown") => {
            format!("# Converted Document\n\n{}", source_content)
        }
        ("md", "txt") | ("markdown", "text") => source_content
            .lines()
            .map(|line| {
                line.trim_start_matches('#')
                    .trim_start_matches('*')
                    .trim_start_matches('-')
                    .trim()
            })
            .collect::<Vec<_>>()
            .join("\n"),
        ("json", "csv") => {
            let data: Result<serde_json::Value, _> = serde_json::from_str(&source_content);
            match data {
                Ok(serde_json::Value::Array(arr)) => {
                    if arr.is_empty() {
                        String::new()
                    } else {
                        let headers = if let Some(serde_json::Value::Object(first)) = arr.first() {
                            first.keys().cloned().collect::<Vec<_>>()
                        } else {
                            vec![]
                        };

                        let mut csv = headers.join(",") + "\n";
                        for item in arr {
                            if let serde_json::Value::Object(obj) = item {
                                let row = headers
                                    .iter()
                                    .map(|h| {
                                        obj.get(h)
                                            .map(|v| {
                                                if let Some(s) = v.as_str() {
                                                    s.to_string()
                                                } else {
                                                    v.to_string()
                                                }
                                            })
                                            .unwrap_or_else(String::new)
                                    })
                                    .collect::<Vec<_>>()
                                    .join(",");
                                csv.push_str(&row);
                                csv.push('\n');
                            }
                        }
                        csv
                    }
                }
                _ => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(
                            serde_json::json!({ "error": "JSON must be an array for CSV conversion" }),
                        ),
                    ));
                }
            }
        }
        ("csv", "json") => {
            let lines: Vec<&str> = source_content.lines().collect();
            if lines.is_empty() {
                "[]".to_string()
            } else {
                let headers: Vec<&str> = lines[0].split(',').collect();
                let mut result = Vec::new();

                for line in lines.iter().skip(1) {
                    let values: Vec<&str> = line.split(',').collect();
                    let mut obj = serde_json::Map::new();
                    for (i, header) in headers.iter().enumerate() {
                        if let Some(value) = values.get(i) {
                            obj.insert(header.trim().to_string(), serde_json::json!(value.trim()));
                        }
                    }
                    result.push(serde_json::Value::Object(obj));
                }
                serde_json::to_string_pretty(&result).unwrap_or_else(|_| "[]".to_string())
            }
        }
        ("html", "txt" | "text") => {
            source_content
                .replace("<br>", "\n")
                .replace("<p>", "\n")
                .replace("</p>", "\n")
                .chars()
                .fold((String::new(), false), |(mut acc, in_tag), c| {
                    if c == '<' {
                        (acc, true)
                    } else if c == '>' {
                        (acc, false)
                    } else if !in_tag {
                        acc.push(c);
                        (acc, in_tag)
                    } else {
                        (acc, in_tag)
                    }
                })
                .0
        }
        _ => source_content,
    };

    s3_client
        .put_object(&req.bucket, &req.output_path, converted_content.into_bytes(), None)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to write converted document: {}", e) })),
            )
        })?;

    Ok(Json(DocumentResponse {
        success: true,
        output_path: Some(req.output_path),
        message: Some(format!(
            "Successfully converted from {} to {}",
            req.from_format, req.to_format
        )),
        metadata: Some(serde_json::json!({
            "from_format": req.from_format,
            "to_format": req.to_format
        })),
    }))
}
pub async fn fill_document(
    State(state): State<Arc<AppState>>,
    Json(req): Json<FillDocumentRequest>,
) -> Result<Json<DocumentResponse>, (StatusCode, Json<serde_json::Value>)> {
    let s3_client = state.drive.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "S3 service not available" })),
        )
    })?;

    let bytes = s3_client
        .get_object(&req.bucket, &req.template_path)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to read template: {}", e) })),
            )
        })?;

    let mut template = String::from_utf8(bytes.to_vec()).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Template is not valid UTF-8: {}", e) })),
        )
    })?;

    if let serde_json::Value::Object(data_map) = &req.data {
        for (key, value) in data_map {
            let placeholder = format!("{{{{{}}}}}", key);
            let replacement = match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Null => String::new(),
                _ => value.to_string(),
            };
            template = template.replace(&placeholder, &replacement);
        }
    }

    s3_client
        .put_object(&req.bucket, &req.output_path, template.into_bytes(), None)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to write filled document: {}", e) })),
            )
        })?;

    Ok(Json(DocumentResponse {
        success: true,
        output_path: Some(req.output_path),
        message: Some("Successfully filled document template".to_string()),
        metadata: Some(serde_json::json!({
            "template": req.template_path,
            "fields_filled": req.data.as_object().map(|o| o.len()).unwrap_or(0)
        })),
    }))
}
