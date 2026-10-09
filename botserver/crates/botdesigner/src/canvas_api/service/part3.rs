use super::*;

impl CanvasService {
    pub(super) fn generate_html(
        &self,
        canvas: &Canvas,
        request: &ExportRequest,
    ) -> Result<String, CanvasError> {
        let svg = self.generate_svg(canvas, request)?;

        let html = format!(
            r#"<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1.0"><title>{}</title>
<style>body{{margin:0;padding:0;display:flex;justify-content:center;align-items:center;min-height:100vh;background:#f0f0f0}}
.canvas-container{{background:white;box-shadow:0 4px 20px rgba(0,0,0,0.1)}}</style></head>
<body><div class="canvas-container">{}</div></body></html>"#,
            canvas.name, svg
        );

        Ok(html)
    }

    async fn save_canvas_elements(
        &self,
        canvas_id: Uuid,
        elements: &[CanvasElement],
    ) -> Result<(), CanvasError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|_| CanvasError::DatabaseConnection)?;

        let elements_json = serde_json::to_string(elements).unwrap_or_else(|_| "[]".to_string());

        diesel::sql_query(
            "UPDATE designer_canvases SET elements_json = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind::<Text, _>(&elements_json)
        .bind::<DieselUuid, _>(canvas_id)
        .execute(&mut conn)
        .map_err(|e| {
            log::error!("Failed to save elements: {e}");
            CanvasError::UpdateFailed
        })?;

        Ok(())
    }

    async fn save_canvas_layers(
        &self,
        canvas_id: Uuid,
        layers: &[Layer],
    ) -> Result<(), CanvasError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|_| CanvasError::DatabaseConnection)?;

        let layers_json = serde_json::to_string(layers).unwrap_or_else(|_| "[]".to_string());

        diesel::sql_query(
            "UPDATE designer_canvases SET layers_json = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind::<Text, _>(&layers_json)
        .bind::<DieselUuid, _>(canvas_id)
        .execute(&mut conn)
        .map_err(|e| {
            log::error!("Failed to save layers: {e}");
            CanvasError::UpdateFailed
        })?;

        Ok(())
    }

    pub(super) fn broadcast_event(
        &self,
        event_type: CanvasEventType,
        canvas_id: Uuid,
        user_id: Uuid,
        data: serde_json::Value,
    ) {
        let event = CanvasEvent {
            event_type,
            canvas_id,
            user_id,
            data,
            timestamp: Utc::now(),
        };
        let _ = self.event_sender.send(event);
    }

    pub async fn get_templates(
        &self,
        category: Option<String>,
    ) -> Result<Vec<CanvasTemplate>, CanvasError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|_| CanvasError::DatabaseConnection)?;

        let sql = match category {
            Some(ref cat) => format!(
                "SELECT id, name, description, category, thumbnail_url, canvas_data, is_system, created_by, created_at FROM designer_templates WHERE category = '{}' ORDER BY name",
                cat
            ),
            None => "SELECT id, name, description, category, thumbnail_url, canvas_data, is_system, created_by, created_at FROM designer_templates ORDER BY category, name".to_string(),
        };

        let rows: Vec<TemplateRow> = diesel::sql_query(&sql)
            .load(&mut conn)
            .unwrap_or_default();

        let templates = rows
            .into_iter()
            .map(|row| CanvasTemplate {
                id: row.id,
                name: row.name,
                description: row.description,
                category: row.category,
                thumbnail_url: row.thumbnail_url,
                canvas_data: serde_json::from_str(&row.canvas_data)
                    .unwrap_or(serde_json::json!({})),
                is_system: row.is_system,
                created_by: row.created_by,
                created_at: row.created_at,
            })
            .collect();

        Ok(templates)
    }

    pub async fn get_asset_library(
        &self,
        asset_type: Option<AssetType>,
    ) -> Result<Vec<AssetLibraryItem>, CanvasError> {
        let icons = vec![
            AssetLibraryItem { id: Uuid::new_v4(), name: "Bot".to_string(), asset_type: AssetType::Icon, url: None, svg_content: None, category: "General Bots".to_string(), tags: vec!["bot".to_string(), "assistant".to_string()], is_system: true },
            AssetLibraryItem { id: Uuid::new_v4(), name: "Analytics".to_string(), asset_type: AssetType::Icon, url: None, svg_content: None, category: "General Bots".to_string(), tags: vec!["analytics".to_string(), "chart".to_string()], is_system: true },
            AssetLibraryItem { id: Uuid::new_v4(), name: "Calendar".to_string(), asset_type: AssetType::Icon, url: None, svg_content: None, category: "General Bots".to_string(), tags: vec!["calendar".to_string(), "date".to_string()], is_system: true },
            AssetLibraryItem { id: Uuid::new_v4(), name: "Chat".to_string(), asset_type: AssetType::Icon, url: None, svg_content: None, category: "General Bots".to_string(), tags: vec!["chat".to_string(), "message".to_string()], is_system: true },
            AssetLibraryItem { id: Uuid::new_v4(), name: "Drive".to_string(), asset_type: AssetType::Icon, url: None, svg_content: None, category: "General Bots".to_string(), tags: vec!["drive".to_string(), "files".to_string()], is_system: true },
            AssetLibraryItem { id: Uuid::new_v4(), name: "Mail".to_string(), asset_type: AssetType::Icon, url: None, svg_content: None, category: "General Bots".to_string(), tags: vec!["mail".to_string(), "email".to_string()], is_system: true },
            AssetLibraryItem { id: Uuid::new_v4(), name: "Meet".to_string(), asset_type: AssetType::Icon, url: None, svg_content: None, category: "General Bots".to_string(), tags: vec!["meet".to_string(), "video".to_string()], is_system: true },
            AssetLibraryItem { id: Uuid::new_v4(), name: "Tasks".to_string(), asset_type: AssetType::Icon, url: None, svg_content: None, category: "General Bots".to_string(), tags: vec!["tasks".to_string(), "todo".to_string()], is_system: true },
        ];

        let filtered = match asset_type {
            Some(t) => icons.into_iter().filter(|i| i.asset_type == t).collect(),
            None => icons,
        };

        Ok(filtered)
    }
}
