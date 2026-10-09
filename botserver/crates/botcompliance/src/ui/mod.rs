pub(crate) use axum::{ extract::{Path, State}, response::Html, routing::get, Router, };
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use crate::DbPool;

mod part1;
mod part2;
mod part3;

pub use part1::*;
pub use part2::*;
pub use part3::*;
