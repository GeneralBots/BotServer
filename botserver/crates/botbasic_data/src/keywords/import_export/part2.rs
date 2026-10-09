use super::*;

#[cfg(feature = "sheet")]
pub(super) fn import_excel(file_path: &str) -> Result<Dynamic, Box<dyn std::error::Error + Send + Sync>> {
    use calamine::{open_workbook, Reader, Xlsx};

    let mut workbook: Xlsx<_> = open_workbook(file_path)?;

    let sheet_names: Vec<String> = workbook.sheet_names();
    if sheet_names.is_empty() {
        return Err("Excel file has no sheets".into());
    }

    let range = workbook
        .worksheet_range(&sheet_names[0])
        .map_err(|e| format!("Failed to read sheet: {}", e))?;

    let mut rows_iter = range.rows();

    let headers: Vec<String> = match rows_iter.next() {
        Some(row) => row
            .iter()
            .map(|cell| cell.to_string().trim().to_string())
            .collect(),
        None => return Err("Excel sheet is empty".into()),
    };

    let mut results: Array = Array::new();

    for row in rows_iter {
        let mut row_map: Map = Map::new();

        for (i, header) in headers.iter().enumerate() {
            let value = row.get(i).map(|cell| cell.to_string()).unwrap_or_default();
            row_map.insert(header.clone().into(), Dynamic::from(value));
        }

        results.push(Dynamic::from(row_map));
    }

    trace!("Imported {} rows from Excel", results.len());
    Ok(Dynamic::from(results))
}
pub(super) fn export_csv(
    file_path: &str,
    data: Dynamic,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let array = to_array(data);
    if array.is_empty() {
        return Err("No data to export".into());
    }

    let headers = get_headers_from_array(&array);

    let mut file = File::create(file_path)?;

    writeln!(file, "{}", headers.join(","))?;

    for item in array {
        let map = dynamic_to_map(&item);
        let values: Vec<String> = headers
            .iter()
            .map(|h| {
                let val = map.get(h).map(|v| v.to_string()).unwrap_or_default();
                escape_csv_value(&val)
            })
            .collect();
        writeln!(file, "{}", values.join(","))?;
    }

    trace!("Exported data to CSV: {}", file_path);
    Ok(file_path.to_string())
}
pub(super) fn export_tsv(
    file_path: &str,
    data: Dynamic,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let array = to_array(data);
    if array.is_empty() {
        return Err("No data to export".into());
    }

    let headers = get_headers_from_array(&array);

    let mut file = File::create(file_path)?;

    writeln!(file, "{}", headers.join("\t"))?;

    for item in array {
        let map = dynamic_to_map(&item);
        let values: Vec<String> = headers
            .iter()
            .map(|h| map.get(h).map(|v| v.to_string()).unwrap_or_default())
            .collect();
        writeln!(file, "{}", values.join("\t"))?;
    }

    trace!("Exported data to TSV: {}", file_path);
    Ok(file_path.to_string())
}
pub(super) fn export_json(
    file_path: &str,
    data: Dynamic,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let json_value = dynamic_to_json(&data);
    let json_string = serde_json::to_string_pretty(&json_value)?;

    std::fs::write(file_path, json_string)?;

    trace!("Exported data to JSON: {}", file_path);
    Ok(file_path.to_string())
}
#[cfg(feature = "sheet")]
pub(super) fn export_excel(
    file_path: &str,
    data: Dynamic,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    use rust_xlsxwriter::Workbook;

    let array = to_array(data);
    if array.is_empty() {
        return Err("No data to export".into());
    }

    let headers = get_headers_from_array(&array);

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();

    for (col, header) in headers.iter().enumerate() {
        sheet.write_string(0, col as u16, header)?;
    }

    for (row_idx, item) in array.iter().enumerate() {
        let map = dynamic_to_map(item);
        for (col, header) in headers.iter().enumerate() {
            let value = map.get(header).map(|v| v.to_string()).unwrap_or_default();
            sheet.write_string((row_idx + 1) as u32, col as u16, &value)?;
        }
    }

    workbook.save(file_path)?;

    trace!("Exported data to Excel: {}", file_path);
    Ok(file_path.to_string())
}
pub(super) fn parse_csv_line(line: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in line.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
            }
            ',' if !in_quotes => {
                result.push(current.trim().to_string());
                current = String::new();
            }
            _ => {
                current.push(ch);
            }
        }
    }

    result.push(current.trim().to_string());
    result
}
pub(super) fn escape_csv_value(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        value.replace('"', "").to_string()
    } else {
        value.to_string()
    }
}
pub(super) fn to_array(data: Dynamic) -> Array {
    if data.is_array() {
        data.cast::<Array>()
    } else {
        vec![data]
    }
}
pub(super) fn dynamic_to_map(data: &Dynamic) -> HashMap<String, Dynamic> {
    let mut result = HashMap::new();

    if data.is_map() {
        let map = data.clone().cast::<Map>();
        for (k, v) in map.iter() {
            result.insert(k.to_string(), v.clone());
        }
    }

    result
}
pub(super) fn get_headers_from_array(array: &Array) -> Vec<String> {
    let mut headers = Vec::new();

    if let Some(first) = array.first() {
        let map = dynamic_to_map(first);
        headers = map.keys().cloned().collect();
        headers.sort();
    }

    headers
}
pub(super) fn json_to_dynamic(value: &Value) -> Dynamic {
    match value {
        Value::Null => Dynamic::UNIT,
        Value::Bool(b) => Dynamic::from(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::from(n.to_string())
            }
        }
        Value::String(s) => Dynamic::from(s.clone()),
        Value::Array(arr) => {
            let rhai_arr: Array = arr.iter().map(json_to_dynamic).collect();
            Dynamic::from(rhai_arr)
        }
        Value::Object(obj) => {
            let mut map = Map::new();
            for (k, v) in obj {
                map.insert(k.clone().into(), json_to_dynamic(v));
            }
            Dynamic::from(map)
        }
    }
}
pub(super) fn dynamic_to_json(data: &Dynamic) -> Value {
    if data.is_unit() {
        Value::Null
    } else if data.is_bool() {
        Value::Bool(data.as_bool().unwrap_or(false))
    } else if data.is_int() {
        Value::Number(serde_json::Number::from(data.as_int().unwrap_or(0)))
    } else if data.is_float() {
        if let Some(n) = serde_json::Number::from_f64(data.as_float().unwrap_or(0.0)) {
            Value::Number(n)
        } else {
            Value::Null
        }
    } else if data.is_string() {
        Value::String(data.to_string())
    } else if data.is_array() {
        let arr = data.clone().cast::<Array>();
        Value::Array(arr.iter().map(dynamic_to_json).collect())
    } else if data.is_map() {
        let map = data.clone().cast::<Map>();
        let mut obj = serde_json::Map::new();
        for (k, v) in map.iter() {
            obj.insert(k.to_string(), dynamic_to_json(v));
        }
        Value::Object(obj)
    } else {
        Value::String(data.to_string())
    }
}
