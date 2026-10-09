use super::*;

impl CanvasService {
    pub async fn group_elements(
        &self,
        canvas_id: Uuid,
        user_id: Uuid,
        request: GroupElementsRequest,
    ) -> Result<CanvasElement, CanvasError> {
        let mut canvas = self.get_canvas(canvas_id).await?;

        let elements_to_group: Vec<&CanvasElement> = canvas
            .elements
            .iter()
            .filter(|e| request.element_ids.contains(&e.id))
            .collect();

        if elements_to_group.is_empty() {
            return Err(CanvasError::InvalidInput(
                "No elements to group".to_string(),
            ));
        }

        let min_x = elements_to_group
            .iter()
            .map(|e| e.x)
            .fold(f64::INFINITY, f64::min);
        let min_y = elements_to_group
            .iter()
            .map(|e| e.y)
            .fold(f64::INFINITY, f64::min);
        let max_x = elements_to_group
            .iter()
            .map(|e| e.x + e.width)
            .fold(f64::NEG_INFINITY, f64::max);
        let max_y = elements_to_group
            .iter()
            .map(|e| e.y + e.height)
            .fold(f64::NEG_INFINITY, f64::max);

        let group_id = Uuid::new_v4();
        let layer_id = elements_to_group
            .first()
            .map(|e| e.layer_id)
            .unwrap_or_else(Uuid::new_v4);
        let max_z = canvas.elements.iter().map(|e| e.z_index).max().unwrap_or(0);

        for element in canvas.elements.iter_mut() {
            if request.element_ids.contains(&element.id) {
                element.parent_id = Some(group_id);
            }
        }

        let group = CanvasElement {
            id: group_id,
            element_type: ElementType::Group,
            layer_id,
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            opacity: 1.0,
            visible: true,
            locked: false,
            name: request.name,
            style: ElementStyle::default(),
            properties: ElementProperties::default(),
            z_index: max_z + 1,
            parent_id: None,
            children: request.element_ids.clone(),
        };

        canvas.elements.push(group.clone());
        self.save_canvas_elements(canvas_id, &canvas.elements)
            .await?;

        self.broadcast_event(
            CanvasEventType::ElementsGrouped,
            canvas_id,
            user_id,
            serde_json::json!({
                "group_id": group_id,
                "element_ids": request.element_ids
            }),
        );

        Ok(group)
    }

    pub async fn add_layer(
        &self,
        canvas_id: Uuid,
        user_id: Uuid,
        request: CreateLayerRequest,
    ) -> Result<Layer, CanvasError> {
        let mut canvas = self.get_canvas(canvas_id).await?;

        let max_z = canvas.layers.iter().map(|l| l.z_index).max().unwrap_or(0);

        let layer = Layer {
            id: Uuid::new_v4(),
            name: request.name,
            visible: true,
            locked: false,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            z_index: request.z_index.unwrap_or(max_z + 1),
        };

        canvas.layers.push(layer.clone());
        self.save_canvas_layers(canvas_id, &canvas.layers).await?;

        self.broadcast_event(
            CanvasEventType::LayerAdded,
            canvas_id,
            user_id,
            serde_json::json!({ "layer_id": layer.id }),
        );

        Ok(layer)
    }

    pub async fn export_canvas(
        &self,
        canvas_id: Uuid,
        request: ExportRequest,
    ) -> Result<ExportResult, CanvasError> {
        let canvas = self.get_canvas(canvas_id).await?;

        let scale = request.scale.unwrap_or(1.0);
        let width = canvas.width * scale;
        let height = canvas.height * scale;

        let (data, content_type, ext) = match request.format {
            ExportFormat::Svg => {
                let svg = self.generate_svg(&canvas, &request)?;
                (svg, "image/svg+xml", "svg")
            }
            ExportFormat::Html => {
                let html = self.generate_html(&canvas, &request)?;
                (html, "text/html", "html")
            }
            ExportFormat::Png | ExportFormat::Jpg | ExportFormat::Pdf => {
                let svg = self.generate_svg(&canvas, &request)?;
                (svg, "image/svg+xml", "svg")
            }
        };

        Ok(ExportResult {
            format: request.format,
            data,
            content_type: content_type.to_string(),
            filename: format!("{}.{}", canvas.name, ext),
            width,
            height,
        })
    }

    pub(super) fn generate_svg(
        &self,
        canvas: &Canvas,
        request: &ExportRequest,
    ) -> Result<String, CanvasError> {
        let scale = request.scale.unwrap_or(1.0);
        let width = canvas.width * scale;
        let height = canvas.height * scale;

        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">"#,
            width, height, canvas.width, canvas.height
        );

        if request.background.unwrap_or(true) {
            svg.push_str(&format!(
                r#"<rect width="100%" height="100%" fill="{}"/>"#,
                canvas.background_color
            ));
        }

        let mut sorted_elements = canvas.elements.clone();
        sorted_elements.sort_by_key(|e| e.z_index);

        for element in sorted_elements.iter().filter(|e| e.visible) {
            svg.push_str(&self.element_to_svg(element));
        }

        svg.push_str("</svg>");
        Ok(svg)
    }

    pub(super) fn element_to_svg(&self, element: &CanvasElement) -> String {
        let transform =
            if element.rotation != 0.0 || element.scale_x != 1.0 || element.scale_y != 1.0 {
                format!(
                    r#" transform="translate({},{}) rotate({}) scale({},{})""#,
                    element.x + element.width / 2.0,
                    element.y + element.height / 2.0,
                    element.rotation,
                    element.scale_x,
                    element.scale_y
                )
            } else {
                String::new()
            };

        let opacity = if element.opacity < 1.0 {
            format!(r#" opacity="{}""#, element.opacity)
        } else {
            String::new()
        };

        let fill = element
            .style
            .fill
            .as_ref()
            .map(|f| match f.fill_type {
                FillType::Solid => f.color.clone().unwrap_or_else(|| "#000000".to_string()),
                FillType::None => "none".to_string(),
                _ => "#000000".to_string(),
            })
            .unwrap_or_else(|| "#000000".to_string());

        let stroke = element
            .style
            .stroke
            .as_ref()
            .map(|s| format!(r#" stroke="{}" stroke-width="{}""#, s.color, s.width))
            .unwrap_or_default();

        match element.element_type {
            ElementType::Rectangle => {
                let rx = element
                    .style
                    .border_radius
                    .as_ref()
                    .map(|r| r.top_left)
                    .unwrap_or(0.0);
                format!(
                    r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="{}"{}{}{}/>"#,
                    element.x, element.y, element.width, element.height, rx, fill, stroke, opacity, transform
                )
            }
            ElementType::Ellipse => {
                format!(
                    r#"<ellipse cx="{}" cy="{}" rx="{}" ry="{}" fill="{}"{}{}{}/>"#,
                    element.x + element.width / 2.0,
                    element.y + element.height / 2.0,
                    element.width / 2.0,
                    element.height / 2.0,
                    fill, stroke, opacity, transform
                )
            }
            ElementType::Line => {
                format!(
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}"{}{}{}/>"#,
                    element.x, element.y, element.x + element.width, element.y + element.height, stroke, opacity, transform
                )
            }
            ElementType::Text => {
                let text = element.properties.text_content.as_deref().unwrap_or("");
                let font_size = element.properties.font_size.unwrap_or(16.0);
                let font_family = element.properties.font_family.as_deref().unwrap_or("sans-serif");
                let text_color = element.properties.text_color.as_deref().unwrap_or("#000000");
                format!(
                    r#"<text x="{}" y="{}" font-size="{}" font-family="{}" fill="{}"{}{}>{}</text>"#,
                    element.x, element.y + font_size, font_size, font_family, text_color, opacity, transform, text
                )
            }
            ElementType::Image => {
                let url = element.properties.image_url.as_deref().unwrap_or("");
                format!(
                    r#"<image x="{}" y="{}" width="{}" height="{}" href="{}"{}{}/>"#,
                    element.x, element.y, element.width, element.height, url, opacity, transform
                )
            }
            ElementType::Svg => element.properties.svg_content.clone().unwrap_or_default(),
            ElementType::Path => {
                let d = element.properties.path_data.as_deref().unwrap_or("");
                format!(
                    r#"<path d="{}" fill="{}"{}{}{}/>"#,
                    d, fill, stroke, opacity, transform
                )
            }
            _ => String::new(),
        }
    }
}
