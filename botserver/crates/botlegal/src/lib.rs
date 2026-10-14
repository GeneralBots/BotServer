

pub mod account_deletion;

use axum::{
    extract::{Path, Query, State},
    response::Html,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

mod lib_part1;
mod lib_part2;
mod lib_part3;
mod lib_part4;
mod lib_part5;
mod lib_part6;
mod lib_part7;

pub use lib_part1::*;
pub use lib_part2::*;
pub use lib_part3::*;
pub use lib_part4::*;
pub use lib_part5::*;
pub use lib_part6::*;
pub use lib_part7::*;
