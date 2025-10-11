use actix_web::web;
use actix_multipart::Multipart;
use actix_web::{post, HttpResponse};
use std::io::Write;
use tempfile::NamedTempFile;
use tokio_stream::StreamExt;
use aws_sdk_s3 as s3;
use aws_sdk_s3::types::ByteStream;
use std::str::FromStr;

use crate::config::AppConfig;
use crate::shared::state::AppState;

pub async fn init_s3(config: &AppConfig) -> Result<s3::Client, Box<dyn std::error::Error>> {
    let endpoint_url = if config.minio.use_ssl {
        format!("https://{}", config.minio.server)
    } else {
        format!("http://{}", config.minio.server)
    };

    let config = aws_config::from_env()
        .endpoint_url(&endpoint_url)
        .region(aws_sdk_s3::config::Region::new("us-east-1"))
        .credentials_provider(
            s3::config::Credentials::new(
                &config.minio.access_key,
                &config.minio.secret_key,
                None,
                None,
                "minio",
            )
        )
        .load()
        .await;

    let client = s3::Client::new(&config);
    Ok(client)
}

#[post("/files/upload/{folder_path}")]
pub async fn upload_file(
    folder_path: web::Path<String>,
    mut payload: Multipart,
    state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let folder_path = folder_path.into_inner();

    let mut temp_file = NamedTempFile::new().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to create temp file: {}", e))
    })?;

    let mut file_name: Option<String> = None;

    while let Some(mut field) = payload.try_next().await? {
        if let Some(disposition) = field.content_disposition() {
            if let Some(name) = disposition.get_filename() {
                file_name = Some(name.to_string());
            }
        }

        while let Some(chunk) = field.try_next().await? {
            temp_file.write_all(&chunk).map_err(|e| {
                actix_web::error::ErrorInternalServerError(format!(
                    "Failed to write to temp file: {}",
                    e
                ))
            })?;
        }
    }

    let file_name = file_name.unwrap_or_else(|| "unnamed_file".to_string());
    let object_name = format!("{}/{}", folder_path, file_name);

    let client = state.s3_client.as_ref().ok_or_else(|| {
        actix_web::error::ErrorInternalServerError("S3 client not initialized")
    })?;

    let bucket_name = state.config.as_ref().unwrap().minio.bucket.clone();

    let body = ByteStream::from_path(temp_file.path()).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to read file: {}", e))
    })?;

    client
        .put_object()
        .bucket(&bucket_name)
        .key(&object_name)
        .body(body)
        .send()
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!(
                "Failed to upload file to S3: {}",
                e
            ))
        })?;

    temp_file.close().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to close temp file: {}", e))
    })?;

    Ok(HttpResponse::Ok().body(format!(
        "Uploaded file '{}' to folder '{}'",
        file_name, folder_path
    )))
}

#[post("/files/list/{folder_path}")]
pub async fn list_file(
    folder_path: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let folder_path = folder_path.into_inner();

    let client = state.s3_client.as_ref().ok_or_else(|| {
        actix_web::error::ErrorInternalServerError("S3 client not initialized")
    })?;

    let bucket_name = "file-upload-rust-bucket";

    let mut objects = client
        .list_objects_v2()
        .bucket(bucket_name)
        .prefix(&folder_path)
        .into_paginator()
        .send();

    let mut file_list = Vec::new();

    while let Some(result) = objects.next().await {
        match result {
            Ok(output) => {
                if let Some(contents) = output.contents {
                    for item in contents {
                        if let Some(key) = item.key {
                            file_list.push(key);
                        }
                    }
                }
            }
            Err(e) => {
                return Err(actix_web::error::ErrorInternalServerError(format!(
                    "Failed to list files in S3: {}",
                    e
                )));
            }
        }
    }

    Ok(HttpResponse::Ok().json(file_list))
}
