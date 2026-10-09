use super::*;

impl Default for ShapeStyle {
    fn default() -> Self {
        ShapeStyle {
            stroke_color: Color::default(),
            fill_color: None,
            stroke_width: 2.0,
            stroke_style: StrokeStyle::Solid,
            opacity: 1.0,
            font_size: None,
            font_family: None,
        }
    }
}
