use log::debug;
use rhai::{Array, Dynamic};
use serde_json::Value;
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

pub async fn call_llm(
    prompt: &str,
    _ai_config: &AIConfig,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    Ok(format!("Generated response for: {}", prompt))
}
