pub(crate) use botbasic_types::UserSession;
pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use log::trace;
pub(crate) use reqwest::{header::HeaderMap, header::HeaderName, header::HeaderValue, Client, Method};
pub(crate) use rhai::{Dynamic, Engine, Map};
pub(crate) use serde_json::{json, Value};
pub(crate) use std::sync::{Arc, Mutex};

thread_local! {
    static HTTP_HEADERS: std::cell::RefCell<HashMap<String, String>> = std::cell::RefCell::new(HashMap::new());
}

use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;

mod part1;
mod part2;
mod part3;

pub use part1::*;
pub use part2::*;
use part3::*;
