//! HTML → PPTX parsing.
//! Split from pptx.rs for file-size limits.

use super::pptx::{Slide, SlideElement, SlideElementType, SlideLayout, PptxDocument, TransitionEffect};


pub fn from_html(html: &str) -> Result<PptxDocument, String> {
    let mut doc = PptxDocument::new();
    let _current_slide: Option<Slide> = None;

    let re_h1 = crate::re_cache::cached_re!(r"<h1[^>]*>(.*?)</h1>")
        .ok_or_else(|| "invalid h1 regex".to_string())?;
    let re_h2 = crate::re_cache::cached_re!(r"<h2[^>]*>(.*?)</h2>")
        .ok_or_else(|| "invalid h2 regex".to_string())?;
    let re_p = crate::re_cache::cached_re!(r"<p[^>]*>(.*?)</p>")
        .ok_or_else(|| "invalid paragraph regex".to_string())?;
    let re_hr = crate::re_cache::cached_re!(r"<hr[^>]*/?>")
        .ok_or_else(|| "invalid horizontal rule regex".to_string())?;
    let re_img = crate::re_cache::cached_re!(r#"<img[^>]*src=["']([^"']+)["'][^>]*>"#)
        .ok_or_else(|| "invalid image regex".to_string())?;
    let re_tag = crate::re_cache::cached_re!(r"<[^>]+>")
        .ok_or_else(|| "invalid tag regex".to_string())?;
    let re_br = crate::re_cache::cached_re!(r"<br\s*/?>")
        .ok_or_else(|| "invalid line break regex".to_string())?;

    let mut parts: Vec<&str> = re_hr.split(html).collect();
    if parts.is_empty() {
        parts.push(html);
    }

    for part in parts {
        let mut elements: Vec<SlideElement> = Vec::new();
        let mut body_lines: Vec<String> = Vec::new();
        let mut title_text: Option<String> = None;
        let mut y_offset: u32 = 685800;

        if let Some(cap) = re_h1.captures(part) {
            let text = re_tag
                .replace_all(&cap[1], "")
                .to_string()
                .trim()
                .to_string();
            title_text = Some(text.clone());
            let text2 = text.clone();
            elements.push(SlideElement {
                element_type: SlideElementType::Text {
                    content: text2,
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
            });
            y_offset = 2286000;
        } else if let Some(cap) = re_h2.captures(part) {
            let text = re_tag
                .replace_all(&cap[1], "")
                .to_string()
                .trim()
                .to_string();
            elements.push(SlideElement {
                element_type: SlideElementType::Text {
                    content: text.clone(),
                    font_size: Some(36),
                    bold: true,
                    italic: false,
                    underline: false,
                    color: Some("2E75B6".to_string()),
                    font_family: Some("Calibri Light".to_string()),
                    alignment: Some("left".to_string()),
                },
                x: 914400,
                y: y_offset,
                width: 10363200,
                height: 685800,
                rotation: None,
            });
            y_offset += 914400;
        }

        for cap in re_img.captures_iter(part) {
            let src = cap[1].to_string();
            body_lines.push(format!("[Image: {}]", src));
        }

        for cap in re_p.captures_iter(part) {
            let raw = &cap[1];
            let with_breaks = re_br.replace_all(raw, "\n");
            let text = re_tag
                .replace_all(&with_breaks, "")
                .to_string()
                .trim()
                .to_string();
            if !text.is_empty() {
                body_lines.push(text);
            }
        }

        if !body_lines.is_empty() {
            let body_text = body_lines.join("\n\n");
            elements.push(SlideElement {
                element_type: SlideElementType::Text {
                    content: body_text.clone(),
                    font_size: Some(24),
                    bold: false,
                    italic: false,
                    underline: false,
                    color: Some("333333".to_string()),
                    font_family: Some("Calibri".to_string()),
                    alignment: Some("left".to_string()),
                },
                x: 914400,
                y: y_offset,
                width: 10363200,
                height: 3657600,
                rotation: None,
            });
        }

        doc.slides.push(Slide {
            title: title_text,
            body: if body_lines.is_empty() {
                None
            } else {
                Some(body_lines.join("\n\n"))
            },
            elements,
            background_color: None,
            background_image: None,
            notes: None,
            transition: TransitionEffect::Fade,
            transition_duration: 400,
            layout: SlideLayout::TitleAndContent,
        });
    }

    Ok(doc)
}
