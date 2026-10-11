//! JSON → PPTX parsing.
//! Split from pptx.rs for file-size limits.

use super::pptx::{Slide, SlideElement, SlideElementType, SlideLayout, PptxDocument, TransitionEffect};


pub fn from_json(json: &str) -> Result<PptxDocument, String> {
    #[derive(serde::Deserialize)]
    struct JsonSlide {
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        body: Option<String>,
        #[serde(default)]
        background_color: Option<String>,
        #[serde(default)]
        notes: Option<String>,
        #[serde(default)]
        transition: Option<String>,
        #[serde(default)]
        transition_duration: Option<u32>,
        #[serde(default)]
        layout: Option<String>,
        #[serde(default)]
        elements: Option<Vec<JsonElement>>,
    }

    #[derive(serde::Deserialize)]
    #[serde(tag = "type")]
    enum JsonElement {
        #[serde(rename = "text")]
        Text {
            content: String,
            #[serde(default)]
            x: Option<u32>,
            #[serde(default)]
            y: Option<u32>,
            #[serde(default)]
            width: Option<u32>,
            #[serde(default)]
            height: Option<u32>,
            #[serde(default)]
            font_size: Option<u32>,
            #[serde(default)]
            bold: Option<bool>,
            #[serde(default)]
            italic: Option<bool>,
            #[serde(default)]
            underline: Option<bool>,
            #[serde(default)]
            color: Option<String>,
            #[serde(default)]
            font_family: Option<String>,
            #[serde(default)]
            alignment: Option<String>,
        },
        #[serde(rename = "image")]
        Image {
            data_base64: String,
            #[serde(default)]
            mime_type: Option<String>,
            #[serde(default)]
            x: Option<u32>,
            #[serde(default)]
            y: Option<u32>,
            #[serde(default)]
            width: Option<u32>,
            #[serde(default)]
            height: Option<u32>,
            #[serde(default)]
            alt_text: Option<String>,
        },
        #[serde(rename = "shape")]
        Shape {
            shape_type: String,
            #[serde(default)]
            x: Option<u32>,
            #[serde(default)]
            y: Option<u32>,
            #[serde(default)]
            width: Option<u32>,
            #[serde(default)]
            height: Option<u32>,
            #[serde(default)]
            fill_color: Option<String>,
            #[serde(default)]
            line_color: Option<String>,
            #[serde(default)]
            text: Option<String>,
        },
        #[serde(rename = "table")]
        Table {
            headers: Vec<String>,
            rows: Vec<Vec<String>>,
            #[serde(default)]
            x: Option<u32>,
            #[serde(default)]
            y: Option<u32>,
            #[serde(default)]
            width: Option<u32>,
            #[serde(default)]
            height: Option<u32>,
        },
    }

    #[derive(serde::Deserialize)]
    struct JsonRoot {
        #[serde(default)]
        slides: Vec<JsonSlide>,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        subject: Option<String>,
        #[serde(default)]
        author: Option<String>,
        #[serde(default)]
        slide_width: Option<u32>,
        #[serde(default)]
        slide_height: Option<u32>,
        #[serde(default)]
        default_font: Option<String>,
        #[serde(default)]
        heading_font: Option<String>,
    }

    let root: JsonRoot = serde_json::from_str(json).map_err(|e| e.to_string())?;

    let mut doc = PptxDocument::new();
    doc.title = root.title;
    doc.subject = root.subject;
    doc.author = root.author;

    if let Some(w) = root.slide_width {
        doc.styles.slide_width = w;
    }
    if let Some(h) = root.slide_height {
        doc.styles.slide_height = h;
    }
    if let Some(f) = root.default_font {
        doc.styles.default_font = f;
    }
    if let Some(f) = root.heading_font {
        doc.styles.heading_font = f;
    }

    for js in root.slides {
        let transition = js
            .transition
            .as_deref()
            .map(TransitionEffect::from_ooxml_name)
            .unwrap_or(TransitionEffect::Fade);
        let layout = match js.layout.as_deref() {
            Some("title") => SlideLayout::Title,
            Some("title_and_content" | "titleAndContent") => SlideLayout::TitleAndContent,
            Some("two_content" | "twoContent") => SlideLayout::TwoContent,
            Some("section_header" | "sectionHeader") => SlideLayout::SectionHeader,
            Some("comparison") => SlideLayout::Comparison,
            Some("content_with_caption" | "contentWithCaption") => SlideLayout::ContentWithCaption,
            _ => SlideLayout::Blank,
        };

        let mut elements: Vec<SlideElement> = Vec::new();

        if let Some(ref json_elements) = js.elements {
            for je in json_elements {
                match je {
                    JsonElement::Text {
                        content,
                        x,
                        y,
                        width,
                        height,
                        font_size,
                        bold,
                        italic,
                        underline,
                        color,
                        font_family,
                        alignment,
                    } => {
                        elements.push(SlideElement {
                            element_type: SlideElementType::Text {
                                content: content.clone(),
                                font_size: *font_size,
                                bold: bold.unwrap_or(false),
                                italic: italic.unwrap_or(false),
                                underline: underline.unwrap_or(false),
                                color: color.clone(),
                                font_family: font_family.clone(),
                                alignment: alignment.clone(),
                            },
                            x: x.unwrap_or(457200),
                            y: y.unwrap_or(685800),
                            width: width.unwrap_or(11277600),
                            height: height.unwrap_or(914400),
                            rotation: None,
                        });
                    }
                    JsonElement::Image {
                        data_base64,
                        mime_type,
                        x,
                        y,
                        width,
                        height,
                        alt_text,
                    } => {
                        let data =
                            base64::Engine::decode(
                                &base64::engine::general_purpose::STANDARD,
                                data_base64,
                            )
                            .map_err(|e| format!("Base64 decode error: {}", e))?;
                        let mime = mime_type
                            .clone()
                            .unwrap_or_else(|| "image/png".to_string());
                        elements.push(SlideElement {
                            element_type: SlideElementType::Image {
                                data,
                                mime_type: mime,
                                alt_text: alt_text.clone(),
                            },
                            x: x.unwrap_or(914400),
                            y: y.unwrap_or(914400),
                            width: width.unwrap_or(3657600),
                            height: height.unwrap_or(2743200),
                            rotation: None,
                        });
                    }
                    JsonElement::Shape {
                        shape_type,
                        x,
                        y,
                        width,
                        height,
                        fill_color,
                        line_color,
                        text,
                    } => {
                        elements.push(SlideElement {
                            element_type: SlideElementType::Shape {
                                shape_type: shape_type.clone(),
                                fill_color: fill_color.clone(),
                                line_color: line_color.clone(),
                                text: text.clone(),
                            },
                            x: x.unwrap_or(914400),
                            y: y.unwrap_or(914400),
                            width: width.unwrap_or(1828800),
                            height: height.unwrap_or(1828800),
                            rotation: None,
                        });
                    }
                    JsonElement::Table {
                        headers,
                        rows,
                        x,
                        y,
                        width,
                        height,
                    } => {
                        elements.push(SlideElement {
                            element_type: SlideElementType::Table {
                                headers: headers.clone(),
                                rows: rows.clone(),
                            },
                            x: x.unwrap_or(914400),
                            y: y.unwrap_or(914400),
                            width: width.unwrap_or(8229600),
                            height: height.unwrap_or(3657600),
                            rotation: None,
                        });
                    }
                }
            }
        }

        doc.slides.push(Slide {
            title: js.title,
            body: js.body,
            elements,
            background_color: js.background_color,
            background_image: None,
            notes: js.notes,
            transition,
            transition_duration: js.transition_duration.unwrap_or(400),
            layout,
        });
    }

    Ok(doc)
}
