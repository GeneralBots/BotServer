//! PPTX packaging entry point: assembles every OOXML part into a .pptx archive.
//! Split from pptx.rs for file-size limits.

use super::pptx::PptxDocument;
use zip::ZipWriter;

impl PptxDocument {
    /// Serializes the presentation to the bytes of a `.pptx` (OOXML) package.
    pub fn export(&self) -> Result<Vec<u8>, String> {
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(&mut buf);

        self.write_package_parts(&mut zip)?;
        self.write_slide_parts(&mut zip)?;
        self.write_doc_props(&mut zip)?;

        zip.finish().map_err(|e| e.to_string())?;
        Ok(buf.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::super::pptx::{PptxDocument, SlideElement, SlideElementType};
    use std::io::Cursor;

    /// Every part declared in `[Content_Types].xml` must be present in the
    /// archive, otherwise PowerPoint rejects the file. The 2026-10 split of
    /// `pptx.rs` silently dropped them (theme, master, layout, rels, props).
    #[test]
    fn export_contains_every_declared_part() {
        let mut doc = PptxDocument::new();
        doc.title = Some("Split check".to_string());
        doc.add_text_slide("Title", "Body", None);
        doc.slides[0].elements.push(SlideElement {
            element_type: SlideElementType::Text {
                content: "element".to_string(),
                font_size: Some(18),
                bold: false,
                italic: false,
                underline: false,
                color: None,
                font_family: None,
                alignment: None,
            },
            x: 0,
            y: 0,
            width: 100,
            height: 100,
            rotation: None,
        });

        let bytes = doc.export().expect("export pptx");
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("valid zip");
        let mut names: Vec<String> = (0..archive.len())
            .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
            .collect();
        names.sort();

        let required = [
            "[Content_Types].xml",
            "_rels/.rels",
            "docProps/app.xml",
            "docProps/core.xml",
            "ppt/_rels/presentation.xml.rels",
            "ppt/presentation.xml",
            "ppt/presProps.xml",
            "ppt/slideLayouts/slideLayout1.xml",
            "ppt/slideMasters/slideMaster1.xml",
            "ppt/slides/slide1.xml",
            "ppt/theme/theme1.xml",
            "ppt/viewProps.xml",
        ];
        for part in required {
            assert!(names.iter().any(|n| n == part), "missing part {part}: {names:?}");
        }

        let mut content_types = String::new();
        std::io::Read::read_to_string(
            &mut archive.by_name("[Content_Types].xml").expect("content types"),
            &mut content_types,
        )
        .expect("read content types");
        assert!(content_types.contains("/ppt/theme/theme1.xml"));

        let mut slide = String::new();
        std::io::Read::read_to_string(
            &mut archive.by_name("ppt/slides/slide1.xml").expect("slide 1"),
            &mut slide,
        )
        .expect("read slide 1");
        assert!(slide.contains("Title"), "slide xml: {slide}");
        assert!(slide.contains("element"), "slide xml: {slide}");
    }
}
