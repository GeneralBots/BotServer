pub(crate) use botbasic_types::UserSession;
pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use botlib::limits::MAX_LOOP_ITERATIONS;
pub(crate) use diesel::prelude::*;
pub(crate) use log::trace;
pub(crate) use rhai::{Dynamic, Engine};
pub(crate) use std::collections::HashMap;
pub(crate) use std::path::PathBuf;
pub(crate) use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct ProcedureDefinition {
    pub name: String,
    pub params: Vec<String>,
    pub body: String,
    pub is_function: bool,
}

static PROCEDURES: std::sync::LazyLock<Arc<Mutex<HashMap<String, ProcedureDefinition>>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

mod part1;
mod part2;
mod part3;

pub use part1::*;
pub use part2::*;
pub use part3::*;
