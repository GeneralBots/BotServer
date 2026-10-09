use super::*;

impl PdfDocument {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            pages: Vec::new(),
            fill_color: "#000000".to_string(),
            stroke_color: "#000000".to_string(),
        }
    }

    pub fn add_page(&mut self, width: f32, height: f32) {
        let _ = (width, height);
        self.pages.push(PdfPage {
        });
    }

    pub fn set_fill_color(&mut self, color: &str) {
        self.fill_color = color.to_string();
    }

    pub fn set_stroke_color(&mut self, color: &str) {
        self.stroke_color = color.to_string();
    }

    pub fn set_line_width(&mut self, _width: f32) {}

    pub fn draw_rect(&mut self, _x: f32, _y: f32, _w: f32, _h: f32, _fill: bool, _stroke: bool) {}

    pub fn draw_ellipse(&mut self, _cx: f32, _cy: f32, _rx: f32, _ry: f32, _fill: bool, _stroke: bool) {}

    pub fn draw_line(&mut self, _x1: f32, _y1: f32, _x2: f32, _y2: f32) {}

    pub fn draw_path(&mut self, _points: &[(f32, f32)]) {}

    pub fn draw_text(&mut self, _text: &str, _x: f32, _y: f32, _font_size: f32) {}

    pub fn draw_image(&mut self, _data: &[u8], _x: f32, _y: f32, _w: f32, _h: f32) {}

    pub fn add_metadata(&mut self, _title: &str, _date: &str) {}

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output = Vec::new();
        output.extend_from_slice(b"%PDF-1.4\n");
        output.extend_from_slice(format!("% {}\n", self.name).as_bytes());
        output.extend_from_slice(b"%%EOF\n");
        output
    }
}
