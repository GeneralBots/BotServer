use crate::session::SessionManager;
use actix_web::{get, post, web, HttpResponse, Responder};
use log::info;
use uuid::Uuid;

pub struct BotOrchestrator {}

impl BotOrchestrator {
    pub fn new<A, B, C, D>(_a: A, _b: B, _c: C, _d: D) -> Self {
        info!("BotOrchestrator initialized");
        BotOrchestrator {}
    }
}

#[get("/")]
pub async fn index() -> impl Responder {
    info!("index requested");
    HttpResponse::Ok().body("General Bots")
}

#[get("/static")]
pub async fn static_files() -> impl Responder {
    info!("static_files requested");
    HttpResponse::Ok().body("static")
}

#[post("/voice/start")]
pub async fn voice_start() -> impl Responder {
    info!("voice_start requested");
    HttpResponse::Ok().body("voice started")
}

#[post("/voice/stop")]
pub async fn voice_stop() -> impl Responder {
    info!("voice_stop requested");
    HttpResponse::Ok().body("voice stopped")
}

#[post("/ws")]
pub async fn websocket_handler() -> impl Responder {
    info!("websocket_handler requested");
    HttpResponse::NotImplemented().finish()
}

#[post("/whatsapp/webhook")]
pub async fn whatsapp_webhook() -> impl Responder {
    info!("whatsapp_webhook called");
    HttpResponse::Ok().finish()
}

#[get("/whatsapp/verify")]
pub async fn whatsapp_webhook_verify() -> impl Responder {
    info!("whatsapp_webhook_verify called");
    HttpResponse::Ok().finish()
}

#[post("/session/create")]
pub async fn create_session(data: web::Data<SessionManagerWrapper>) -> impl Responder {
    let mut mgr = data.0.lock().unwrap();
    let id = mgr.create_session();
    info!("create_session -> {}", id);
    HttpResponse::Ok().body(id.to_string())
}

#[get("/sessions")]
pub async fn get_sessions(data: web::Data<SessionManagerWrapper>) -> impl Responder {
    let mgr = data.0.lock().unwrap();
    let list = mgr.list_sessions();
    HttpResponse::Ok().json(list)
}

#[get("/session/{id}/history")]
pub async fn get_session_history(
    path: web::Path<Uuid>,
    data: web::Data<SessionManagerWrapper>,
) -> impl Responder {
    let id = path.into_inner();
    let mgr = data.0.lock().unwrap();
    if let Some(sess) = mgr.get_session(&id) {
        HttpResponse::Ok().json(sess)
    } else {
        HttpResponse::NotFound().finish()
    }
}

#[post("/session/{id}/mode")]
pub async fn set_mode_handler(path: web::Path<Uuid>) -> impl Responder {
    let id = path.into_inner();
    info!("set_mode_handler called for {}", id);
    HttpResponse::Ok().finish()
}

use std::sync::{Arc, Mutex};
pub struct SessionManagerWrapper(pub Arc<Mutex<SessionManager>>);
