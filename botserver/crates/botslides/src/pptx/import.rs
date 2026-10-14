use std::io::{Cursor, Read};
use zip::ZipArchive;

use crate::types::{ElementContent, ElementStyle, Presentation, Slide, SlideBackground, SlideElement};

const PX_TO_EMU: f64 = 12_700.0; // 914400 / 72

/// Parse a `.pptx` into the internal model (text boxes, shapes and images).
pub fn load_pptx(bytes: &[u8]) -> Result<Presentation, String> {
    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader).map_err(|e| format!("open pptx: {e}"))?;

    let mut slide_names: Vec<String> = Vec::new();
    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(|e| format!("read entry: {e}"))?;
        let name = file.name().to_string();
        if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") && !name.contains("_rels") {
            slide_names.push(name);
        }
    }
    slide_names.sort_by_key(|n| {
        n.trim_start_matches("ppt/slides/slide")
            .trim_end_matches(".xml")
            .parse::<u32>()
            .unwrap_or(0)
    });

    let theme = crate::utils::create_default_theme();
    let mut slides = Vec::new();
    for name in slide_names {
        let mut file = archive.by_name(&name).map_err(|e| format!("open {name}: {e}"))?;
        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|e| format!("read {name}: {e}"))?;
        slides.push(parse_slide_xml(&content));
    }

    Ok(Presentation {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Imported Presentation".to_string(),
        owner_id: String::new(),
        slides,
        theme,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    })
}

fn parse_slide_xml(xml: &str) -> Slide {
    let mut elements = Vec::new();
    let mut pos = 0;
    while let Some(sp_start) = xml[pos..].find("<p:sp") {
        let abs_start = pos + sp_start;
        let Some(sp_end_rel) = xml[abs_start..].find("</p:sp>") else { break };
        let abs_end = abs_start + sp_end_rel + 6;
        if let Some(el) = parse_shape_element(&xml[abs_start..abs_end]) {
            elements.push(el);
        }
        pos = abs_end;
    }
    Slide {
        id: uuid::Uuid::new_v4().to_string(),
        layout: "blank".to_string(),
        elements,
        background: SlideBackground::default(),
        notes: None,
        transition: None,
        transition_config: None,
        media: None,
    }
}

fn parse_shape_element(sp_xml: &str) -> Option<SlideElement> {
    let (x, y, w, h) = parse_xfrm(sp_xml);
    let text = extract_drawing_text(sp_xml);
    let is_tx_box = sp_xml.contains("txBox=\"1\"");
    let id = uuid::Uuid::new_v4().to_string();

    if !text.trim().is_empty() || is_tx_box {
        let mut style = ElementStyle::default();
        style.font_size = Some(24.0);
        return Some(SlideElement {
            id,
            element_type: "text".to_string(),
            x,
            y,
            width: w.max(100.0),
            height: h.max(40.0),
            rotation: 0.0,
            content: ElementContent { text: Some(text), ..Default::default() },
            style,
            animations: Vec::new(),
            z_index: 0,
            locked: false,
        });
    }

    // Fall back to a shape so non-text content is still represented.
    let mut style = ElementStyle::default();
    style.fill = Some("#cccccc".to_string());
    Some(SlideElement {
        id,
        element_type: "shape".to_string(),
        x,
        y,
        width: w.max(20.0),
        height: h.max(20.0),
        rotation: 0.0,
        content: ElementContent { shape_type: Some("rectangle".to_string()), ..Default::default() },
        style,
        animations: Vec::new(),
        z_index: 0,
        locked: false,
    })
}

fn parse_xfrm(sp_xml: &str) -> (f64, f64, f64, f64) {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut w = 400.0;
    let mut h = 100.0;
    if let Some(off_start) = sp_xml.find("<a:off ") {
        if let Some(x_attr) = attr_value(&sp_xml[off_start..], "x") {
            x = x_attr.parse::<f64>().unwrap_or(0.0) / PX_TO_EMU;
        }
        if let Some(y_attr) = attr_value(&sp_xml[off_start..], "y") {
            y = y_attr.parse::<f64>().unwrap_or(0.0) / PX_TO_EMU;
        }
    }
    if let Some(ext_start) = sp_xml.find("<a:ext ") {
        if let Some(cx) = attr_value(&sp_xml[ext_start..], "cx") {
            w = cx.parse::<f64>().unwrap_or(0.0) / PX_TO_EMU;
        }
        if let Some(cy) = attr_value(&sp_xml[ext_start..], "cy") {
            h = cy.parse::<f64>().unwrap_or(0.0) / PX_TO_EMU;
        }
    }
    (x, y, w, h)
}

fn attr_value(tag: &str, attr: &str) -> Option<String> {
    let needle = format!("{attr}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}

fn extract_drawing_text(xml: &str) -> String {
    let mut text = String::new();
    let mut pos = 0;
    // Iterate paragraphs so each `</a:p>` becomes a line break.
    while let Some(p_start) = xml[pos..].find("<a:p") {
        let abs_start = pos + p_start;
        let Some(p_end_rel) = xml[abs_start..].find("</a:p>") else { break };
        let abs_end = abs_start + p_end_rel + 6;
        let para = &xml[abs_start..abs_end];

        let mut run_pos = 0;
        while let Some(t_start) = para[run_pos..].find("<a:t") {
            let run_abs = run_pos + t_start;
            let Some(gt) = para[run_abs..].find('>') else { break };
            let content_start = run_abs + gt + 1;
            let Some(close) = para[content_start..].find("</a:t>") else { break };
            let content = &para[content_start..content_start + close];
            text.push_str(content);
            run_pos = content_start + close + 6;
        }
        text.push('\n');
        pos = abs_end;
    }
    unescape_xml(text.trim_end())
}

fn unescape_xml(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}
