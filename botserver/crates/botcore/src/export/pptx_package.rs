//! PPTX package skeleton (content types, presentation, master, layout, theme).
//! Split from pptx.rs for file-size limits; the OOXML bodies are verbatim.

use super::pptx::PptxDocument;
use std::io::Write;
use zip::ZipWriter;

impl PptxDocument {

    /// Writes every OOXML part that is independent of the individual slides.
    pub(crate) fn write_package_parts(&self, zip: &mut ZipWriter<&mut std::io::Cursor<Vec<u8>>>) -> Result<(), String> {

        // [Content_Types].xml
        zip.start_file("[Content_Types].xml", <zip::write::FileOptions<()>>::default())
            .map_err(|e| e.to_string())?;
        write!(
            zip,
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="png" ContentType="image/png"/>
  <Default Extension="jpeg" ContentType="image/jpeg"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
  <Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
  <Override PartName="/ppt/presProps.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presProps+xml"/>
  <Override PartName="/ppt/viewProps.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.viewProps+xml"/>
  <Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
  <Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>
"#,
        )
        .map_err(|e| e.to_string())?;

        for (i, _slide) in self.slides.iter().enumerate() {
            write!(
                zip,
                r#"  <Override PartName="/ppt/slides/slide{}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#,
                i + 1
            )
            .map_err(|e| e.to_string())?;
        }

        let has_notes = self.slides.iter().any(|s| s.notes.is_some());
        if has_notes {
            for (i, slide) in self.slides.iter().enumerate() {
                if slide.notes.is_some() {
                    write!(
                        zip,
                        r#"  <Override PartName="/ppt/notesSlides/notesSlide{}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml"/>"#,
                        i + 1
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
        }

        writeln!(zip, "</Types>").map_err(|e| e.to_string())?;

        // _rels/.rels
        zip.start_file("_rels/.rels", <zip::write::FileOptions<()>>::default())
            .map_err(|e| e.to_string())?;
        write!(
            zip,
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/>
</Relationships>"#,
        )
        .map_err(|e| e.to_string())?;

        // ppt/presentation.xml
        zip.start_file("ppt/presentation.xml", <zip::write::FileOptions<()>>::default())
            .map_err(|e| e.to_string())?;

        let slide_id_list: String = self
            .slides
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let r_id = i + 256;
                let notes_rel = if s.notes.is_some() {
                    format!(r#"<p:notesIdLst><p:notesId id="{}" r:id="notes{}"/></p:notesIdLst>"#, i + 1, i + 1)
                } else {
                    String::new()
                };
                format!(
                    r#"    <p:sldId id="{}" r:id="rId{}"/>{}
"#,
                    i + 256,
                    r_id,
                    notes_rel
                )
            })
            .collect();

        write!(
            zip,
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst>
    <p:sldMasterId id="2147483648" r:id="rIdSlM"/>
  </p:sldMasterIdLst>
  <p:sldIdLst>
{sld_ids}  </p:sldIdLst>
  <p:sldSz cx="{slide_width}" cy="{slide_height}"/>
  <p:notesSz cx="9144000" cy="6858000"/>
  <p:defaultTextStyle>
    <a:defPPr>
      <a:defRPr sz="1800" kern="1200">
        <a:solidFill><a:schemeClr val="tx1"/></a:solidFill>
        <a:latin typeface="+mn-lt"/>
      </a:defRPr>
    </a:defPPr>
  </p:defaultTextStyle>
</p:presentation>"#,
            sld_ids = slide_id_list,
            slide_width = self.styles.slide_width,
            slide_height = self.styles.slide_height,
        )
        .map_err(|e| e.to_string())?;

        // ppt/_rels/presentation.xml.rels
        zip.start_file("ppt/_rels/presentation.xml.rels", <zip::write::FileOptions<()>>::default())
            .map_err(|e| e.to_string())?;
        let mut pres_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdSlM" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rIdThm" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
  <Relationship Id="rIdPrPr" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/presProps" Target="presProps.xml"/>
  <Relationship Id="rIdVwPr" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/viewProps" Target="viewProps.xml"/>
"#.to_string();
        for (i, _slide) in self.slides.iter().enumerate() {
            let r_id = i + 256;
            pres_rels.push_str(&format!(
                r#"  <Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{}.xml"/>"#,
                r_id,
                i + 1
            ));
        }
        for (i, slide) in self.slides.iter().enumerate() {
            if slide.notes.is_some() {
                pres_rels.push_str(&format!(
                    r#"  <Relationship Id="notes{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide" Target="notesSlides/notesSlide{}.xml"/>"#,
                    i + 1,
                    i + 1
                ));
            }
        }
        pres_rels.push_str("</Relationships>");
        write!(zip, "{}", pres_rels).map_err(|e| e.to_string())?;

        // ppt/slideMasters/slideMaster1.xml
        zip.start_file(
            "ppt/slideMasters/slideMaster1.xml", <zip::write::FileOptions<()>>::default())
        .map_err(|e| e.to_string())?;
        write!(
            zip,
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:solidFill>
          <a:srgbClr val="{}"/>
        </a:solidFill>
      </p:bgPr>
    </p:bg>
  </p:cSld>
  <p:typeLst/>
</p:sldMaster>"#,
            self.styles.background_color
        )
        .map_err(|e| e.to_string())?;

        // ppt/slideMasters/_rels/slideMaster1.xml.rels
        zip.start_file(
            "ppt/slideMasters/_rels/slideMaster1.xml.rels", <zip::write::FileOptions<()>>::default())
        .map_err(|e| e.to_string())?;
        write!(
            zip,
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdLay" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
  <Relationship Id="rIdThm" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/>
</Relationships>"#,
        )
        .map_err(|e| e.to_string())?;

        // ppt/slideLayouts/slideLayout1.xml
        zip.start_file(
            "ppt/slideLayouts/slideLayout1.xml", <zip::write::FileOptions<()>>::default())
        .map_err(|e| e.to_string())?;
        write!(
            zip,
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
             type="cust">
  <p:cSld name="Custom Layout">
    <p:spTree>
      <p:nvGrpSpPr>
        <p:cNvPr id="1" name=""/>
        <p:cNvGrpSpPr/>
        <p:nvPr/>
      </p:nvGrpSpPr>
      <p:grpSpPr/>
    </p:spTree>
  </p:cSld>
</p:sldLayout>"#,
        )
        .map_err(|e| e.to_string())?;

        // ppt/slideLayouts/_rels/slideLayout1.xml.rels
        zip.start_file(
            "ppt/slideLayouts/_rels/slideLayout1.xml.rels", <zip::write::FileOptions<()>>::default())
        .map_err(|e| e.to_string())?;
        write!(
            zip,
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdThm" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/>
</Relationships>"#,
        )
        .map_err(|e| e.to_string())?;

        // ppt/theme/theme1.xml
        zip.start_file("ppt/theme/theme1.xml", <zip::write::FileOptions<()>>::default())
            .map_err(|e| e.to_string())?;
        write!(
            zip,
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Default Theme">
  <a:themeElements>
    <a:clrScheme name="Default">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="{}"/></a:dk2>
      <a:lt2><a:srgbClr val="EEEEEE"/></a:lt2>
      <a:accent1><a:srgbClr val="{}"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="4472C4"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="Default">
      <a:majorFont><a:latin typeface="{}"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont>
      <a:minorFont><a:latin typeface="{}"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont>
    </a:fontScheme>
    <a:fmtScheme name="Default">
      <a:fillStyleLst>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
      </a:fillStyleLst>
      <a:lnStyleLst>
        <a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
        <a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
        <a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
      </a:lnStyleLst>
      <a:effectStyleLst>
        <a:effectStyle><a:effectLst/></a:effectStyle>
        <a:effectStyle><a:effectLst/></a:effectStyle>
        <a:effectStyle><a:effectLst/></a:effectStyle>
      </a:effectStyleLst>
      <a:bgFillStyleLst>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
      </a:bgFillStyleLst>
    </a:fmtScheme>
  </a:themeElements>
</a:theme>"#,
            self.styles.title_color,
            self.styles.accent_color,
            self.styles.heading_font,
            self.styles.default_font,
        )
        .map_err(|e| e.to_string())?;

        // ppt/presProps.xml
        zip.start_file("ppt/presProps.xml", <zip::write::FileOptions<()>>::default())
            .map_err(|e| e.to_string())?;
        write!(
            zip,
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentationPr xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"/>
"#,
        )
        .map_err(|e| e.to_string())?;

        // ppt/viewProps.xml
        zip.start_file("ppt/viewProps.xml", <zip::write::FileOptions<()>>::default())
            .map_err(|e| e.to_string())?;
        write!(
            zip,
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:viewPr xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:normalViewPr><p:restoredLeft sz="17067"/><p:restoredTop sz="90733"/></p:normalViewPr>
  <p:slideViewPr><p:cSldViewPr><p:cViewPrefs><p:showGuides val="0"/></p:cViewPrefs></p:cSldViewPr></p:slideViewPr>
  <p:notesTextViewPr><p:cSldViewPr/></p:notesTextViewPr>
  <p:gridSpacing cx="72000" cy="72000"/>
</p:viewPr>"#,
        )
        .map_err(|e| e.to_string())?;


        Ok(())
    }

    /// Writes docProps/core.xml and docProps/app.xml.
    pub(crate) fn write_doc_props(&self, zip: &mut ZipWriter<&mut std::io::Cursor<Vec<u8>>>) -> Result<(), String> {

        // docProps/core.xml
        zip.start_file("docProps/core.xml", <zip::write::FileOptions<()>>::default())
            .map_err(|e| e.to_string())?;
        write!(
            zip,
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/"
                   xmlns:dcterms="http://purl.org/dc/terms/"
                   xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <dc:creator>{}</dc:creator>
  <dc:title>{}</dc:title>
  <dc:subject>{}</dc:subject>
  <cp:keywords>presentation slides</cp:keywords>
  <dcterms:created xsi:type="dcterms:W3CDTF">{}</dcterms:created>
</cp:coreProperties>"#,
            self.author.as_deref().unwrap_or("General Bots"),
            self.title.as_deref().unwrap_or("Presentation"),
            self.subject.as_deref().unwrap_or(""),
            chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
        )
        .map_err(|e| e.to_string())?;

        // docProps/app.xml
        zip.start_file("docProps/app.xml", <zip::write::FileOptions<()>>::default())
            .map_err(|e| e.to_string())?;
        write!(
            zip,
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"
            xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">
  <Application>General Bots</Application>
  <Slides>{}</Slides>
  <PresentationFormat>Widescreen</PresentationFormat>
  <SlideCount>{}</SlideCount>
  <NotesSlideCount>{}</NotesSlideCount>
</Properties>"#,
            self.slides.len(),
            self.slides.len(),
            self.slides.iter().filter(|s| s.notes.is_some()).count(),
        )
        .map_err(|e| e.to_string())?;


        Ok(())

    }
}
