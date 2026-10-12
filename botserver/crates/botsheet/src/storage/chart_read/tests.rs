//! Tests for chart extraction from an xlsx package.
//!
//! Split out of `chart_read.rs` to keep that file inside the project size
//! limit; the parser itself stays beside its callers.


use super::*;

/// Builds a workbook carrying a bar chart, the same way a spreadsheet tool
/// writes one: categories on the header row, values underneath, and a
/// drawing part that points at the chart. Generating the package here keeps
/// the test self-contained instead of depending on a file left in `/tmp`.
fn workbook_with_chart() -> Vec<u8> {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("Planilha1").expect("set sheet name");
    worksheet.write(0, 0, "Mês").expect("write header");
    for (column, label) in ["Jan", "Fev", "Mar", "Abr"].iter().enumerate() {
        worksheet
            .write(0, column as u16 + 1, *label)
            .expect("write category");
    }
    worksheet.write(1, 0, "Vendas").expect("write series name");
    for (column, value) in [110.0, 150.0, 130.0, 170.0].iter().enumerate() {
        worksheet
            .write(1, column as u16 + 1, *value)
            .expect("write value");
    }

    let mut chart = rust_xlsxwriter::Chart::new(rust_xlsxwriter::ChartType::Bar);
    chart.title().set_name("Vendas por Mês");
    // Row 1 holds the categories, row 2 the values they label.
    chart
        .add_series()
        .set_categories("=Planilha1!$B$1:$E$1")
        .set_values("=Planilha1!$B$2:$E$2");
    worksheet.insert_chart(6, 1, &chart).expect("insert chart");

    workbook.save_to_buffer().expect("save workbook")
}

#[test]
fn extracts_chart_from_workbook_package() {
    let bytes = workbook_with_chart();

    let workbook = umya_spreadsheet::reader::xlsx::read_reader(
        std::io::Cursor::new(&bytes),
        true,
    )
    .expect("parse");
    let mut worksheets = Vec::new();
    for sheet in workbook.get_sheet_collection() {
        let mut data = HashMap::new();
        let (max_col, max_row) = sheet.get_highest_column_and_row();
        for row in 1..=max_row {
            for col in 1..=max_col {
                if let Some(cell) = sheet.get_cell((col, row)) {
                    let value = cell.get_value().to_string();
                    if !value.is_empty() {
                        data.insert(
                            format!("{},{}", row - 1, col - 1),
                            crate::types::CellData {
                                value: Some(value),
                                typed: None,
                                formula: None,
                                style: None,
                                format: None,
                                note: None,
                                locked: None,
                                has_comment: None,
                                array_formula_id: None,
                            },
                        );
                    }
                }
            }
        }
        worksheets.push(Worksheet {
            name: sheet.get_name().to_string(),
            data,
            ..Worksheet::default()
        });
    }
    let charts = extract_charts(&bytes, &worksheets).expect("extract");
    assert!(!charts.is_empty(), "no charts extracted at all");
    let ws0 = &charts[0];
    assert!(!ws0.is_empty(), "worksheet 0 has no charts");
    let c = &ws0[0];
    assert_eq!(c.chart_type, "bar");
    assert_eq!(c.title, "Vendas por Mês");
    assert_eq!(c.labels, vec!["Jan", "Fev", "Mar", "Abr"]);
    assert_eq!(c.datasets.len(), 1);
    assert_eq!(c.datasets[0].data, vec![110.0, 150.0, 130.0, 170.0]);
}
