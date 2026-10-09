use super::*;

impl WhiteboardState {
    pub fn new(id: Uuid, conversation_id: Uuid, name: String) -> Self {
        let now = Utc::now();
        WhiteboardState {
            id,
            conversation_id,
            name,
            shapes: HashMap::new(),
            cursors: HashMap::new(),
            selections: HashMap::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_history: 100,
            next_z_index: 0,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn apply_operation(
        &mut self,
        operation: WhiteboardOperation,
        user_id: Uuid,
    ) -> Result<Option<WhiteboardOperation>, String> {
        let inverse = self.compute_inverse(&operation)?;

        match operation {
            WhiteboardOperation::AddShape { shape } => {
                let mut shape = shape;
                shape.z_index = self.next_z_index;
                self.next_z_index += 1;
                self.shapes.insert(shape.id, shape);
            }
            WhiteboardOperation::UpdateShape { shape } => {
                if !self.shapes.contains_key(&shape.id) {
                    return Err("Shape not found".to_string());
                }
                self.shapes.insert(shape.id, shape);
            }
            WhiteboardOperation::DeleteShape { shape_id } => {
                if self.shapes.remove(&shape_id).is_none() {
                    return Err("Shape not found".to_string());
                }
                for selection in self.selections.values_mut() {
                    selection.shape_ids.retain(|id| *id != shape_id);
                }
            }
            WhiteboardOperation::MoveShapes { shape_ids, delta } => {
                for shape_id in &shape_ids {
                    if let Some(shape) = self.shapes.get_mut(shape_id) {
                        for point in &mut shape.points {
                            point.x += delta.x;
                            point.y += delta.y;
                        }
                        shape.updated_at = Utc::now();
                    }
                }
            }
            WhiteboardOperation::ResizeShape { shape_id, bounds } => {
                if let Some(shape) = self.shapes.get_mut(&shape_id) {
                    if shape.points.len() >= 2 {
                        shape.points[0] = Point {
                            x: bounds.x,
                            y: bounds.y,
                        };
                        shape.points[1] = Point {
                            x: bounds.x + bounds.width,
                            y: bounds.y + bounds.height,
                        };
                    }
                    shape.updated_at = Utc::now();
                }
            }
            WhiteboardOperation::RotateShape { shape_id, angle } => {
                if let Some(shape) = self.shapes.get_mut(&shape_id) {
                    shape.rotation = angle;
                    shape.updated_at = Utc::now();
                }
            }
            WhiteboardOperation::BringToFront { shape_id } => {
                if let Some(shape) = self.shapes.get_mut(&shape_id) {
                    shape.z_index = self.next_z_index;
                    self.next_z_index += 1;
                    shape.updated_at = Utc::now();
                }
            }
            WhiteboardOperation::SendToBack { shape_id } => {
                let min_z = self.shapes.values().map(|s| s.z_index).min().unwrap_or(0) - 1;
                if let Some(shape) = self.shapes.get_mut(&shape_id) {
                    shape.z_index = min_z;
                    shape.updated_at = Utc::now();
                }
            }
            WhiteboardOperation::LockShape { shape_id } => {
                if let Some(shape) = self.shapes.get_mut(&shape_id) {
                    shape.locked = true;
                    shape.updated_at = Utc::now();
                }
            }
            WhiteboardOperation::UnlockShape { shape_id } => {
                if let Some(shape) = self.shapes.get_mut(&shape_id) {
                    shape.locked = false;
                    shape.updated_at = Utc::now();
                }
            }
            WhiteboardOperation::GroupShapes { .. } => {}
            WhiteboardOperation::UngroupShapes { .. } => {}
            WhiteboardOperation::Clear => {
                self.shapes.clear();
                self.selections.clear();
                self.next_z_index = 0;
            }
        }

        self.updated_at = Utc::now();

        if let Some(inv) = inverse.clone() {
            let entry = UndoRedoEntry {
                id: Uuid::new_v4(),
                operation: self.reconstruct_operation(&inv),
                inverse_operation: inv,
                user_id,
                timestamp: Utc::now(),
            };
            self.undo_stack.push(entry);
            if self.undo_stack.len() > self.max_undo_history {
                self.undo_stack.remove(0);
            }
            self.redo_stack.clear();
        }

        Ok(inverse)
    }

    pub(super) fn compute_inverse(&self, operation: &WhiteboardOperation) -> Result<Option<WhiteboardOperation>, String> {
        match operation {
            WhiteboardOperation::AddShape { shape } => {
                Ok(Some(WhiteboardOperation::DeleteShape { shape_id: shape.id }))
            }
            WhiteboardOperation::UpdateShape { shape } => {
                if let Some(old_shape) = self.shapes.get(&shape.id) {
                    Ok(Some(WhiteboardOperation::UpdateShape {
                        shape: old_shape.clone(),
                    }))
                } else {
                    Err("Shape not found".to_string())
                }
            }
            WhiteboardOperation::DeleteShape { shape_id } => {
                if let Some(shape) = self.shapes.get(shape_id) {
                    Ok(Some(WhiteboardOperation::AddShape {
                        shape: shape.clone(),
                    }))
                } else {
                    Err("Shape not found".to_string())
                }
            }
            WhiteboardOperation::MoveShapes { shape_ids, delta } => {
                Ok(Some(WhiteboardOperation::MoveShapes {
                    shape_ids: shape_ids.clone(),
                    delta: Point {
                        x: -delta.x,
                        y: -delta.y,
                    },
                }))
            }
            WhiteboardOperation::RotateShape { shape_id, .. } => {
                if let Some(shape) = self.shapes.get(shape_id) {
                    Ok(Some(WhiteboardOperation::RotateShape {
                        shape_id: *shape_id,
                        angle: shape.rotation,
                    }))
                } else {
                    Ok(None)
                }
            }
            WhiteboardOperation::LockShape { shape_id } => {
                Ok(Some(WhiteboardOperation::UnlockShape {
                    shape_id: *shape_id,
                }))
            }
            WhiteboardOperation::UnlockShape { shape_id } => {
                Ok(Some(WhiteboardOperation::LockShape {
                    shape_id: *shape_id,
                }))
            }
            WhiteboardOperation::Clear => {
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    pub(super) fn reconstruct_operation(&self, inverse: &WhiteboardOperation) -> WhiteboardOperation {
        inverse.clone()
    }

    pub fn undo(&mut self, user_id: Uuid) -> Option<WhiteboardOperation> {
        let user_entries: Vec<_> = self
            .undo_stack
            .iter()
            .enumerate()
            .filter(|(_, e)| e.user_id == user_id)
            .map(|(i, _)| i)
            .collect();

        if let Some(&last_idx) = user_entries.last() {
            let entry = self.undo_stack.remove(last_idx);
            let inverse = entry.inverse_operation.clone();
            let _ = self.apply_operation_without_history(inverse.clone());
            self.redo_stack.push(entry);
            Some(inverse)
        } else {
            None
        }
    }

    pub fn redo(&mut self, user_id: Uuid) -> Option<WhiteboardOperation> {
        let user_entries: Vec<_> = self
            .redo_stack
            .iter()
            .enumerate()
            .filter(|(_, e)| e.user_id == user_id)
            .map(|(i, _)| i)
            .collect();

        if let Some(&last_idx) = user_entries.last() {
            let entry = self.redo_stack.remove(last_idx);
            let operation = entry.operation.clone();
            let _ = self.apply_operation_without_history(operation.clone());
            self.undo_stack.push(entry);
            Some(operation)
        } else {
            None
        }
    }

    pub(super) fn apply_operation_without_history(&mut self, operation: WhiteboardOperation) -> Result<(), String> {
        match operation {
            WhiteboardOperation::AddShape { shape } => {
                self.shapes.insert(shape.id, shape);
            }
            WhiteboardOperation::UpdateShape { shape } => {
                self.shapes.insert(shape.id, shape);
            }
            WhiteboardOperation::DeleteShape { shape_id } => {
                self.shapes.remove(&shape_id);
            }
            WhiteboardOperation::MoveShapes { shape_ids, delta } => {
                for shape_id in &shape_ids {
                    if let Some(shape) = self.shapes.get_mut(shape_id) {
                        for point in &mut shape.points {
                            point.x += delta.x;
                            point.y += delta.y;
                        }
                    }
                }
            }
            _ => {}
        }
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn update_cursor(&mut self, cursor: CursorPosition) {
        self.cursors.insert(cursor.user_id, cursor);
    }

    pub fn remove_cursor(&mut self, user_id: &Uuid) {
        self.cursors.remove(user_id);
    }

    pub fn set_selection(&mut self, selection: Selection) {
        self.selections.insert(selection.user_id, selection);
    }

    pub fn clear_selection(&mut self, user_id: &Uuid) {
        self.selections.remove(user_id);
    }

    pub fn get_shapes_sorted(&self) -> Vec<&Shape> {
        let mut shapes: Vec<_> = self.shapes.values().collect();
        shapes.sort_by_key(|s| s.z_index);
        shapes
    }

    pub fn to_sync_message(&self) -> WhiteboardMessage {
        WhiteboardMessage::FullSync {
            shapes: self.shapes.values().cloned().collect(),
            cursors: self.cursors.values().cloned().collect(),
            selections: self.selections.values().cloned().collect(),
        }
    }
}
