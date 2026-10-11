//! PptxDocument build/render implementation.
//! Split from pptx.rs for file-size limits.

use super::pptx::{
    PptxDocument, PptxStyles, Slide, SlideElement, SlideElementType, SlideLayout,
    TransitionEffect,
};

impl PptxDocument {
    pub fn new() -> Self {
        Self {
            slides: Vec::new(),
            styles: PptxStyles::default(),
            author: None,
            title: None,
            subject: None,
        }
    }

    pub fn add_slide(&mut self, slide: Slide) {
        self.slides.push(slide);
    }

    pub fn add_text_slide(
        &mut self,
        title: &str,
        body: &str,
        background_color: Option<&str>,
    ) {
        let elements = vec![
            SlideElement {
                element_type: SlideElementType::Text {
                    content: title.to_string(),
                    font_size: Some(44),
                    bold: true,
                    italic: false,
                    underline: false,
                    color: Some("#1F3864".to_string()),
                    font_family: Some("Calibri Light".to_string()),
                    alignment: Some("center".to_string()),
                },
                x: 457200,
                y: 685800,
                width: 11277600,
                height: 914400,
                rotation: None,
            },
            SlideElement {
                element_type: SlideElementType::Text {
                    content: body.to_string(),
                    font_size: Some(24),
                    bold: false,
                    italic: false,
                    underline: false,
                    color: Some("#333333".to_string()),
                    font_family: Some("Calibri".to_string()),
                    alignment: Some("left".to_string()),
                },
                    x: 914400,
                y: 2286000,
                width: 10363200,
                height: 3657600,
                rotation: None,
            },
        ];
        self.slides.push(Slide {
            title: Some(title.to_string()),
            body: Some(body.to_string()),
            elements,
            background_color: background_color.map(|c| c.to_string()),
            background_image: None,
            notes: None,
            transition: TransitionEffect::Fade,
            transition_duration: 400,
            layout: SlideLayout::TitleAndContent,
        });
    }
}
