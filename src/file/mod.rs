use actix_multipart::Multipart;
use actix_web::{get, post, web, HttpResponse, Result};
use futures_util::StreamExt as _;
use log::info;
use std::io::Write;
use tokio::fs;

#[post("/files/upload/{folder_path}")]
pub async fn upload_file(
    mut payload: Multipart,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let folder_path = path.into_inner();
    
    while let Some(item) = payload.next().await {
        let mut field = item?;
        let content_disposition = field.content_disposition();
        
        let file_name = if let Some(name) = content_disposition.get_filename() {
            name.to_string()
        } else {
            continue;
        };

        let file_path = format!("./uploads/{}/{}", folder_path, file_name);
        
        if let Some(parent) = std::path::Path::new(&file_path).parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut f = web::block(|| std::fs::File::create(&file_path))
            .await??;

        while let Some(chunk) = field.next().await {
            let data = chunk?;
            f = web::block(move || f.write_all(&data).map(|_| f)).await??;
        }
    }

    info!("File uploaded to folder: {}", folder_path);
    Ok(HttpResponse::Ok().json(serde_json::json!({"status": "uploaded"})))
}

#[post("/files/list/{folder_path}")]
pub async fn list_file(
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let folder_path = path.into_inner();
    let dir_path = format!("./uploads/{}", folder_path);
    
    let mut entries = Vec::new();
    
    if let Ok(mut read_dir) = fs::read_dir(&dir_path).await {
        while let Ok(Some(entry)) = read_dir.next_entry().await {
            if let Ok(file_name) = entry.file_name().into_string() {
                entries.push(file_name);
            }
        }
    }

    Ok(HttpResponse::Ok().json(entries))
}

#[get("/files/download/{file_path:.*}")]
pub async fn download_file(
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let file_path = path.into_inner();
    let full_path = format!("./uploads/{}", file_path);
    
    if let Ok(content) = fs::read(&full_path).await {
        Ok(HttpResponse::Ok()
            .content_type("application/octet-stream")
            .body(content))
    } else {
        Ok(HttpResponse::NotFound().body("File not found"))
    }
}
