use super::*;

impl WhiteboardExportService {
    pub fn new() -> Self {
        Self {
            export_history: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn export(
        &self,
        whiteboard: &WhiteboardData,
        options: ExportOptions,
        user_id: Uuid,
        user_name: &str,
    ) -> Result<ExportResult, ExportError> {
        let shapes = if options.selected_shapes_only {
            whiteboard
                .shapes
                .iter()
                .filter(|s| options.selected_shape_ids.contains(&s.id))
                .cloned()
                .collect()
        } else {
            whiteboard.shapes.clone()
        };

        let bounds = self.calculate_bounds(&shapes, &options);

        let (data, content_type, extension) = match options.format {
            ExportFormat::Png => {
                let png_data = self.render_to_png(&shapes, &bounds, &options)?;
                (png_data, "image/png".to_string(), "png")
            }
            ExportFormat::Pdf => {
                let pdf_data = self.render_to_pdf(&shapes, &bounds, &options, whiteboard)?;
                (pdf_data, "application/pdf".to_string(), "pdf")
            }
            ExportFormat::Svg => {
                let svg_data = self.render_to_svg(&shapes, &bounds, &options)?;
                (svg_data.into_bytes(), "image/svg+xml".to_string(), "svg")
            }
            ExportFormat::Json => {
                let json_data = self.export_to_json(whiteboard, &shapes)?;
                (json_data.into_bytes(), "application/json".to_string(), "json")
            }
        };

        let file_name = format!(
            "{}_{}.{}",
            sanitize_filename(&whiteboard.name),
            Utc::now().format("%Y%m%d_%H%M%S"),
            extension
        );

        let result = ExportResult {
            id: Uuid::new_v4(),
            whiteboard_id: whiteboard.id,
            format: options.format.clone(),
            file_name,
            file_size: data.len() as u64,
            content_type,
            data,
            created_at: Utc::now(),
            created_by: user_id,
            metadata: ExportMetadata {
                whiteboard_name: whiteboard.name.clone(),
                shape_count: shapes.len() as u32,
                export_dimensions: (bounds.width as u32, bounds.height as u32),
                original_dimensions: (whiteboard.width, whiteboard.height),
                exported_by: user_name.to_string(),
                export_time: Utc::now(),
            },
        };

        let mut history = self.export_history.write().await;
        history
            .entry(whiteboard.id)
            .or_default()
            .push(result.clone());

        Ok(result)
    }

    pub(super) fn calculate_bounds(&self, shapes: &[WhiteboardShape], options: &ExportOptions) -> ExportBounds {
        if shapes.is_empty() {
            return ExportBounds {
                min_x: 0.0,
                min_y: 0.0,
                max_x: options.width as f64,
                max_y: options.height as f64,
                width: options.width as f64,
                height: options.height as f64,
            };
        }

        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for shape in shapes {
            let shape_bounds = self.get_shape_bounds(shape);
            min_x = min_x.min(shape_bounds.0);
            min_y = min_y.min(shape_bounds.1);
            max_x = max_x.max(shape_bounds.2);
            max_y = max_y.max(shape_bounds.3);
        }

        let padding = options.padding as f64;
        min_x -= padding;
        min_y -= padding;
        max_x += padding;
        max_y += padding;

        let width = (max_x - min_x) * options.scale as f64;
        let height = (max_y - min_y) * options.scale as f64;

        ExportBounds {
            min_x,
            min_y,
            max_x,
            max_y,
            width,
            height,
        }
    }

    pub(super) fn get_shape_bounds(&self, shape: &WhiteboardShape) -> (f64, f64, f64, f64) {
        match shape.shape_type {
            ShapeType::Freehand | ShapeType::Line | ShapeType::Arrow => {
                if shape.points.is_empty() {
                    return (shape.x, shape.y, shape.x + shape.width, shape.y + shape.height);
                }
                let min_x = shape.points.iter().map(|p| p.x).fold(f64::MAX, f64::min);
                let min_y = shape.points.iter().map(|p| p.y).fold(f64::MAX, f64::min);
                let max_x = shape.points.iter().map(|p| p.x).fold(f64::MIN, f64::max);
                let max_y = shape.points.iter().map(|p| p.y).fold(f64::MIN, f64::max);
                (min_x, min_y, max_x, max_y)
            }
            _ => (shape.x, shape.y, shape.x + shape.width, shape.y + shape.height),
        }
    }

    pub(super) fn render_to_png(
        &self,
        shapes: &[WhiteboardShape],
        bounds: &ExportBounds,
        options: &ExportOptions,
    ) -> Result<Vec<u8>, ExportError> {
        let width = bounds.width.max(1.0) as u32;
        let height = bounds.height.max(1.0) as u32;

        let mut pixels = vec![255u8; (width * height * 4) as usize];

        if let Some(bg_color) = &options.background_color {
            let (r, g, b) = parse_hex_color(bg_color).unwrap_or((255, 255, 255));
            for chunk in pixels.chunks_mut(4) {
                chunk[0] = r;
                chunk[1] = g;
                chunk[2] = b;
                chunk[3] = 255;
            }
        }

        let mut sorted_shapes = shapes.to_vec();
        sorted_shapes.sort_by_key(|s| s.z_index);

        for shape in &sorted_shapes {
            self.render_shape_to_pixels(shape, &mut pixels, width, height, bounds, options);
        }

        let mut png_data = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png_data, width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);

            if let Ok(mut writer) = encoder.write_header() {
                let _ = writer.write_image_data(&pixels);
            }
        }

        if png_data.is_empty() {
            png_data = self.create_placeholder_png(width, height, options)?;
        }

        Ok(png_data)
    }

    pub(super) fn render_shape_to_pixels(
        &self,
        shape: &WhiteboardShape,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        bounds: &ExportBounds,
        options: &ExportOptions,
    ) {
        let scale = options.scale as f64;
        let offset_x = bounds.min_x;
        let offset_y = bounds.min_y;

        let x = ((shape.x - offset_x) * scale) as i32;
        let y = ((shape.y - offset_y) * scale) as i32;
        let w = (shape.width * scale) as i32;
        let h = (shape.height * scale) as i32;

        let fill = shape
            .fill_color
            .as_ref()
            .and_then(|c| parse_hex_color(c));

        let stroke = shape
            .stroke_color
            .as_ref()
            .and_then(|c| parse_hex_color(c))
            .unwrap_or((0, 0, 0));

        let alpha = (shape.opacity * 255.0) as u8;
        let area = CanvasRect { x, y, w, h };

        match shape.shape_type {
            ShapeType::Rectangle | ShapeType::Sticky => {
                if let Some((r, g, b)) = fill {
                    self.fill_rect(pixels, width, height, area, Rgba { r, g, b, a: alpha });
                }
                self.draw_rect_outline(
                    pixels,
                    width,
                    height,
                    area,
                    Rgba { r: stroke.0, g: stroke.1, b: stroke.2, a: alpha },
                );
            }
            ShapeType::Ellipse => {
                if let Some((r, g, b)) = fill {
                    self.fill_ellipse(pixels, width, height, area, Rgba { r, g, b, a: alpha });
                }
            }
            ShapeType::Line | ShapeType::Arrow | ShapeType::Freehand => {
                for i in 0..shape.points.len().saturating_sub(1) {
                    let p1 = &shape.points[i];
                    let p2 = &shape.points[i + 1];
                    let x1 = ((p1.x - offset_x) * scale) as i32;
                    let y1 = ((p1.y - offset_y) * scale) as i32;
                    let x2 = ((p2.x - offset_x) * scale) as i32;
                    let y2 = ((p2.y - offset_y) * scale) as i32;
                    self.draw_line(
                        pixels,
                        width,
                        height,
                        CanvasPoint { x: x1, y: y1 },
                        CanvasPoint { x: x2, y: y2 },
                        Rgba { r: stroke.0, g: stroke.1, b: stroke.2, a: alpha },
                    );
                }
            }
            _ => {
                if let Some((r, g, b)) = fill {
                    self.fill_rect(pixels, width, height, area, Rgba { r, g, b, a: alpha });
                }
            }
        }
    }

    pub(super) fn fill_rect(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        area: CanvasRect,
        color: Rgba,
    ) {
        let CanvasRect { x, y, w, h } = area;
        let Rgba { r, g, b, a } = color;
        let x_start = x.max(0) as u32;
        let y_start = y.max(0) as u32;
        let x_end = ((x + w) as u32).min(width);
        let y_end = ((y + h) as u32).min(height);

        for py in y_start..y_end {
            for px in x_start..x_end {
                let idx = ((py * width + px) * 4) as usize;
                if idx + 3 < pixels.len() {
                    self.blend_pixel(pixels, idx, r, g, b, a);
                }
            }
        }
    }

    pub(super) fn draw_rect_outline(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        area: CanvasRect,
        color: Rgba,
    ) {
        let CanvasRect { x, y, w, h } = area;
        self.draw_line(pixels, width, height, CanvasPoint { x, y }, CanvasPoint { x: x + w, y }, color);
        self.draw_line(
            pixels,
            width,
            height,
            CanvasPoint { x: x + w, y },
            CanvasPoint { x: x + w, y: y + h },
            color,
        );
        self.draw_line(
            pixels,
            width,
            height,
            CanvasPoint { x: x + w, y: y + h },
            CanvasPoint { x, y: y + h },
            color,
        );
        self.draw_line(pixels, width, height, CanvasPoint { x, y: y + h }, CanvasPoint { x, y }, color);
    }
}
