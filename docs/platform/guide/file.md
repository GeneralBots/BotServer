# File Upload Service with Actix Web and S3/MinIO

## Overview

This service provides a REST API endpoint for uploading files to S3-compatible storage (including MinIO) using Actix Web. It handles multipart form data, temporarily stores files locally, and transfers them to object storage.

## BASIC Keywords Reference

- **UPLOAD**: Handles file uploads via multipart form data
- **CONFIG**: Manages S3/MinIO configuration and client initialization
- **TEMP**: Uses temporary files for processing uploads
- **CLIENT**: Maintains S3 client connection
- **ERROR**: Comprehensive error handling for upload failures
- **BUCKET**: Configures and uses S3 buckets for storage
- **PATH**: Manages folder paths for object organization

## API Reference

### POST `/files/upload/{folder_path}`

Uploads a file to the specified folder in S3/MinIO storage.

**Path Parameters:**
- `folder_path` (string): Target folder path in S3 bucket

**Request:**
- Content-Type: `multipart/form-data`
- Body: File data in multipart format

**Response:**
- `200 OK`: Upload successful
- `500 Internal Server Error`: Upload failed

**Example:**
```bash
curl -X POST \
  http://localhost:8080/files/upload/documents \
  -F "file=@report.pdf"
```

## Configuration

### DriveConfig Structure

```rust
// Example configuration
let config = DriveConfig {
    access_key: "your-access-key".to_string(),
    secret_key: "your-secret-key".to_string(),
    server: "minio.example.com:9000".to_string(),
    s3_bucket: "my-bucket".to_string(),
    use_ssl: false,
};
```

### Client Initialization

```rust
use crate::config::DriveConfig;

// Initialize S3 client
let drive_config = DriveConfig {
    access_key: "minioadmin".to_string(),
    secret_key: "minioadmin".to_string(),
    server: "localhost:9000".to_string(),
    s3_bucket: "uploads".to_string(),
    use_ssl: false,
};

let s3_client = init_drive(&drive_config).await?;
```

## Implementation Guide

### 1. Setting Up AppState

```rust
use crate::shared::state::AppState;

// Configure application state with S3 client
let app_state = web::Data::new(AppState {
    s3_client: Some(s3_client),
    config: Some(drive_config),
    // ... other state fields
});
```

### 2. Error Handling Patterns

The service implements several error handling strategies:

```rust
// Configuration errors
let bucket_name = state.get_ref().config.as_ref()
    .ok_or_else(|| actix_web::error::ErrorInternalServerError(
        "S3 bucket configuration is missing"
    ))?;

// Client initialization errors
let s3_client = state.get_ref().s3_client.as_ref()
    .ok_or_else(|| actix_web::error::ErrorInternalServerError(
        "S3 client is not initialized"
    ))?;

// File operation errors with cleanup
let mut temp_file = NamedTempFile::new().map_err(|e| {
    actix_web::error::ErrorInternalServerError(format!(
        "Failed to create temp file: {}", e
    ))
})?;
```

### 3. File Processing Flow

```rust
// 1. Create temporary file
let mut temp_file = NamedTempFile::new()?;

// 2. Process multipart data
while let Some(mut field) = payload.try_next().await? {
    // Extract filename from content disposition
    if let Some(disposition) = field.content_disposition() {
        file_name = disposition.get_filename().map(|s| s.to_string());
    }

    // Stream data to temporary file
    while let Some(chunk) = field.try_next().await? {
        temp_file.write_all(&chunk)?;
    }
}

// 3. Upload to S3
upload_to_s3(&s3_client, &bucket_name, &s3_key, &temp_file_path).await?;

// 4. Cleanup temporary file
let _ = std::fs::remove_file(&temp_file_path);
```

## Key Features

### Temporary File Management
- Uses `NamedTempFile` for secure temporary storage
- Automatic cleanup on both success and failure
- Efficient streaming of multipart data

### S3/MinIO Compatibility
- Path-style addressing for MinIO compatibility
- Configurable SSL/TLS
- Custom endpoint support

### Security Considerations
- Temporary files are automatically deleted
- No persistent storage of uploaded files on server
- Secure credential handling

## Error Scenarios

1. **Missing Configuration**: Returns 500 if S3 bucket or client not configured
2. **File System Errors**: Handles temp file creation/write failures
3. **Network Issues**: Manages S3 connection timeouts and errors
4. **Invalid Uploads**: Handles malformed multipart data

## Performance Notes

- Streams data directly from multipart to temporary file
- Uses async operations for I/O-bound tasks
- Minimal memory usage for large file uploads
- Efficient cleanup prevents disk space leaks
