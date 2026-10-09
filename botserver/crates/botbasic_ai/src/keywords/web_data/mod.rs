pub(crate) use botbasic_types::UserSession;
pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use log::{debug, trace};
pub(crate) use rhai::{Array, Dynamic, Engine, EvalAltResult, Map, Position};
pub(crate) use scraper::{Html, Selector};

use reqwest::Url;
use std::sync::Arc;
use std::time::Duration;

mod part1;
mod part2;

pub use part1::*;
use part2::*;
