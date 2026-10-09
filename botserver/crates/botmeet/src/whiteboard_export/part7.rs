use super::*;

impl WhiteboardExportService {
    pub(super) fn generate_svg_grid(&self, width: f64, height: f64) -> String {
        let grid_size = 20.0;
        let mut grid = String::new();

        grid.push_str(r##"<g class="grid" stroke="#e0e0e0" stroke-width="0.5">"##);

        let mut x = 0.0;
        while x <= width {
            grid.push_str(&format!(
                r#"<line x1="{}" y1="0" x2="{}" y2="{}"/>"#,
                x, x, height
            ));
            x += grid_size;
        }

        let mut y = 0.0;
        while y <= height {
            grid.push_str(&format!(
                r#"<line x1="0" y1="{}" x2="{}" y2="{}"/>"#,
                y, width, y
            ));
            y += grid_size;
        }

        grid.push_str("</g>");
        grid
    }

    pub(super) fn shape_to_svg(
        &self,
        shape: &WhiteboardShape,
        bounds: &ExportBounds,
        options: &ExportOptions,
    ) -> String {
        let scale = options.scale as f64;
        let x = (shape.x - bounds.min_x) * scale;
        let y = (shape.y - bounds.min_y) * scale;
        let w = shape.width * scale;
        let h = shape.height * scale;

        let fill = shape
            .fill_color.as_deref()
            .unwrap_or("none");

        let stroke = shape
            .stroke_color.as_deref()
            .unwrap_or("none");

        let stroke_width = shape.stroke_width * options.scale;
        let opacity = shape.opacity;

        let transform = if shape.rotation != 0.0 {
            format!(
                r#" transform="rotate({} {} {})""#,
                shape.rotation,
                x + w / 2.0,
                y + h / 2.0
            )
        } else {
            String::new()
        };

        match shape.shape_type {
            ShapeType::Rectangle | ShapeType::Sticky => {
                format!(
                    r#"<rect class="shape" x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"{}/>"#,
                    x, y, w, h, fill, stroke, stroke_width, opacity, transform
                )
            }
            ShapeType::Ellipse => {
                format!(
                    r#"<ellipse class="shape" cx="{}" cy="{}" rx="{}" ry="{}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"{}/>"#,
                    x + w / 2.0,
                    y + h / 2.0,
                    w / 2.0,
                    h / 2.0,
                    fill,
                    stroke,
                    stroke_width,
                    opacity,
                    transform
                )
            }
            ShapeType::Line | ShapeType::Arrow | ShapeType::Freehand => {
                if shape.points.is_empty() {
                    return String::new();
                }
                let points: Vec<String> = shape
                    .points
                    .iter()
                    .map(|p| {
                        format!(
                            "{},{}",
                            (p.x - bounds.min_x) * scale,
                            (p.y - bounds.min_y) * scale
                        )
                    })
                    .collect();

                if shape.shape_type == ShapeType::Freehand {
                    let mut path = format!("M {}", points[0]);
                    for point in points.iter().skip(1) {
                        path.push_str(&format!(" L {}", point));
                    }
                    format!(
                        r#"<path class="shape" d="{}" fill="none" stroke="{}" stroke-width="{}" opacity="{}"{}/>"#,
                        path, stroke, stroke_width, opacity, transform
                    )
                } else {
                    let line_points = points.join(" ");
                    let marker = if shape.shape_type == ShapeType::Arrow {
                        r#" marker-end="url(#arrowhead)""#
                    } else {
                        ""
                    };
                    format!(
                        r#"<polyline class="shape" points="{}" fill="none" stroke="{}" stroke-width="{}" opacity="{}"{}{}/>"#,
                        line_points, stroke, stroke_width, opacity, marker, transform
                    )
                }
            }
            ShapeType::Text => {
                let font_size = f64::from(shape.font_size.unwrap_or(16.0)) * scale;
                let text_content = shape.text.as_deref().unwrap_or("");
                format!(
                    r#"<text class="shape" x="{}" y="{}" font-size="{}" fill="{}" opacity="{}"{}>{}</text>"#,
                    x, y + font_size, font_size, fill, opacity, transform, text_content
                )
            }
            ShapeType::Image => {
                if let Some(src) = &shape.image_data {
                    format!(
                        r#"<image class="shape" x="{}" y="{}" width="{}" height="{}" href="{}" opacity="{}"{}/>"#,
                        x, y, w, h, src, opacity, transform
                    )
                } else {
                    String::new()
                }
            }
            ShapeType::Connector => {
                if shape.points.len() >= 2 {
                    let points: Vec<String> = shape
                        .points
                        .iter()
                        .map(|p| {
                            format!(
                                "{},{}",
                                (p.x - bounds.min_x) * scale,
                                (p.y - bounds.min_y) * scale
                            )
                        })
                        .collect();
                    let line_points = points.join(" ");
                    format!(
                        r#"<polyline class="shape" points="{}" fill="none" stroke="{}" stroke-width="{}" opacity="{}" marker-end="url(#arrowhead)"{}/>"#,
                        line_points, stroke, stroke_width, opacity, transform
                    )
                } else {
                    String::new()
                }
            }
            ShapeType::Triangle => {
                let x1 = x + w / 2.0;
                let y1 = y;
                let x2 = x;
                let y2 = y + h;
                let x3 = x + w;
                let y3 = y + h;
                format!(
                    r#"<polygon class="shape" points="{},{} {},{} {},{}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"{}/>"#,
                    x1, y1, x2, y2, x3, y3, fill, stroke, stroke_width, opacity, transform
                )
            }
            ShapeType::Diamond => {
                let x1 = x + w / 2.0;
                let y1 = y;
                let x2 = x + w;
                let y2 = y + h / 2.0;
                let x3 = x + w / 2.0;
                let y3 = y + h;
                let x4 = x;
                let y4 = y + h / 2.0;
                format!(
                    r#"<polygon class="shape" points="{},{} {},{} {},{} {},{}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"{}/>"#,
                    x1, y1, x2, y2, x3, y3, x4, y4, fill, stroke, stroke_width, opacity, transform
                )
            }
            ShapeType::Star => {
                let cx = x + w / 2.0;
                let cy = y + h / 2.0;
                let outer_r = w.min(h) / 2.0;
                let inner_r = outer_r * 0.4;
                let mut points = Vec::new();
                for i in 0..10 {
                    let angle = std::f64::consts::PI / 2.0 - (i as f64) * std::f64::consts::PI / 5.0;
                    let r = if i % 2 == 0 { outer_r } else { inner_r };
                    let px = cx + r * angle.cos();
                    let py = cy - r * angle.sin();
                    points.push(format!("{px},{py}"));
                }
                format!(
                    r#"<polygon class="shape" points="{}" fill="{}" stroke="{}" stroke-width="{}" opacity="{}"{}/>"#,
                    points.join(" "), fill, stroke, stroke_width, opacity, transform
                )
            }
        }
    }

    pub(super) fn export_to_json(
        &self,
        whiteboard: &WhiteboardData,
        shapes: &[WhiteboardShape],
    ) -> Result<String, ExportError> {
        let export_data = serde_json::json!({
            "id": whiteboard.id,
            "name": whiteboard.name,
            "created_at": whiteboard.created_at,
            "updated_at": whiteboard.updated_at,
            "shapes": shapes,
        });

        serde_json::to_string_pretty(&export_data)
            .map_err(|e| ExportError::RenderFailed(format!("JSON serialization failed: {e}")))
    }
}
