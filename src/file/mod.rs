use actix_multipart::Multipart;
use actix_web::web;
use actix_web::{post, HttpResponse};
use aws_sdk_s3::{Client, Error as S3Error};
use std::io::Write;
use tempfile::NamedTempFile;
use tokio_stream::StreamExt as TokioStreamExt;

use crate::shared::state::AppState;

#[post("/files/upload/{folder_path}")]
pub async fn upload_file(
    folder_path: web::Path<String>,
    mut payload: Multipart,
    state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let folder_path = folder_path.into_inner();

    // Create a temporary file that will hold the uploaded data
    let mut temp_file = NamedTempFile::new().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to create temp file: {}", e))
    })?;

    let mut file_name: Option<String> = None;

    // Process multipart form data
    while let Some(mut field) = payload.try_next().await? {
        if let Some(disposition) = field.content_disposition() {
            if let Some(name) = disposition.get_filename() {
                file_name = Some(name.to_string());
            }
        }

        // Write each chunk of the field to the temporary file
        while let Some(chunk) = field.try_next().await? {
            temp_file.write_all(&chunk).map_err(|e| {
                actix_web::error::ErrorInternalServerError(format!(
                    "Failed to write to temp file: {}",
                    e
                ))
            })?;
        }
    }

    // Use a fallback name if the client didn't supply one
    let file_name = file_name.unwrap_or_else(|| "unnamed_file".to_string());

    // Convert the NamedTempFile into a TempPath so we can get a stable path
    let temp_file_path = temp_file.into_temp_path();

    // Retrieve the bucket name from configuration, handling the case where it is missing
    let bucket_name = match &state.config {
        Some(cfg) => cfg.s3_bucket.clone(),
        None => {
            // Clean up the temp file before returning the error
            let _ = std::fs::remove_file(&temp_file_path);
            return Err(actix_web::error::ErrorInternalServerError(
                "S3 bucket configuration is missing",
            ));
        }
    };

    // Build the S3 object key (folder + filename)
    let s3_key = format!("{}/{}", folder_path, file_name);

    // Perform the upload
    let s3_client = get_s3_client(&state).await;
    match upload_to_s3(&s3_client, &bucket_name, &s3_key, &temp_file_path).await {
        Ok(_) => {
            // Remove the temporary file now that the upload succeeded
            let _ = std::fs::remove_file(&temp_file_path);
            Ok(HttpResponse::Ok().body(format!(
                "Uploaded file '{}' to folder '{}' in S3 bucket '{}'",
                file_name, folder_path, bucket_name
            )))
        }
        Err(e) => {
            // Ensure the temporary file is cleaned up even on failure
            let _ = std::fs::remove_file(&temp_file_path);
            Err(actix_web::error::ErrorInternalServerError(format!(
                "Failed to upload file to S3: {}",
                e
            )))
        }
    }
}

// Helper function to get S3 client
async fn get_s3_client(state: &AppState) -> Client {
    if let Some(cfg) = &state.config.as_ref().and_then(|c| Some(&c.minio)) {
        // Build static credentials from the Drive configuration.
        let credentials = aws_sdk_s3::config::Credentials::new(
            cfg.access_key.clone(),
            cfg.secret_key.clone(),
            None,
            None,
            "static",
        );

        // Construct the endpoint URL, respecting the SSL flag.
        let scheme = if cfg.use_ssl { "https" } else { "http" };
        let endpoint = format!("{}://{}", scheme, cfg.server);

        // MinIO requires path‑style addressing.
        let s3_config = aws_sdk_s3::config::Builder::new()
            .region(aws_sdk_s3::config::Region::new("us-east-1"))
            .endpoint_url(endpoint)
            .credentials_provider(credentials)
            .force_path_style(true)
            .build();

        Client::from_conf(s3_config)
    } else {
        panic!("MinIO configuration is missing in application state");
    }
}

// Helper function to upload file to S3
async fn upload_to_s3(
    client: &Client,
    bucket: &str,
    key: &str,
    file_path: &std::path::Path,
) -> Result<(), S3Error> {
    // Convert the file at `file_path` into a `ByteStream`. Any I/O error is
    // turned into a construction‑failure `SdkError` so that the function’s
    // `Result` type (`Result<(), S3Error>`) stays consistent.
    let body = aws_sdk_s3::primitives::ByteStream::from_path(file_path)
        .await
        .map_err(|e| {
            aws_sdk_s3::error::SdkError::<
                aws_sdk_s3::operation::put_object::PutObjectError,
                aws_sdk_s3::operation::put_object::PutObjectOutput,
            >::construction_failure(e)
        })?;

    // Perform the actual upload to S3.
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(body)
        .send()
        .await?;

    Ok(())
}
