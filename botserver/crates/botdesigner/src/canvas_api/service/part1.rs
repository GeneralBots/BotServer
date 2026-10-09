use super::*;

impl CanvasService {
    pub fn new(pool: Arc<DbPool>) -> Self {
        let (event_sender, _) = broadcast::channel(1000);
        Self { pool, event_sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<CanvasEvent> {
        self.event_sender.subscribe()
    }

    pub async fn create_canvas(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        request: CreateCanvasRequest,
    ) -> Result<Canvas, CanvasError> {
        let mut conn = self.pool.get().map_err(|e| {
            log::error!("Failed to get database connection: {e}");
            CanvasError::DatabaseConnection
        })?;

        let id = Uuid::new_v4();
        let width = request.width.unwrap_or(1920.0);
        let height = request.height.unwrap_or(1080.0);

        let default_layer = Layer {
            id: Uuid::new_v4(),
            name: "Layer 1".to_string(),
            visible: true,
            locked: false,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            z_index: 0,
        };

        let elements: Vec<CanvasElement> = Vec::new();
        let layers = vec![default_layer.clone()];

        let elements_json = serde_json::to_string(&elements).unwrap_or_else(|_| "[]".to_string());
        let layers_json = serde_json::to_string(&layers).unwrap_or_else(|_| "[]".to_string());

        let sql = r#"
            INSERT INTO designer_canvases (
                id, organization_id, name, description, width, height,
                background_color, grid_enabled, grid_size, snap_to_grid, zoom_level,
                elements_json, layers_json, created_by, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, '#ffffff', TRUE, 10, TRUE, 1.0,
                $7, $8, $9, NOW(), NOW()
            )
        "#;

        diesel::sql_query(sql)
            .bind::<DieselUuid, _>(id)
            .bind::<DieselUuid, _>(organization_id)
            .bind::<Text, _>(&request.name)
            .bind::<diesel::sql_types::Nullable<Text>, _>(request.description.as_deref())
            .bind::<diesel::sql_types::Double, _>(width)
            .bind::<diesel::sql_types::Double, _>(height)
            .bind::<Text, _>(&elements_json)
            .bind::<Text, _>(&layers_json)
            .bind::<DieselUuid, _>(user_id)
            .execute(&mut conn)
            .map_err(|e| {
                log::error!("Failed to create canvas: {e}");
                CanvasError::CreateFailed
            })?;

        log::info!("Created canvas {} for org {}", id, organization_id);

        Ok(Canvas {
            id,
            organization_id,
            name: request.name,
            description: request.description,
            width,
            height,
            background_color: "#ffffff".to_string(),
            grid_enabled: true,
            grid_size: 10,
            snap_to_grid: true,
            zoom_level: 1.0,
            elements,
            layers,
            created_by: user_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    pub async fn get_canvas(&self, canvas_id: Uuid) -> Result<Canvas, CanvasError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|_| CanvasError::DatabaseConnection)?;

        let sql = r#"
            SELECT id, organization_id, name, description, width, height,
            background_color, grid_enabled, grid_size, snap_to_grid, zoom_level,
            elements_json, layers_json, created_by, created_at, updated_at
            FROM designer_canvases WHERE id = $1
        "#;

        let rows: Vec<CanvasRow> = diesel::sql_query(sql)
            .bind::<DieselUuid, _>(canvas_id)
            .load(&mut conn)
            .map_err(|e| {
                log::error!("Failed to get canvas: {e}");
                CanvasError::DatabaseConnection
            })?;

        let row = rows.into_iter().next().ok_or(CanvasError::NotFound)?;
        Ok(row_to_canvas(row))
    }

    pub async fn add_element(
        &self,
        canvas_id: Uuid,
        user_id: Uuid,
        request: AddElementRequest,
    ) -> Result<CanvasElement, CanvasError> {
        let mut canvas = self.get_canvas(canvas_id).await?;

        let layer_id =
            request
                .layer_id
                .unwrap_or_else(|| canvas.layers.first().map(|l| l.id).unwrap_or_else(Uuid::new_v4));

        let max_z = canvas.elements.iter().map(|e| e.z_index).max().unwrap_or(0);

        let element = CanvasElement {
            id: Uuid::new_v4(),
            element_type: request.element_type,
            layer_id,
            x: request.x,
            y: request.y,
            width: request.width,
            height: request.height,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            opacity: 1.0,
            visible: true,
            locked: false,
            name: None,
            style: request.style.unwrap_or_default(),
            properties: request.properties.unwrap_or_default(),
            z_index: max_z + 1,
            parent_id: None,
            children: Vec::new(),
        };

        canvas.elements.push(element.clone());
        self.save_canvas_elements(canvas_id, &canvas.elements)
            .await?;

        self.broadcast_event(
            CanvasEventType::ElementAdded,
            canvas_id,
            user_id,
            serde_json::json!({
                "element_id": element.id,
                "element_type": element.element_type.to_string()
            }),
        );

        Ok(element)
    }

    pub async fn update_element(
        &self,
        canvas_id: Uuid,
        element_id: Uuid,
        user_id: Uuid,
        request: UpdateElementRequest,
    ) -> Result<CanvasElement, CanvasError> {
        let mut canvas = self.get_canvas(canvas_id).await?;

        let element = canvas
            .elements
            .iter_mut()
            .find(|e| e.id == element_id)
            .ok_or(CanvasError::ElementNotFound)?;

        if element.locked {
            return Err(CanvasError::ElementLocked);
        }

        if let Some(x) = request.x {
            element.x = x;
        }
        if let Some(y) = request.y {
            element.y = y;
        }
        if let Some(w) = request.width {
            element.width = w;
        }
        if let Some(h) = request.height {
            element.height = h;
        }
        if let Some(r) = request.rotation {
            element.rotation = r;
        }
        if let Some(sx) = request.scale_x {
            element.scale_x = sx;
        }
        if let Some(sy) = request.scale_y {
            element.scale_y = sy;
        }
        if let Some(o) = request.opacity {
            element.opacity = o;
        }
        if let Some(v) = request.visible {
            element.visible = v;
        }
        if let Some(l) = request.locked {
            element.locked = l;
        }
        if let Some(n) = request.name {
            element.name = Some(n);
        }
        if let Some(s) = request.style {
            element.style = s;
        }
        if let Some(p) = request.properties {
            element.properties = p;
        }
        if let Some(z) = request.z_index {
            element.z_index = z;
        }
        if let Some(lid) = request.layer_id {
            element.layer_id = lid;
        }

        let updated_element = element.clone();
        self.save_canvas_elements(canvas_id, &canvas.elements)
            .await?;

        self.broadcast_event(
            CanvasEventType::ElementUpdated,
            canvas_id,
            user_id,
            serde_json::json!({ "element_id": element_id }),
        );

        Ok(updated_element)
    }

    pub async fn delete_element(
        &self,
        canvas_id: Uuid,
        element_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), CanvasError> {
        let mut canvas = self.get_canvas(canvas_id).await?;

        let idx = canvas
            .elements
            .iter()
            .position(|e| e.id == element_id)
            .ok_or(CanvasError::ElementNotFound)?;

        if canvas.elements[idx].locked {
            return Err(CanvasError::ElementLocked);
        }

        canvas.elements.remove(idx);
        self.save_canvas_elements(canvas_id, &canvas.elements)
            .await?;

        self.broadcast_event(
            CanvasEventType::ElementDeleted,
            canvas_id,
            user_id,
            serde_json::json!({ "element_id": element_id }),
        );

        Ok(())
    }
}
