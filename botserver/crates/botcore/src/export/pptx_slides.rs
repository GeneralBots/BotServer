//! PPTX slide parts (text, images, tables, notes).
//! Split from pptx.rs for file-size limits; the OOXML bodies are verbatim.

use super::pptx::{PptxDocument, SlideElementType, TransitionEffect};
use std::io::Write;
use zip::ZipWriter;

impl PptxDocument {

    /// Writes one ppt/slides/slideN.xml part per slide, plus its image and notes parts.
    pub(crate) fn write_slide_parts(
        &self,
        zip: &mut ZipWriter<&mut std::io::Cursor<Vec<u8>>>,
    ) -> Result<(), String> {

        // Individual slides
        let mut image_counter = 0u32;
        for (i, slide) in self.slides.iter().enumerate() {
            let slide_num = i + 1;
            let mut rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdLay" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
"#.to_string();

            let mut sp_tree = String::from(r#"<p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>"#);

            for (ei, element) in slide.elements.iter().enumerate() {
                let el_id = ei + 2;
                match &element.element_type {
                    SlideElementType::Text {
                        content,
                        font_size,
                        bold,
                        italic,
                        underline,
                        color,
                        font_family,
                        alignment,
                    } => {
                        let sz = font_size.unwrap_or(18) * 100;
                        let bold_xml = if *bold { r#"<a:b/>"# } else { "" };
                        let italic_xml = if *italic { r#"<a:i/>"# } else { "" };
                        let underline_xml = if *underline { r#"<a:u/>"# } else { "" };
                        let color_val = color
                            .as_deref()
                            .unwrap_or(&self.styles.body_color)
                            .trim_start_matches('#');
                        let font = font_family
                            .as_deref()
                            .unwrap_or(&self.styles.default_font);
                        let align_val = match alignment.as_deref() {
                            Some("center") => "ctr",
                            Some("right") => "r",
                            _ => "l",
                        };
                        let escaped = content
                            .replace("&", "&amp;")
                            .replace("<", "&lt;")
                            .replace(">", "&gt;")
                            .replace("\"", "&quot;")
                            .replace("'", "&apos;");
                        let cx = element.width;
                        let cy = element.height;
                        let off_x = element.x;
                        let off_y = element.y;
                        let font_decor = format!("{bold_xml}{italic_xml}{underline_xml}");

                        sp_tree.push_str(&format!(
                            r#"<p:sp>
        <p:nvSpPr>
          <p:cNvPr id="{el_id}" name="TextBox{el_id}"/>
          <p:cNvSpPr txBox="1"/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm>
            <a:off x="{off_x}" y="{off_y}"/>
            <a:ext cx="{cx}" cy="{cy}"/>
          </a:xfrm>
          <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
          <a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill>
        </p:spPr>
        <p:txBody>
          <a:bodyPr rtlCol="0" anchor="t"/>
          <a:lstStyle/>
          <a:p>
            <a:pPr algn="{align_val}"/>
            <a:r>
              <a:rPr lang="en-US" sz="{sz}" dirty="0" smtClean="0"{font_decor}>
                <a:solidFill><a:srgbClr val="{color_val}"/></a:solidFill>
                <a:latin typeface="{font}"/>
              </a:rPr>
              <a:t xml:space="preserve">{escaped}</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>"#,
                        ));
                    }
                    SlideElementType::Image {
                        data,
                        mime_type,
                        alt_text,
                    } => {
                        image_counter += 1;
                        let img_id = image_counter;
                        let ext = if mime_type.contains("jpeg") || mime_type.contains("jpg") {
                            "jpeg"
                        } else {
                            "png"
                        };
                        let img_path = format!("ppt/media/image{}.{}", img_id, ext);
                        rels.push_str(&format!(
                            r#"  <Relationship Id="rImg{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image{}.{}"/>"#,
                            img_id, img_id, ext
                        ));

                        zip.start_file(&img_path, <zip::write::FileOptions<()>>::default())
                            .map_err(|e| e.to_string())?;
                            std::io::copy(
                            &mut std::io::Cursor::new(data),
                            &mut *zip,
                        )
                        .map_err(|e| e.to_string())?;

                        let cx = element.width;
                        let cy = element.height;
                        let off_x = element.x;
                        let off_y = element.y;
                        let alt = alt_text
                            .as_deref()
                            .unwrap_or("Image")
                            .replace("&", "&amp;")
                            .replace("<", "&lt;")
                            .replace(">", "&gt;");

                        sp_tree.push_str(&format!(
                            r#"<p:pic>
        <p:nvPicPr>
          <p:cNvPr id="{}" name="Image{}" descr="{}"/>
          <p:cNvPicPr/>
          <p:nvPr/>
        </p:nvPicPr>
        <p:blipFill>
          <a:blip r:embed="rImg{}"/>
          <a:stretch><a:fillRect/></a:stretch>
        </p:blipFill>
        <p:spPr>
          <a:xfrm>
            <a:off x="{}" y="{}"/>
            <a:ext cx="{}" cy="{}"/>
          </a:xfrm>
          <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
        </p:spPr>
      </p:pic>"#,
                            el_id, img_id, alt, img_id,
                            off_x, off_y, cx, cy
                        ));
                    }
                    SlideElementType::Shape {
                        shape_type,
                        fill_color,
                        line_color: _line_color,
                        text,
                    } => {
                        let fill = fill_color
                            .as_deref()
                            .unwrap_or("2E75B6")
                            .trim_start_matches('#');
                        let cx = element.width;
                        let cy = element.height;
                        let off_x = element.x;
                        let off_y = element.y;

                        let shape_prst = match shape_type.to_lowercase().as_str() {
                            "circle" | "ellipse" | "oval" => "ellipse",
                            "triangle" => "triangle",
                            "diamond" => "diamond",
                            "arrow" | "right-arrow" => "rightArrow",
                            "line" => "line",
                            _ => "rect",
                        };

                        if let Some(txt) = text {
                            let escaped_txt = txt
                                .replace("&", "&amp;")
                                .replace("<", "&lt;")
                                .replace(">", "&gt;");
                            sp_tree.push_str(&format!(
                                r#"<p:sp>
        <p:nvSpPr>
          <p:cNvPr id="{}" name="Shape{}"/>
          <p:cNvSpPr/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm>
            <a:off x="{}" y="{}"/>
            <a:ext cx="{}" cy="{}"/>
          </a:xfrm>
          <a:prstGeom prst="{}"><a:avLst/></a:prstGeom>
          <a:solidFill><a:srgbClr val="{}"/></a:solidFill>
        </p:spPr>
        <p:txBody>
          <a:bodyPr rtlCol="0" anchor="ctr"/>
          <a:lstStyle/>
          <a:p>
            <a:pPr algn="ctr"/>
            <a:r>
              <a:rPr lang="en-US" sz="1800">
                <a:solidFill><a:schemeClr val="lt1"/></a:solidFill>
              </a:rPr>
              <a:t>{}</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>"#,
                                el_id, el_id,
                                off_x, off_y, cx, cy,
                                shape_prst, fill, escaped_txt
                            ));
                        } else {
                            sp_tree.push_str(&format!(
                                r#"<p:sp>
        <p:nvSpPr>
          <p:cNvPr id="{}" name="Shape{}"/>
          <p:cNvSpPr/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm>
            <a:off x="{}" y="{}"/>
            <a:ext cx="{}" cy="{}"/>
          </a:xfrm>
          <a:prstGeom prst="{}"><a:avLst/></a:prstGeom>
          <a:solidFill><a:srgbClr val="{}"/></a:solidFill>
        </p:spPr>
      </p:sp>"#,
                                el_id, el_id,
                                off_x, off_y, cx, cy,
                                shape_prst, fill
                            ));
                        }
                    }
                    SlideElementType::Table { headers, rows } => {
                        let cx = element.width;
                        let cy = element.height;
                        let off_x = element.x;
                        let off_y = element.y;
                        let col_count = headers.len().max(1);
                        let col_width = cx / col_count as u32;

                        let mut tbl_xml = format!(
                            r#"<p:graphicFrame>
        <p:nvGraphicFramePr>
          <p:cNvPr id="{}" name="Table{}"/>
          <p:cNvGraphicFramePr/>
          <p:nvPr/>
        </p:nvGraphicFramePr>
        <p:xfrm>
          <a:off x="{}" y="{}"/>
          <a:ext cx="{}" cy="{}"/>
        </p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            <a:tbl>
              <a:tblPr>
                <a:tableStyleId>{{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}}</a:tableStyleId>
              </a:tblPr>
              <a:tblGrid>"#,
                            el_id, el_id,
                            off_x, off_y, cx, cy
                        );
                        for _ in 0..col_count {
                            tbl_xml.push_str(&format!(
                                r#"<a:gridCol w="{}"/>"#,
                                col_width
                            ));
                        }
                        tbl_xml.push_str(r#"</a:tblGrid>"#);

                        // Header row
                        tbl_xml.push_str(r#"<a:tr h="370840"><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:rPr sz="1400" b="1"><a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill></a:rPr><a:t>"#);
                        for (hi, h) in headers.iter().enumerate() {
                            let escaped = h
                                .replace("&", "&amp;")
                                .replace("<", "&lt;")
                                .replace(">", "&gt;");
                            if hi > 0 {
                                tbl_xml.push_str(r#"</a:t></a:r></a:p></a:txBody></a:tc><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:rPr sz="1400" b="1"><a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill></a:rPr><a:t>"#);
                            }
                            tbl_xml.push_str(&escaped);
                        }
                        tbl_xml.push_str(r#"</a:t></a:r></a:p></a:txBody></a:tc></a:tr>"#);

                        // Data rows
                        for row in rows {
                            tbl_xml.push_str(r#"<a:tr h="370840">"#);
                            for (ci, cell) in row.iter().enumerate() {
                                let escaped = cell
                                    .replace("&", "&amp;")
                                    .replace("<", "&lt;")
                                    .replace(">", "&gt;");
                                tbl_xml.push_str(&format!(
                                    r#"<a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:rPr sz="1200"><a:solidFill><a:srgbClr val="333333"/></a:solidFill></a:rPr><a:t>{esc}</a:t></a:r></a:p></a:txBody></a:tc>"#,
                                    esc = escaped
                                ));
                                if ci == 0 && row.len() < headers.len() {
                                    let extra = headers.len() - row.len();
                                    for _ in 0..extra {
                                        tbl_xml.push_str(r#"<a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:rPr sz="1200"/><a:t/></a:r></a:p></a:txBody></a:tc>"#);
                                    }
                                }
                            }
                            tbl_xml.push_str(r#"</a:tr>"#);
                        }
                        tbl_xml.push_str(
                            r#"</a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#,
                        );
                        sp_tree.push_str(&tbl_xml);
                    }
                }
            }

            sp_tree.push_str("</p:spTree>");

            // Slide XML
            let bg_color = slide
                .background_color
                .as_deref()
                .unwrap_or(&self.styles.background_color)
                .trim_start_matches('#');

            let transition_xml = match slide.transition {
                TransitionEffect::None => String::new(),
                _ => {
                    let dur = slide.transition_duration * 1000;
                    let trans_type = slide.transition.as_ooxml();
                    format!(
                        r#"  <p:transition spd="med" advTm="{}">
    <p:{}/>
  </p:transition>"#,
                        dur, trans_type
                    )
                }
            };

            let slide_xml = format!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld name="Slide{}">
    <p:bg>
      <p:bgPr>
        <a:solidFill>
          <a:srgbClr val="{}"/>
        </a:solidFill>
      </p:bgPr>
    </p:bg>
    {}
  </p:cSld>
  {}
</p:sld>"#,
                slide_num, bg_color, sp_tree, transition_xml
            );

            let slide_path = format!("ppt/slides/slide{}.xml", slide_num);
            zip.start_file(&slide_path, <zip::write::FileOptions<()>>::default())
                .map_err(|e| e.to_string())?;
            write!(zip, "{}", slide_xml).map_err(|e| e.to_string())?;

            // Slide rels
            let rels_path = format!("ppt/slides/_rels/slide{}.xml.rels", slide_num);
            rels.push_str("</Relationships>");
            zip.start_file(&rels_path, <zip::write::FileOptions<()>>::default())
                .map_err(|e| e.to_string())?;
            write!(zip, "{}", rels).map_err(|e| e.to_string())?;

            // Notes slide
            if let Some(ref notes) = slide.notes {
                let escaped_notes = notes
                    .replace("&", "&amp;")
                    .replace("<", "&lt;")
                    .replace(">", "&gt;");
                let notes_path = format!("ppt/notesSlides/notesSlide{}.xml", slide_num);
                let notes_xml = format!(
                    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:notes xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
         xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
         xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld name="NotesSlide{}">
    <p:spTree>
      <p:nvGrpSpPr>
        <p:cNvPr id="1" name=""/>
        <p:cNvGrpSpPr/>
        <p:nvPr/>
      </p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Notes Placeholder"/>
          <p:cNvSpPr txBox="1"/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm>
            <a:off x="457200" y="457200"/>
            <a:ext cx="8229600" cy="5943600"/>
          </a:xfrm>
        </p:spPr>
        <p:txBody>
          <a:bodyPr rtlCol="0" anchor="t"/>
          <a:lstStyle/>
          <a:p>
            <a:r>
              <a:rPr lang="en-US" sz="1800">
                <a:solidFill><a:srgbClr val="333333"/></a:solidFill>
              </a:rPr>
              <a:t xml:space="preserve">{}</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:notes>"#,
                    slide_num, escaped_notes
                );
                zip.start_file(&notes_path, <zip::write::FileOptions<()>>::default())
                    .map_err(|e| e.to_string())?;
                write!(zip, "{}", notes_xml).map_err(|e| e.to_string())?;
            }
        }


        Ok(())

    }
}