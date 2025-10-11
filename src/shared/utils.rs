use diesel::prelude::*;
use log::{debug, warn};
use rhai::{Array, Dynamic};
use serde_json::{json, Value};
use smartstring::SmartString;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tokio::fs::File as TokioFile;
use tokio_stream::StreamExt;
use zip::ZipArchive;

use crate::config::AIConfig;
use reqwest::Client;
use tokio::io::AsyncWriteExt;

pub fn extract_zip_recursive(
    zip_path: &Path,
    destination_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(zip_path)?;
    let buf_reader = BufReader::new(file);
    let mut archive = ZipArchive::new(buf_reader)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = destination_path.join(file.mangled_name());

        if file.is_dir() {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(&parent)?;
                }
            }
            let mut outfile = File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}

pub fn row_to_json(row: diesel::QueryResult<diesel::pg::PgRow>) -> Result<Value, Box<dyn Error>> {
    let row = row?;
    let mut result = serde_json::Map::new();
    let columns = row.columns();
    debug!("Converting row with {} columns", columns.len());

    for (i, column) in columns.iter().enumerate() {
        let column_name = column.name();
        let type_name = column.type_name();

        let value = match type_name {
            "INT4" | "int4" => handle_nullable_type::<i32>(&row, i, column_name),
            "INT8" | "int8" => handle_nullable_type::<i64>(&row, i, column_name),
            "FLOAT4" | "float4" => handle_nullable_type::<f32>(&row, i, column_name),
            "FLOAT8" | "float8" => handle_nullable_type::<f64>(&row, i, column_name),
            "TEXT" | "VARCHAR" | "text" | "varchar" => {
                handle_nullable_type::<String>(&row, i, column_name)
            }
            "BOOL" | "bool" => handle_nullable_type::<bool>(&row, i, column_name),
            "JSON" | "JSONB" | "json" | "jsonb" => handle_json(&row, i, column_name),
            _ => {
                warn!("Unknown type {} for column {}", type_name, column_name);
                handle_nullable_type::<String>(&row, i, column_name)
            }
        };

        result.insert(column_name.to_string(), value);
    }

    Ok(Value::Object(result))
}

fn handle_nullable_type<'r, T>(row: &'r diesel::pg::PgRow, idx: usize, col_name: &str) -> Value
where
    T: diesel::deserialize::FromSql<
            diesel::sql_types::Nullable<diesel::sql_types::Text>,
            diesel::pg::Pg,
        > + serde::Serialize
        + std::fmt::Debug,
{
    match row.get::<Option<T>, _>(idx) {
        Ok(Some(val)) => {
            debug!("Successfully read column {} as {:?}", col_name, val);
            json!(val)
        }
        Ok(None) => {
            debug!("Column {} is NULL", col_name);
            Value::Null
        }
        Err(e) => {
            warn!("Failed to read column {}: {}", col_name, e);
            Value::Null
        }
    }
}

fn handle_json(row: &diesel::pg::PgRow, idx: usize, col_name: &str) -> Value {
    match row.get::<Option<Value>, _>(idx) {
        Ok(Some(val)) => {
            debug!("Successfully read JSON column {} as Value", col_name);
            return val;
        }
        Ok(None) => return Value::Null,
        Err(_) => (),
    }

    match row.get::<Option<String>, _>(idx) {
        Ok(Some(s)) => match serde_json::from_str(&s) {
            Ok(val) => val,
            Err(_) => {
                debug!("Column {} contains string that's not JSON", col_name);
                json!(s)
            }
        },
        Ok(None) => Value::Null,
        Err(e) => {
            warn!("Failed to read JSON column {}: {}", col_name, e);
            Value::Null
        }
    }
}

pub fn json_value_to_dynamic(value: &Value) -> Dynamic {
    match value {
        Value::Null => Dynamic::UNIT,
        Value::Bool(b) => Dynamic::from(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::UNIT
            }
        }
        Value::String(s) => Dynamic::from(s.clone()),
        Value::Array(arr) => Dynamic::from(
            arr.iter()
                .map(json_value_to_dynamic)
                .collect::<rhai::Array>(),
        ),
        Value::Object(obj) => Dynamic::from(
            obj.iter()
                .map(|(k, v)| (SmartString::from(k), json_value_to_dynamic(v)))
                .collect::<rhai::Map>(),
        ),
    }
}

pub fn to_array(value: Dynamic) -> Array {
    if value.is_array() {
        value.cast::<Array>()
    } else if value.is_unit() || value.is::<()>() {
        Array::new()
    } else {
        Array::from([value])
    }
}

pub async fn download_file(url: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client.get(url).send().await?;

    if response.status().is_success() {
        let mut file = TokioFile::create(output_path).await?;

        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            file.write_all(&chunk?).await?;
        }
        debug!("File downloaded successfully to {}", output_path);
    } else {
        return Err("Failed to download file".into());
    }

    Ok(())
}

pub fn parse_filter(filter_str: &str) -> Result<(String, Vec<String>), Box<dyn Error>> {
    let parts: Vec<&str> = filter_str.split('=').collect();
    if parts.len() != 2 {
        return Err("Invalid filter format. Expected 'KEY=VALUE'".into());
    }

    let column = parts[0].trim();
    let value = parts[1].trim();

    if !column
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err("Invalid column name in filter".into());
    }

    Ok((format!("{} = $1", column), vec![value.to_string()]))
}

pub fn parse_filter_with_offset(
    filter_str: &str,
    offset: usize,
) -> Result<(String, Vec<String>), Box<dyn Error>> {
    let mut clauses = Vec::new();
    let mut params = Vec::new();

    for (i, condition) in filter_str.split('&').enumerate() {
        let parts: Vec<&str> = condition.split('=').collect();
        if parts.len() != 2 {
            return Err("Invalid filter format".into());
        }

        let column = parts[0].trim();
        let value = parts[1].trim();

        if !column
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err("Invalid column name".into());
        }

        clauses.push(format!("{} = ${}", column, i + 1 + offset));
        params.push(value.to_string());
    }

    Ok((clauses.join(" AND "), params))
}

pub async fn call_llm(prompt: &str, _ai_config: &AIConfig) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    Ok(format!("Generated response for: {}", prompt))
}
