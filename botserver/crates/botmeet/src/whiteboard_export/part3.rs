use super::*;

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: ExportFormat::Png,
            width: 1920,
            height: 1080,
            background_color: Some("#ffffff".to_string()),
            include_grid: false,
            scale: 1.0,
            padding: 20,
            quality: 90,
            include_metadata: false,
            selected_shapes_only: false,
            selected_shape_ids: Vec::new(),
        }
    }
}
