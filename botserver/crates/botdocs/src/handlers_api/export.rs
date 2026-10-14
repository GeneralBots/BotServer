use crate::re_cache::cached_re;
use crate::state::DocState;
use crate::storage::{get_current_user_id, load_document_from_drive};
use crate::storage_core::{parse_html_to_paragraphs, ParagraphData, strip_html};
use crate::types::ExportQuery;
use crate::utils::html_to_markdown;
use crate::utils_ooxml::html_to_odt_zip;
use crate::utils_pdf::html_to_pdf;
use axum::{extract::{Query, State}, http::StatusCode, Json, response::IntoResponse};
use docx_rs::{AlignmentType, Docx, Paragraph, Run, RunFonts, Table, TableCell, TableRow};
use std::sync::Arc;

pub async fn handle_export_pdf(
    State(state): State<Arc<DocState>>,
    Query(query): Query<ExportQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let doc = load_export_document(&state, &query).await?;
    let pdf_bytes = html_to_pdf(&doc.content);

    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/pdf")],
        pdf_bytes,
    ))
}

pub async fn handle_export_docx(
    State(state): State<Arc<DocState>>,
    Query(query): Query<ExportQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let doc = load_export_document(&state, &query).await?;
    let docx_bytes = html_to_docx_full(&doc.title, &doc.content)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::export", None)}))))?;

    Ok((
        [(
            axum::http::header::CONTENT_TYPE,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        )],
        docx_bytes,
    ))
}

pub async fn handle_export_odt(
    State(state): State<Arc<DocState>>,
    Query(query): Query<ExportQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let doc = load_export_document(&state, &query).await?;
    let odt_bytes = html_to_odt_zip(&doc.title, &doc.content)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::export", None)}))))?;

    Ok((
        [(
            axum::http::header::CONTENT_TYPE,
            "application/vnd.oasis.opendocument.text",
        )],
        odt_bytes,
    ))
}

async fn load_export_document(
    state: &Arc<DocState>,
    query: &ExportQuery,
) -> Result<crate::types::Document, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();

    match load_document_from_drive(state, &user_id, &query.id).await {
        Ok(Some(d)) => Ok(d),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Document not found" })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::export", None)})),
        )),
    }
}

enum HtmlBlock {
    Fragment(String),
    Table(Vec<Vec<String>>),
    Image(String),
}

fn html_to_docx_full(title: &str, html: &str) -> Result<Vec<u8>, String> {
    let mut docx = Docx::new();

    if !title.is_empty() {
        let title_para = Paragraph::new()
            .add_run(Run::new().add_text(title).bold().size(48))
            .align(AlignmentType::Center);
        docx = docx.add_paragraph(title_para);
        docx = docx.add_paragraph(Paragraph::new());
    }

    for block in split_html_blocks(html) {
        match block {
            HtmlBlock::Table(rows) => {
                docx = docx.add_table(build_docx_table(&rows));
            }
            HtmlBlock::Image(alt) => {
                let label = if alt.is_empty() {
                    "[Image]".to_string()
                } else {
                    format!("[Image: {alt}]")
                };
                docx = docx.add_paragraph(Paragraph::new().add_run(Run::new().add_text(label).italic()));
            }
            HtmlBlock::Fragment(fragment) => {
                for p in parse_html_to_paragraphs(&fragment) {
                    docx = docx.add_paragraph(paragraph_from_data(&p));
                }
            }
        }
    }

    let mut buf = std::io::Cursor::new(Vec::new());
    docx.build()
        .pack(&mut buf)
        .map_err(|e| format!("Failed to build DOCX: {e}"))?;
    Ok(buf.into_inner())
}

fn paragraph_from_data(p: &ParagraphData) -> Paragraph {
    match p.style.as_str() {
        "h1" => Paragraph::new().add_run(Run::new().add_text(&p.text).bold().size(32)),
        "h2" => Paragraph::new().add_run(Run::new().add_text(&p.text).bold().size(28)),
        "h3" => Paragraph::new().add_run(Run::new().add_text(&p.text).bold().size(24)),
        "li" => Paragraph::new()
            .add_run(Run::new().add_text("• "))
            .add_run(Run::new().add_text(&p.text)),
        "blockquote" => Paragraph::new()
            .indent(Some(720), None, None, None)
            .add_run(Run::new().add_text(&p.text).italic()),
        "code" => Paragraph::new().add_run(
            Run::new()
                .add_text(&p.text)
                .fonts(RunFonts::new().ascii("Courier New")),
        ),
        _ => {
            let mut run = Run::new().add_text(&p.text);
            if p.bold {
                run = run.bold();
            }
            if p.italic {
                run = run.italic();
            }
            if p.underline {
                run = run.underline("single");
            }
            Paragraph::new().add_run(run)
        }
    }
}

fn split_html_blocks(html: &str) -> Vec<HtmlBlock> {
    let mut blocks = Vec::new();
    let mut pos = 0usize;

    while pos < html.len() {
        let table_start = html[pos..].find("<table").map(|r| pos + r);
        let img_start = html[pos..].find("<img").map(|r| pos + r);

        let next = match (table_start, img_start) {
            (Some(t), Some(i)) => Some((std::cmp::min(t, i), t <= i)),
            (Some(t), None) => Some((t, true)),
            (None, Some(i)) => Some((i, false)),
            (None, None) => None,
        };

        match next {
            None => {
                let rest = html[pos..].to_string();
                if !rest.trim().is_empty() {
                    blocks.push(HtmlBlock::Fragment(rest));
                }
                break;
            }
            Some((n, is_table)) => {
                let before = html[pos..n].to_string();
                if !before.trim().is_empty() {
                    blocks.push(HtmlBlock::Fragment(before));
                }

                if is_table {
                    let Some(end_rel) = html[n..].find("</table>") else {
                        blocks.push(HtmlBlock::Fragment(html[n..].to_string()));
                        break;
                    };
                    let end = n + end_rel + "</table>".len();
                    blocks.push(HtmlBlock::Table(parse_table_html(&html[n..end])));
                    pos = end;
                } else {
                    let Some(end_rel) = html[n..].find('>') else {
                        break;
                    };
                    let end = n + end_rel + 1;
                    blocks.push(HtmlBlock::Image(extract_img_alt(&html[n..end])));
                    pos = end;
                }
            }
        }
    }

    blocks
}

fn parse_table_html(table: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let Some(tr_re) = cached_re!(r"<tr[^>]*>(.*?)</tr>") else {
        return rows;
    };
    let Some(cell_re) = cached_re!(r"<t[dh][^>]*>(.*?)</t[dh]>") else {
        return rows;
    };

    for tr in tr_re.captures_iter(table) {
        let mut cells = Vec::new();
        for cell in cell_re.captures_iter(&tr[1]) {
            cells.push(strip_html(&cell[1]).trim().to_string());
        }
        if !cells.is_empty() {
            rows.push(cells);
        }
    }
    rows
}

fn build_docx_table(rows: &[Vec<String>]) -> Table {
    let doc_rows: Vec<TableRow> = rows
        .iter()
        .map(|cells| {
            let doc_cells: Vec<TableCell> = cells
                .iter()
                .map(|c| TableCell::new().add_paragraph(Paragraph::new().add_run(Run::new().add_text(c))))
                .collect();
            TableRow::new(doc_cells)
        })
        .collect();
    Table::new(doc_rows)
}

fn extract_img_alt(tag: &str) -> String {
    if let Some(pos) = tag.find("alt=\"") {
        let rest = &tag[pos + 5..];
        if let Some(end) = rest.find('"') {
            return rest[..end].to_string();
        }
    }
    String::new()
}

pub async fn handle_export_md(
    State(state): State<Arc<DocState>>,
    Query(query): Query<ExportQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();

    let doc = match load_document_from_drive(&state, &user_id, &query.id).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Document not found" })),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::export", None)})),
            ))
        }
    };

    let markdown = html_to_markdown(&doc.content);

    Ok(([(axum::http::header::CONTENT_TYPE, "text/markdown")], markdown))
}

pub async fn handle_export_html(
    State(state): State<Arc<DocState>>,
    Query(query): Query<ExportQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();

    let doc = match load_document_from_drive(&state, &user_id, &query.id).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Document not found" })),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::export", None)})),
            ))
        }
    };

    let full_html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{}</title>
<style>
body {{ font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }}
</style>
</head>
<body>
{}
</body>
</html>"#,
        doc.title, doc.content
    );

    Ok(([(axum::http::header::CONTENT_TYPE, "text/html")], full_html))
}

pub async fn handle_export_txt(
    State(state): State<Arc<DocState>>,
    Query(query): Query<ExportQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();

    let doc = match load_document_from_drive(&state, &user_id, &query.id).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Document not found" })),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::export", None)})),
            ))
        }
    };

    let plain_text = strip_html(&doc.content);

    Ok(([(axum::http::header::CONTENT_TYPE, "text/plain")], plain_text))
}
