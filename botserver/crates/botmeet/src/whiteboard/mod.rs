pub(crate) use axum::{ extract::{ ws::{Message, WebSocket, WebSocketUpgrade}, Path, State, }, response::IntoResponse, routing::get, Router, };
pub(crate) use chrono::{DateTime, Utc};
pub(crate) use futures::{SinkExt, StreamExt};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::{broadcast, RwLock};
pub(crate) use uuid::Uuid;
pub(crate) use botcore::shared::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ShapeType {
    Rectangle,
    Ellipse,
    Line,
    Arrow,
    Freehand,
    Text,
    Image,
    Sticky,
    Polygon,
    Triangle,
    Diamond,
    Star,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StrokeStyle {
    Solid,
    Dashed,
    Dotted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeStyle {
    pub stroke_color: Color,
    pub fill_color: Option<Color>,
    pub stroke_width: f64,
    pub stroke_style: StrokeStyle,
    pub opacity: f32,
    pub font_size: Option<f64>,
    pub font_family: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shape {
    pub id: Uuid,
    pub shape_type: ShapeType,
    pub points: Vec<Point>,
    pub style: ShapeStyle,
    pub text: Option<String>,
    pub image_url: Option<String>,
    pub rotation: f64,
    pub z_index: i32,
    pub locked: bool,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub user_id: Uuid,
    pub username: String,
    pub color: Color,
    pub position: Point,
    pub last_update: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selection {
    pub user_id: Uuid,
    pub shape_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WhiteboardOperation {
    AddShape { shape: Shape },
    UpdateShape { shape: Shape },
    DeleteShape { shape_id: Uuid },
    MoveShapes { shape_ids: Vec<Uuid>, delta: Point },
    ResizeShape { shape_id: Uuid, bounds: ShapeBounds },
    RotateShape { shape_id: Uuid, angle: f64 },
    BringToFront { shape_id: Uuid },
    SendToBack { shape_id: Uuid },
    LockShape { shape_id: Uuid },
    UnlockShape { shape_id: Uuid },
    GroupShapes { shape_ids: Vec<Uuid>, group_id: Uuid },
    UngroupShapes { group_id: Uuid },
    Clear,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoRedoEntry {
    pub id: Uuid,
    pub operation: WhiteboardOperation,
    pub inverse_operation: WhiteboardOperation,
    pub user_id: Uuid,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WhiteboardMessage {
    Join {
        user_id: Uuid,
        username: String,
        color: Color,
    },
    Leave {
        user_id: Uuid,
    },
    Operation {
        user_id: Uuid,
        operation: WhiteboardOperation,
    },
    CursorMove {
        cursor: CursorPosition,
    },
    Select {
        selection: Selection,
    },
    Deselect {
        user_id: Uuid,
    },
    Undo {
        user_id: Uuid,
    },
    Redo {
        user_id: Uuid,
    },
    RequestSync,
    FullSync {
        shapes: Vec<Shape>,
        cursors: Vec<CursorPosition>,
        selections: Vec<Selection>,
    },
    Ping,
    Pong,
    Error {
        message: String,
    },
    UserJoined {
        user_id: Uuid,
        username: String,
        color: Color,
    },
    UserLeft {
        user_id: Uuid,
    },
}

#[derive(Debug)]
pub struct WhiteboardState {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub name: String,
    pub shapes: HashMap<Uuid, Shape>,
    pub cursors: HashMap<Uuid, CursorPosition>,
    pub selections: HashMap<Uuid, Selection>,
    pub undo_stack: Vec<UndoRedoEntry>,
    pub redo_stack: Vec<UndoRedoEntry>,
    pub max_undo_history: usize,
    pub next_z_index: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct WhiteboardManager {
    whiteboards: Arc<RwLock<HashMap<Uuid, WhiteboardState>>>,
    broadcast_channels: Arc<RwLock<HashMap<Uuid, broadcast::Sender<WhiteboardMessage>>>>,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

pub use part4::*;
use part5::*;
