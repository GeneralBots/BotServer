use actix_web::{HttpRequest, HttpResponse, Result};
use log::{debug, error, warn};
use std::fs;

#[actix_web::get("/")]
async fn index() -> Result<HttpResponse> {
    match fs::read_to_string("web/index.html") {
        Ok(html) => Ok(HttpResponse::Ok().content_type("text/html").body(html)),
        Err(e) => {
            error!("Failed to load index page: {}", e);
            Ok(HttpResponse::InternalServerError().body("Failed to load index page"))
        }
    }
}

#[actix_web::get("/static/{filename:.*}")]
async fn static_files(req: HttpRequest) -> Result<HttpResponse> {
    let filename = req.match_info().query("filename");
    let path = format!("web/static/{}", filename);
    match fs::read(&path) {
        Ok(content) => {
            debug!(
                "Static file {} loaded successfully, size: {} bytes",
                filename,
                content.len()
            );
            let content_type = match filename {
                f if f.ends_with(".js") => "application/javascript",
                f if f.ends_with(".css") => "text/css",
                f if f.ends_with(".png") => "image/png",
                f if f.ends_with(".jpg") | f.ends_with(".jpeg") => "image/jpeg",
                _ => "text/plain",
            };
            Ok(HttpResponse::Ok().content_type(content_type).body(content))
        }
        Err(e) => {
            warn!("Static file not found: {} - {}", filename, e);
            Ok(HttpResponse::NotFound().body("File not found"))
        }
    }
}
