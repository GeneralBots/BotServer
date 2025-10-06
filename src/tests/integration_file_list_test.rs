use actix_web::{test, web, App};
use anyhow::Result;
use bytes::Bytes;
use gb_core::models::AppState;
use gb_file::handlers::list_file;
use minio::s3::args::{BucketExistsArgs, MakeBucketArgs};
use minio::s3::builders::SegmentedBytes;
use minio::s3::client::ClientBuilder as MinioClientBuilder;
use minio::s3::creds::StaticProvider;
use minio::s3::http::BaseUrl;
use minio::s3::types::ToStream;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::str::FromStr;
use tempfile::NamedTempFile;
use tokio_stream::StreamExt;

#[tokio::test]

async fn test_successful_file_listing() -> Result<(), Box<dyn std::error::Error>> {
    // Setup test environment and MinIO client
    let base_url = format!("http://{}", "localhost:9000");
    let base_url = BaseUrl::from_str(&base_url)?;
    let credentials = StaticProvider::new("minioadmin", "minioadmin", None);

    let minio_client = MinioClientBuilder::new(base_url.clone())
        .provider(Some(Box::new(credentials)))
        .build()?;

    // Create test bucket if it doesn't exist
    let bucket_name = "file-upload-rust-bucket";

    // Using object-based API for bucket_exists
    let bucket_exists_args = BucketExistsArgs::new(bucket_name)?;
    let bucket_exists = minio_client.bucket_exists(&bucket_exists_args).await?;

    if !bucket_exists {
        // Using object-based API for make_bucket
        let make_bucket_args = MakeBucketArgs::new(bucket_name)?;
        minio_client.make_bucket(&make_bucket_args).await?;
    }

    // Put a single file in the bucket
    let folder_path = "test-folder";
    let file_name = "test.txt";
    let object_name = format!("{}/{}", folder_path, file_name);

    // Create a temporary file with some content
    let mut temp_file = NamedTempFile::new()?;
    writeln!(temp_file, "This is a test file.")?;

    // Upload the file to the bucket
    let mut file = File::open(temp_file.path())?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let content = SegmentedBytes::from(Bytes::from(buffer));
    minio_client.put_object(bucket_name, &object_name, content);

    let app_state = web::Data::new(AppState {
        minio_client: Some(minio_client.clone()),
        config: None,
        db_pool: None,
    });

    let app = test::init_service(App::new().app_data(app_state.clone()).service(list_file)).await;

    // Execute request to list files in the folder
    let req = test::TestRequest::post()
        .uri(&format!("/files/list/{}", folder_path))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Verify response
    assert_eq!(resp.status(), 200);

    // Parse the response body as JSON
    let body = test::read_body(resp).await;
    let file_list: Vec<String> = serde_json::from_slice(&body)?;

    // Verify the uploaded file is in the list
    assert!(
        file_list.contains(&object_name),
        "Uploaded file should be listed"
    );

    // List all objects in a directory.
    let mut list_objects = minio_client
        .list_objects("my-bucket")
        .use_api_v1(true)
        .recursive(true)
        .to_stream()
        .await;
    while let Some(result) = list_objects.next().await {
        match result {
            Ok(resp) => {
                for item in resp.contents {
                    info!("{:?}", item);
                }
            }
            Err(e) => info!("Error: {:?}", e),
        }
    }

    Ok(())
}
