use super::*;

impl Default for WhiteboardManager {
    fn default() -> Self {
        Self::new()
    }
}
impl WhiteboardManager {
    pub fn new() -> Self {
        WhiteboardManager {
            whiteboards: Arc::new(RwLock::new(HashMap::new())),
            broadcast_channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_whiteboard(
        &self,
        conversation_id: Uuid,
        name: String,
    ) -> Uuid {
        let whiteboard_id = Uuid::new_v4();
        let state = WhiteboardState::new(whiteboard_id, conversation_id, name);

        let (tx, _) = broadcast::channel(1024);

        {
            let mut whiteboards = self.whiteboards.write().await;
            whiteboards.insert(whiteboard_id, state);
        }

        {
            let mut channels = self.broadcast_channels.write().await;
            channels.insert(whiteboard_id, tx);
        }

        whiteboard_id
    }

    pub async fn get_whiteboard(&self, whiteboard_id: &Uuid) -> Option<WhiteboardState> {
        let whiteboards = self.whiteboards.read().await;
        whiteboards.get(whiteboard_id).cloned()
    }

    pub async fn delete_whiteboard(&self, whiteboard_id: &Uuid) -> bool {
        let mut whiteboards = self.whiteboards.write().await;
        let mut channels = self.broadcast_channels.write().await;

        channels.remove(whiteboard_id);
        whiteboards.remove(whiteboard_id).is_some()
    }

    pub async fn subscribe(&self, whiteboard_id: &Uuid) -> Option<broadcast::Receiver<WhiteboardMessage>> {
        let channels = self.broadcast_channels.read().await;
        channels.get(whiteboard_id).map(|tx| tx.subscribe())
    }

    pub async fn broadcast(&self, whiteboard_id: &Uuid, message: WhiteboardMessage) {
        let channels = self.broadcast_channels.read().await;
        if let Some(tx) = channels.get(whiteboard_id) {
            let _ = tx.send(message);
        }
    }

    pub async fn apply_operation(
        &self,
        whiteboard_id: &Uuid,
        operation: WhiteboardOperation,
        user_id: Uuid,
    ) -> Result<(), String> {
        let mut whiteboards = self.whiteboards.write().await;
        let whiteboard = whiteboards
            .get_mut(whiteboard_id)
            .ok_or("Whiteboard not found")?;

        whiteboard.apply_operation(operation.clone(), user_id)?;

        drop(whiteboards);

        self.broadcast(
            whiteboard_id,
            WhiteboardMessage::Operation { user_id, operation },
        )
        .await;

        Ok(())
    }

    pub async fn undo(&self, whiteboard_id: &Uuid, user_id: Uuid) -> Option<WhiteboardOperation> {
        let mut whiteboards = self.whiteboards.write().await;
        if let Some(whiteboard) = whiteboards.get_mut(whiteboard_id) {
            whiteboard.undo(user_id)
        } else {
            None
        }
    }

    pub async fn redo(&self, whiteboard_id: &Uuid, user_id: Uuid) -> Option<WhiteboardOperation> {
        let mut whiteboards = self.whiteboards.write().await;
        if let Some(whiteboard) = whiteboards.get_mut(whiteboard_id) {
            whiteboard.redo(user_id)
        } else {
            None
        }
    }

    pub async fn update_cursor(&self, whiteboard_id: &Uuid, cursor: CursorPosition) {
        let mut whiteboards = self.whiteboards.write().await;
        if let Some(whiteboard) = whiteboards.get_mut(whiteboard_id) {
            whiteboard.update_cursor(cursor.clone());
        }
        drop(whiteboards);

        self.broadcast(whiteboard_id, WhiteboardMessage::CursorMove { cursor })
            .await;
    }

    pub async fn user_join(
        &self,
        whiteboard_id: &Uuid,
        user_id: Uuid,
        username: String,
        color: Color,
    ) {
        self.broadcast(
            whiteboard_id,
            WhiteboardMessage::UserJoined {
                user_id,
                username,
                color,
            },
        )
        .await;
    }

    pub async fn user_leave(&self, whiteboard_id: &Uuid, user_id: Uuid) {
        {
            let mut whiteboards = self.whiteboards.write().await;
            if let Some(whiteboard) = whiteboards.get_mut(whiteboard_id) {
                whiteboard.remove_cursor(&user_id);
                whiteboard.clear_selection(&user_id);
            }
        }

        self.broadcast(whiteboard_id, WhiteboardMessage::UserLeft { user_id })
            .await;
    }
}
impl Clone for WhiteboardState {
    fn clone(&self) -> Self {
        WhiteboardState {
            id: self.id,
            conversation_id: self.conversation_id,
            name: self.name.clone(),
            shapes: self.shapes.clone(),
            cursors: self.cursors.clone(),
            selections: self.selections.clone(),
            undo_stack: self.undo_stack.clone(),
            redo_stack: self.redo_stack.clone(),
            max_undo_history: self.max_undo_history,
            next_z_index: self.next_z_index,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
pub fn configure() -> Router<Arc<AppState>> {
    Router::new()
        .route("/whiteboard/:id/ws", get(whiteboard_websocket))
        .route("/whiteboard/create/:conversation_id", get(create_whiteboard))
}
pub(super) async fn create_whiteboard(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<Uuid>,
) -> impl IntoResponse {
    let manager = state
        .extensions
        .get::<WhiteboardManager>()
        .await
        .unwrap_or_else(|| Arc::new(WhiteboardManager::new()));

    let whiteboard_id = manager
        .create_whiteboard(conversation_id, "New Whiteboard".to_string())
        .await;

    axum::Json(serde_json::json!({
        "whiteboard_id": whiteboard_id,
        "conversation_id": conversation_id
    }))
}
pub(super) async fn whiteboard_websocket(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(whiteboard_id): Path<Uuid>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_whiteboard_socket(socket, state, whiteboard_id))
}
