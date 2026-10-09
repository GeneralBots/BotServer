pub(crate) use std::net::IpAddr;
pub(crate) use std::sync::Arc;
pub(crate) use axum::{ extract::{Path, Query, State, WebSocketUpgrade}, extract::ws::{Message, WebSocket}, response::IntoResponse, response::Json, routing::{delete, get, post}, Router, };
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use tokio::sync::Mutex;
pub(crate) use crate::{agent, BrowserSession, SessionMap};

#[derive(Deserialize)]
pub struct ProxyParams {
    pub url: String,
}

#[cfg(test)]
mod tests {
use super::*;

#[test]
fn private_ip_detection() {
    assert!(is_private_ip("127.0.0.1".parse().unwrap()));
    assert!(is_private_ip("192.168.1.1".parse().unwrap()));
    assert!(is_private_ip("10.0.0.1".parse().unwrap()));
    assert!(is_private_ip("::1".parse().unwrap()));
    assert!(!is_private_ip("8.8.8.8".parse().unwrap()));
    assert!(!is_private_ip("93.184.216.34".parse().unwrap()));
}
}

#[derive(Deserialize)]
pub struct CreateSessionRequest {
    pub headless: Option<bool>,
}

#[derive(Deserialize)]
pub struct NavigateRequest {
    pub url: String,
}

#[derive(Deserialize)]
pub struct ClickRequest {
    pub selector: String,
}

#[derive(Deserialize)]
pub struct FillRequest {
    pub selector: String,
    pub text: String,
}

#[derive(Deserialize)]
pub struct ExecuteRequest {
    pub script: String,
}

#[derive(Deserialize)]
pub struct AgentRequest {
    pub goal: String,
    pub max_steps: Option<usize>,
    pub llm_url: Option<String>,
    pub llm_key: Option<String>,
    pub llm_model: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct WsAgentParams {
    pub goal: String,
    pub max_steps: Option<usize>,
    pub llm_url: Option<String>,
    pub llm_key: Option<String>,
    pub llm_model: Option<String>,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
mod part8;
mod part9;

pub use part1::*;
use part2::*;
pub use part3::*;
pub use part4::*;
pub use part5::*;
pub use part6::*;
pub use part7::*;
pub use part8::*;
pub use part9::*;
