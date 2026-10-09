

pub mod models;
pub mod scope;
pub mod employee;
pub mod attendance;
pub mod payroll;
pub mod time_clock;
pub mod ui;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post, put},
    Json, Router,
};

use bigdecimal::BigDecimal;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

mod lib_part1;
mod lib_part2;
mod lib_part3;
mod lib_part4;
mod lib_part5;

pub use lib_part1::*;
pub use lib_part2::*;
pub use lib_part3::*;
pub use lib_part4::*;
pub use lib_part5::*;
