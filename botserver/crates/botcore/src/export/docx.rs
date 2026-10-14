use std::io::Write;
use zip::ZipWriter;

pub struct DocxDocument {
    pub paragraphs: Vec<DocxParagraph>,
    pub tables: Vec<DocxTable>,
    pub images: Vec<DocxImage>,
    pub styles: DocxStyles,
}

pub struct DocxParagraph {
    pub text: String,
    pub style: String,
    pub alignment: Option<String>,
    pub font_size: Option<u32>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: Option<String>,
    pub num_id: Option<u32>,
}

pub struct DocxTable {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub header_style: Option<String>,
}

pub struct DocxImage {
    pub data: Vec<u8>,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
    pub alt_text: Option<String>,
}

pub struct DocxStyles {
    pub default_font: String,
    pub heading_font: String,
    pub font_size: u32,
    pub line_spacing: f32,
    pub margins: Margins,
    pub page_size: PageSize,
    pub header_text: Option<String>,
    pub footer_text: Option<String>,
    pub page_numbers: bool,
}

pub struct Margins {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

pub struct PageSize {
    pub width: u32,
    pub height: u32,
}

impl Default for DocxStyles {
    fn default() -> Self {
        Self {
            default_font: "Calibri".to_string(),
            heading_font: "Calibri Light".to_string(),
            font_size: 22,
            line_spacing: 1.15,
            margins: Margins { top: 1440, right: 1440, bottom: 1440, left: 1440 },
            page_size: PageSize { width: 12240, height: 15840 },
            header_text: None,
            footer_text: None,
            page_numbers: true,
        }
    }
}

impl Default for DocxDocument {
    fn default() -> Self {
        Self::new()
    }
}

impl DocxDocument {
    pub fn new() -> Self {
        Self {
            paragraphs: Vec::new(),
            tables: Vec::new(),
            images: Vec::new(),
            styles: DocxStyles::default(),
        }
    }

    pub fn add_paragraph(&mut self, text: &str, style: Option<&str>) {
        self.paragraphs.push(DocxParagraph {
            text: text.to_string(),
            style: style.unwrap_or("Normal").to_string(),
            alignment: None,
            font_size: None,
            bold: false,
            italic: false,
            underline: false,
            color: None,
            num_id: None,
        });
    }

    pub fn add_heading(&mut self, text: &str, level: u32) {
        self.paragraphs.push(DocxParagraph {
            text: text.to_string(),
            style: format!("Heading {}", level),
            alignment: None,
            font_size: Some(if level == 1 { 48 } else if level == 2 { 36 } else { 28 }),
            bold: true,
            italic: false,
            underline: false,
            color: None,
            num_id: None,
        });
    }

    pub fn add_table(&mut self, headers: Vec<String>, rows: Vec<Vec<String>>) {
        self.tables.push(DocxTable {
            headers,
            rows,
            header_style: None,
        });
    }

    pub fn add_image(&mut self, data: Vec<u8>, mime: &str, width: u32, height: u32) {
        self.images.push(DocxImage {
            data,
            mime_type: mime.to_string(),
            width,
            height,
            alt_text: None,
        });
    }

    pub fn export(&self) -> Result<Vec<u8>, String> {
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(&mut buf);

        zip.start_file("[Content_Types].xml", <zip::write::FileOptions<()>>::default()).map_err(|e| e.to_string())?;
        write!(zip, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
  <Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/>
  <Override PartName="/word/footer1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"/>
</Types>"#).map_err(|e| e.to_string())?;

        zip.start_file("_rels/.rels", <zip::write::FileOptions<()>>::default()).map_err(|e| e.to_string())?;
        write!(zip, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#).map_err(|e| e.to_string())?;

        zip.start_file("word/_rels/document.xml.rels", <zip::write::FileOptions<()>>::default()).map_err(|e| e.to_string())?;
        let mut rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer1.xml"/>
"#.to_string();
        for (i, _img) in self.images.iter().enumerate() {
            rels.push_str(&format!(r#"  <Relationship Id="rImage{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image{}.png"/>"#, i + 1, i + 1));
        }
        rels.push_str("</Relationships>");
        write!(zip, "{}", rels).map_err(|e| e.to_string())?;

        zip.start_file("word/styles.xml", <zip::write::FileOptions<()>>::default()).map_err(|e| e.to_string())?;
        write!(zip, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal">
    <w:name w:val="Normal"/>
    <w:pPr><w:spacing w:line="{}" w:lineRule="auto"/></w:pPr>
    <w:rPr><w:rFonts w:ascii="{}" w:hAnsi="{}"/><w:sz w:val="{}/>"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading1">
    <w:name w:val="Heading 1"/>
    <w:pPr><w:spacing w:before="480" w:after="240"/></w:pPr>
    <w:rPr><w:rFonts w:ascii="{}" w:hAnsi="{}"/><w:b/><w:sz w:val="48"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading2">
    <w:name w:val="Heading 2"/>
    <w:pPr><w:spacing w:before="360" w:after="120"/></w:pPr>
    <w:rPr><w:rFonts w:ascii="{}" w:hAnsi="{}"/><w:b/><w:sz w:val="36"/></w:rPr>
  </w:style>
</w:styles>"#,
            (self.styles.line_spacing * 240.0) as u32,
            self.styles.default_font, self.styles.default_font, self.styles.font_size,
            self.styles.heading_font, self.styles.heading_font,
            self.styles.heading_font, self.styles.heading_font,
        ).map_err(|e| e.to_string())?;

        zip.start_file("word/document.xml", <zip::write::FileOptions<()>>::default()).map_err(|e| e.to_string())?;
        write!(zip, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body>
    <w:sectPr>
      <w:pgSz w:w="{}" w:h="{}"/>
      <w:pgMar w:top="{}" w:right="{}" w:bottom="{}" w:left="{}"/>
    </w:sectPr>"#,
            self.styles.page_size.width, self.styles.page_size.height,
            self.styles.margins.top, self.styles.margins.right,
            self.styles.margins.bottom, self.styles.margins.left,
        ).map_err(|e| e.to_string())?;

        for p in &self.paragraphs {
            let align_xml = match p.alignment.as_deref() {
                Some("center") => r#"<w:jc w:val="center"/>"#,
                Some("right") => r#"<w:jc w:val="right"/>"#,
                Some("justify") => r#"<w:jc w:val="both"/>"#,
                _ => "",
            };
            let bold_xml = if p.bold { r#"<w:b/>"# } else { "" };
            let italic_xml = if p.italic { r#"<w:i/>"# } else { "" };
            let sz_xml = p.font_size.map(|s| format!(r#"<w:sz w:val="{}"/>"#, s)).unwrap_or_default();
            let num_id_xml = p.num_id.map(|n| format!(r#"<w:numPr><w:ilvl w:val="0"/><w:numId w:val="{}"/></w:numPr>"#, n)).unwrap_or_default();
            let escaped = p.text.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace("\"", "&quot;");
            write!(zip, r#"<w:p>
      <w:pPr>{align_xml}{num_id_xml}</w:pPr>
      <w:r>
        <w:rPr>{bold_xml}{italic_xml}{sz_xml}</w:rPr>
        <w:t xml:space="preserve">{escaped}</w:t>
      </w:r>
    </w:p>"#,
                align_xml = align_xml,
                num_id_xml = num_id_xml,
                bold_xml = bold_xml,
                italic_xml = italic_xml,
                sz_xml = sz_xml,
                escaped = escaped,
            ).map_err(|e| e.to_string())?;
        }

        for t in &self.tables {
            write!(zip, r#"<w:tbl><w:tblPr><w:tblBorders>
        <w:top w:val="single" w:sz="4" w:space="0" w:color="auto"/>
        <w:bottom w:val="single" w:sz="4" w:space="0" w:color="auto"/>
        <w:left w:val="single" w:sz="4" w:space="0" w:color="auto"/>
        <w:right w:val="single" w:sz="4" w:space="0" w:color="auto"/>
        <w:insideH w:val="single" w:sz="4" w:space="0" w:color="auto"/>
        <w:insideV w:val="single" w:sz="4" w:space="0" w:color="auto"/>
      </w:tblBorders></w:tblPr><w:tblGrid>"#).map_err(|e| e.to_string())?;
            for _ in &t.headers {
                write!(zip, r#"<w:gridCol w:w="2000"/>"#).map_err(|e| e.to_string())?;
            }
            write!(zip, r#"</w:tblGrid>"#).map_err(|e| e.to_string())?;

            write!(zip, r#"<w:tr><w:trPr><w:shd w:val="clear" w:color="auto" w:fill="D9E2F3"/><w:tblHeader/></w:trPr>"#).map_err(|e| e.to_string())?;
            for h in &t.headers {
                let escaped = h.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;");
                write!(zip, r#"<w:tc><w:p><w:r><w:rPr><w:b/><w:sz w:val="20"/></w:rPr><w:t>{}</w:t></w:r></w:p></w:tc>"#, escaped).map_err(|e| e.to_string())?;
            }
            write!(zip, r#"</w:tr>"#).map_err(|e| e.to_string())?;

            for row in &t.rows {
                write!(zip, r#"<w:tr>"#).map_err(|e| e.to_string())?;
                for cell in row {
                    let escaped = cell.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;");
                    write!(zip, r#"<w:tc><w:p><w:r><w:t>{}</w:t></w:r></w:p></w:tc>"#, escaped).map_err(|e| e.to_string())?;
                }
                write!(zip, r#"</w:tr>"#).map_err(|e| e.to_string())?;
            }
            write!(zip, r#"</w:tbl>"#).map_err(|e| e.to_string())?;
        }

        write!(zip, r#"</w:body></w:document>"#).map_err(|e| e.to_string())?;

        for (i, img) in self.images.iter().enumerate() {
            let path = format!("word/media/image{}.png", i + 1);
            zip.start_file(&path, <zip::write::FileOptions<()>>::default()).map_err(|e| e.to_string())?;
            std::io::copy(&mut std::io::Cursor::new(&img.data), &mut zip).map_err(|e| e.to_string())?;
        }

        if let Some(ref hdr) = self.styles.header_text {
            zip.start_file("word/header1.xml", <zip::write::FileOptions<()>>::default()).map_err(|e| e.to_string())?;
            let escaped = hdr.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;");
            write!(zip, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:p><w:r><w:t>{}</w:t></w:r></w:p></w:hdr>"#, escaped).map_err(|e| e.to_string())?;
        }

        if let Some(ref ftr) = self.styles.footer_text {
            zip.start_file("word/footer1.xml", <zip::write::FileOptions<()>>::default()).map_err(|e| e.to_string())?;
            let escaped = ftr.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;");
            write!(zip, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:ftr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:p><w:r><w:t>{}</w:t></w:r></w:p></w:ftr>"#, escaped).map_err(|e| e.to_string())?;
        }

        zip.finish().map_err(|e| e.to_string())?;
        Ok(buf.into_inner())
    }
}

pub fn from_html(html: &str) -> Result<DocxDocument, String> {
    let mut doc = DocxDocument::new();
    let re_heading = crate::re_cache::cached_re!(r"<h([1-6])[^>]*>(.*?)</h[1-6]>")
        .ok_or_else(|| "invalid heading regex".to_string())?;
    let re_para = crate::re_cache::cached_re!(r"<p[^>]*>(.*?)</p>")
        .ok_or_else(|| "invalid paragraph regex".to_string())?;
    let re_table = crate::re_cache::cached_re!(r"<table[^>]*>(.*?)</table>")
        .ok_or_else(|| "invalid table regex".to_string())?;
    let re_tr = crate::re_cache::cached_re!(r"<tr[^>]*>(.*?)</tr>")
        .ok_or_else(|| "invalid row regex".to_string())?;
    let re_th = crate::re_cache::cached_re!(r"<th[^>]*>(.*?)</th>")
        .ok_or_else(|| "invalid header cell regex".to_string())?;
    let re_td = crate::re_cache::cached_re!(r"<td[^>]*>(.*?)</td>")
        .ok_or_else(|| "invalid cell regex".to_string())?;
    let re_tag = crate::re_cache::cached_re!(r"<[^>]+>")
        .ok_or_else(|| "invalid tag regex".to_string())?;

    if let Some(tbl) = re_table.find(html) {
        let tbl_html = tbl.as_str();
        let mut headers = Vec::new();
        let mut rows: Vec<Vec<String>> = Vec::new();
        for tr in re_tr.captures_iter(tbl_html) {
            let tr_html = &tr[1];
            if let Some(th) = re_th.captures(tr_html) {
                headers.push(re_tag.replace_all(&th[1], "").to_string().trim().to_string());
            } else {
                let mut cells = Vec::new();
                for td in re_td.captures_iter(tr_html) {
                    cells.push(re_tag.replace_all(&td[1], "").to_string().trim().to_string());
                }
                if !cells.is_empty() {
                    rows.push(cells);
                }
            }
        }
        doc.add_table(headers, rows);
    }

    for cap in re_heading.captures_iter(html) {
        let level: u32 = cap[1].parse().unwrap_or(1);
        let text = re_tag.replace_all(&cap[2], "").to_string().trim().to_string();
        doc.add_heading(&text, level);
    }

    for cap in re_para.captures_iter(html) {
        let text = re_tag.replace_all(&cap[1], "").to_string().trim().to_string();
        if !text.is_empty() {
            doc.add_paragraph(&text, None);
        }
    }

    Ok(doc)
}
