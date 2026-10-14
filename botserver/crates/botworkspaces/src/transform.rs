use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationalTransform {
    pub base_version: u64,
    pub operations: Vec<TransformOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformOperation {
    pub op_type: TransformOpType,
    pub path: Vec<usize>,
    pub value: Option<serde_json::Value>,
    pub old_value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransformOpType {
    Insert,
    Delete,
    Replace,
    Move,
}

pub fn transform_operations(
    op1: &TransformOperation,
    op2: &TransformOperation,
) -> (TransformOperation, TransformOperation) {
    let mut transformed_op1 = op1.clone();
    let mut transformed_op2 = op2.clone();

    if op1.path.is_empty() || op2.path.is_empty() {
        return (transformed_op1, transformed_op2);
    }

    let common_prefix_len = op1
        .path
        .iter()
        .zip(op2.path.iter())
        .take_while(|(a, b)| a == b)
        .count();

    if common_prefix_len == 0 {
        return (transformed_op1, transformed_op2);
    }

    match (&op1.op_type, &op2.op_type) {
        (TransformOpType::Insert, TransformOpType::Insert) => {
            if op1.path <= op2.path {
                if let Some(idx) = transformed_op2.path.get_mut(common_prefix_len) {
                    *idx += 1;
                }
            } else if let Some(idx) = transformed_op1.path.get_mut(common_prefix_len) {
                *idx += 1;
            }
        }
        (TransformOpType::Delete, TransformOpType::Insert) => {
            if op1.path < op2.path {
                if let Some(idx) = transformed_op2.path.get_mut(common_prefix_len) {
                    *idx = idx.saturating_sub(1);
                }
            }
        }
        (TransformOpType::Insert, TransformOpType::Delete) => {
            if op2.path < op1.path {
                if let Some(idx) = transformed_op1.path.get_mut(common_prefix_len) {
                    *idx = idx.saturating_sub(1);
                }
            }
        }
        (TransformOpType::Delete, TransformOpType::Delete)
            if op1.path == op2.path => {
                transformed_op2.op_type = TransformOpType::Replace;
                transformed_op2.value = None;
            }
        _ => {}
    }

    (transformed_op1, transformed_op2)
}

#[derive(Debug, Clone)]
pub enum CollaborationError {
    SessionNotFound,
    UserNotInSession,
    BroadcastError(String),
    InvalidOperation(String),
}

impl std::fmt::Display for CollaborationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SessionNotFound => write!(f, "Collaboration session not found"),
            Self::UserNotInSession => write!(f, "User is not in the session"),
            Self::BroadcastError(e) => write!(f, "Broadcast error: {e}"),
            Self::InvalidOperation(e) => write!(f, "Invalid operation: {e}"),
        }
    }
}

impl std::error::Error for CollaborationError {}

pub async fn collaboration_cleanup_job(manager: Arc<CollaborationManager>, interval_seconds: u64) {
    let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(interval_seconds));

    loop {
        ticker.tick().await;
        manager.cleanup_stale_sessions(300).await;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceInfo {
    pub page_id: Uuid,
    pub users: Vec<PresenceUser>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceUser {
    pub user_id: Uuid,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub color: String,
    pub is_typing: bool,
    pub current_block: Option<Uuid>,
}

impl From<&ActiveUser> for PresenceUser {
    fn from(user: &ActiveUser) -> Self {
        Self {
            user_id: user.user_id,
            display_name: user.display_name.clone(),
            avatar_url: user.avatar_url.clone(),
            color: user.color.clone(),
            is_typing: false,
            current_block: user.cursor_position.as_ref().map(|c| c.block_id),
        }
    }
}

