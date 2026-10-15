use super::*;

impl WhiteboardExportService {
    pub(super) fn fill_ellipse(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        area: CanvasRect,
        color: Rgba,
    ) {
        let CanvasRect { x, y, w, h } = area;
        let Rgba { r, g, b, a } = color;
        let cx = x + w / 2;
        let cy = y + h / 2;
        let rx = w / 2;
        let ry = h / 2;

        if rx <= 0 || ry <= 0 {
            return;
        }

        let x_start = (x.max(0)) as u32;
        let y_start = (y.max(0)) as u32;
        let x_end = ((x + w) as u32).min(width);
        let y_end = ((y + h) as u32).min(height);

        for py in y_start..y_end {
            for px in x_start..x_end {
                let dx = (px as i32 - cx) as f64 / rx as f64;
                let dy = (py as i32 - cy) as f64 / ry as f64;
                if dx * dx + dy * dy <= 1.0 {
                    let idx = ((py * width + px) * 4) as usize;
                    if idx + 3 < pixels.len() {
                        self.blend_pixel(pixels, idx, r, g, b, a);
                    }
                }
            }
        }
    }

    pub(super) fn draw_line(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        from: CanvasPoint,
        to: CanvasPoint,
        color: Rgba,
    ) {
        let CanvasPoint { x: x0, y: y0 } = from;
        let CanvasPoint { x: x1, y: y1 } = to;
        let Rgba { r, g, b, a } = color;
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let mut x = x0;
        let mut y = y0;

        loop {
            if x >= 0 && y >= 0 && (x as u32) < width && (y as u32) < height {
                let idx = ((y as u32 * width + x as u32) * 4) as usize;
                if idx + 3 < pixels.len() {
                    self.blend_pixel(pixels, idx, r, g, b, a);
                }
            }

            if x == x1 && y == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    pub(super) fn blend_pixel(&self, pixels: &mut [u8], idx: usize, r: u8, g: u8, b: u8, a: u8) {
        let alpha = a as f32 / 255.0;
        let inv_alpha = 1.0 - alpha;

        pixels[idx] = (r as f32 * alpha + pixels[idx] as f32 * inv_alpha) as u8;
        pixels[idx + 1] = (g as f32 * alpha + pixels[idx + 1] as f32 * inv_alpha) as u8;
        pixels[idx + 2] = (b as f32 * alpha + pixels[idx + 2] as f32 * inv_alpha) as u8;
        pixels[idx + 3] = 255;
    }

    pub(super) fn create_placeholder_png(
        &self,
        width: u32,
        height: u32,
        options: &ExportOptions,
    ) -> Result<Vec<u8>, ExportError> {
        let mut pixels = vec![255u8; (width * height * 4) as usize];

        if let Some(bg) = &options.background_color {
            if let Some((r, g, b)) = parse_hex_color(bg) {
                for chunk in pixels.chunks_mut(4) {
                    chunk[0] = r;
                    chunk[1] = g;
                    chunk[2] = b;
                    chunk[3] = 255;
                }
            }
        }

        let mut png_data = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png_data, width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);

            let mut writer = encoder
                .write_header()
                .map_err(|e| ExportError::RenderError(e.to_string()))?;

            writer
                .write_image_data(&pixels)
                .map_err(|e| ExportError::RenderError(e.to_string()))?;
        }

        Ok(png_data)
    }

    pub(super) fn render_to_pdf(
        &self,
        shapes: &[WhiteboardShape],
        bounds: &ExportBounds,
        options: &ExportOptions,
        whiteboard: &WhiteboardData,
    ) -> Result<Vec<u8>, ExportError> {
        let mut pdf = PdfDocument::new(&whiteboard.name);

        let page_width = bounds.width.max(595.0) as f32;
        let page_height = bounds.height.max(842.0) as f32;

        pdf.add_page(page_width, page_height);

        if let Some(bg_color) = &options.background_color {
            pdf.set_fill_color(bg_color);
            pdf.draw_rect(0.0, 0.0, page_width, page_height, true, false);
        }

        let mut sorted_shapes = shapes.to_vec();
        sorted_shapes.sort_by_key(|s| s.z_index);

        for shape in &sorted_shapes {
            self.render_shape_to_pdf(&mut pdf, shape, bounds, options);
        }

        if options.include_metadata {
            pdf.add_metadata(&whiteboard.name, &Utc::now().to_rfc3339());
        }

        Ok(pdf.to_bytes())
    }

    pub(super) fn render_shape_to_pdf(
        &self,
        pdf: &mut PdfDocument,
        shape: &WhiteboardShape,
        bounds: &ExportBounds,
        options: &ExportOptions,
    ) {
        let scale = options.scale as f64;
        let x = ((shape.x - bounds.min_x) * scale) as f32;
        let y = ((shape.y - bounds.min_y) * scale) as f32;
        let w = (shape.width * scale) as f32;
        let h = (shape.height * scale) as f32;

        if let Some(fill) = &shape.fill_color {
            pdf.set_fill_color(fill);
        }
        if let Some(stroke) = &shape.stroke_color {
            pdf.set_stroke_color(stroke);
        }
        pdf.set_line_width(shape.stroke_width);

        match shape.shape_type {
            ShapeType::Rectangle | ShapeType::Sticky => {
                pdf.draw_rect(x, y, w, h, shape.fill_color.is_some(), shape.stroke_color.is_some());
            }
            ShapeType::Ellipse => {
                pdf.draw_ellipse(x + w / 2.0, y + h / 2.0, w / 2.0, h / 2.0, shape.fill_color.is_some(), shape.stroke_color.is_some());
            }
            ShapeType::Line | ShapeType::Arrow | ShapeType::Freehand => {
                if !shape.points.is_empty() {
                    let points: Vec<(f32, f32)> = shape
                        .points
                        .iter()
                        .map(|p| {
                            (((p.x - bounds.min_x) * scale) as f32, ((p.y - bounds.min_y) * scale) as f32)
                        })
                        .collect();
                    pdf.draw_path(&points);
                }
            }
            ShapeType::Text => {
                if let Some(text) = &shape.text {
                    let font_size = shape.font_size.unwrap_or(12.0) * options.scale ;
                    pdf.draw_text(text, x, y, font_size);
                }
            }
            ShapeType::Triangle => {
                let points: Vec<(f32, f32)> = vec![
                    (x + w / 2.0, y),
                    (x + w, y + h),
                    (x, y + h),
                    (x + w / 2.0, y),
                ];
                pdf.draw_path(&points);
            }
            ShapeType::Diamond => {
                let points: Vec<(f32, f32)> = vec![
                    (x + w / 2.0, y),
                    (x + w, y + h / 2.0),
                    (x + w / 2.0, y + h),
                    (x, y + h / 2.0),
                    (x + w / 2.0, y),
                ];
                pdf.draw_path(&points);
            }
            _ => {
                pdf.draw_rect(x, y, w, h, shape.fill_color.is_some(), shape.stroke_color.is_some());
            }
        }
    }

    pub(super) fn render_to_svg(
        &self,
        shapes: &[WhiteboardShape],
        bounds: &ExportBounds,
        options: &ExportOptions,
    ) -> Result<String, ExportError> {
        let width = bounds.width.max(1.0);
        let height = bounds.height.max(1.0);

        let mut svg = String::new();
        svg.push_str(&format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">"#,
            width, height, width, height
        ));

        svg.push_str(r#"
<defs>
    <style>
        .shape { transition: all 0.2s ease; }
        .shape:hover { filter: brightness(1.1); }
    </style>
</defs>"#);

        if let Some(bg_color) = &options.background_color {
            svg.push_str(&format!(
                r#"<rect width="100%" height="100%" fill="{}"/>"#,
                bg_color
            ));
        }

        if options.include_grid {
            svg.push_str(&self.generate_svg_grid(width, height));
        }

        let mut sorted_shapes = shapes.to_vec();
        sorted_shapes.sort_by_key(|s| s.z_index);

        for shape in &sorted_shapes {
            svg.push_str(&self.shape_to_svg(shape, bounds, options));
        }

        svg.push_str("</svg>");

        Ok(svg)
    }
}
