//! CSV/Excel import-export for the CASIROS CLI.
//!
//! This module converts between the JSON payloads used by the engine/simulator
//! and tabular file formats. CSV files use a simple two-column layout;
//! Excel files mirror the same data on the first worksheet.
//!
//! Supported conversions:
//!
//! - CSV/Excel **import** → JSON inputs map (`{ "node": "value", ... }`).
//! - JSON **export** → CSV/Excel. Detects `EvaluateResponse` (uses an
//!   `outputs` map) or `SimulateResponse` (uses `count/mean/median/min/max`).
//!   Any other top-level object is treated as an inputs map.

use std::collections::HashMap;
use std::path::Path;

use casiros_core::prelude::Decimal;
use serde_json::Value;

use crate::commands::CliError;

/// Converts a file from one supported format to another.
///
/// The input and output formats are inferred from file extensions:
/// `.csv`, `.xlsx`, and `.json` are supported.
///
/// # Errors
///
/// Returns [`CliError::Convert`] if reading, parsing, writing, or format
/// detection fails.
pub(crate) fn convert(input: &Path, output: &Path) -> Result<String, CliError> {
    let input_ext = extension_lower(input)?;
    let output_ext = extension_lower(output)?;

    let value: Value = match input_ext.as_str() {
        "json" => read_json(input)?,
        "csv" => read_csv(input)?,
        "xlsx" => read_excel(input)?,
        _ => {
            return Err(CliError::Convert {
                path: input.display().to_string(),
                message: format!("unsupported input format: {input_ext}"),
            });
        }
    };

    match output_ext.as_str() {
        "json" => write_json(output, &value)?,
        "csv" => write_csv(output, &value)?,
        "xlsx" => write_excel(output, &value)?,
        _ => {
            return Err(CliError::Convert {
                path: output.display().to_string(),
                message: format!("unsupported output format: {output_ext}"),
            });
        }
    }

    return Ok(format!(
        "Converted {} to {}",
        input.display(),
        output.display()
    ));
}

/// Reads a JSON file into a generic [`serde_json::Value`].
fn read_json(path: &Path) -> Result<Value, CliError> {
    let text = std::fs::read_to_string(path).map_err(|err| CliError::Read {
        path: path.display().to_string(),
        source: err,
    })?;
    return serde_json::from_str(&text).map_err(|err| CliError::Json {
        path: path.display().to_string(),
        source: err,
    });
}

/// Writes a [`serde_json::Value`] to a JSON file.
fn write_json(path: &Path, value: &Value) -> Result<(), CliError> {
    let text = serde_json::to_string_pretty(value).map_err(|err| CliError::Json {
        path: path.display().to_string(),
        source: err,
    })?;
    std::fs::write(path, text).map_err(|err| CliError::Write {
        path: path.display().to_string(),
        source: err,
    })?;
    return Ok(());
}

/// Reads a CSV file into a JSON object map.
///
/// The CSV must contain at least two columns. Rows with fewer than two cells
/// are skipped; additional cells are ignored.
#[cfg(feature = "csv")]
fn read_csv(path: &Path) -> Result<Value, CliError> {
    let file = std::fs::File::open(path).map_err(|err| CliError::Read {
        path: path.display().to_string(),
        source: err,
    })?;
    let mut reader = csv::ReaderBuilder::new().flexible(true).from_reader(file);
    let mut map = HashMap::<String, Decimal>::new();

    for result in reader.records() {
        let record = result.map_err(|err| CliError::Convert {
            path: path.display().to_string(),
            message: err.to_string(),
        })?;
        if record.len() < 2 {
            continue;
        }
        let key = record[0].trim().to_string();
        if key.is_empty() {
            continue;
        }
        let raw = record[1].trim().to_string();
        if (key == "node" || key == "metric") && raw == "value" {
            continue;
        }
        let value = raw.parse::<Decimal>().map_err(|err| CliError::Convert {
            path: path.display().to_string(),
            message: format!("invalid decimal for '{key}': {err}"),
        })?;
        map.insert(key, value);
    }

    return serde_json::to_value(map).map_err(|err| CliError::Json {
        path: path.display().to_string(),
        source: err,
    });
}

/// Writes a JSON value to a CSV file.
#[cfg(feature = "csv")]
fn write_csv(path: &Path, value: &Value) -> Result<(), CliError> {
    let rows = rows_from_value(value, path)?;
    let file = std::fs::File::create(path).map_err(|err| CliError::Write {
        path: path.display().to_string(),
        source: err,
    })?;
    let mut writer = csv::Writer::from_writer(file);
    writer
        .write_record([rows.header.0, rows.header.1])
        .map_err(|err| CliError::Convert {
            path: path.display().to_string(),
            message: err.to_string(),
        })?;
    for (key, value_string) in rows.entries {
        writer
            .write_record([&key, &value_string])
            .map_err(|err| CliError::Convert {
                path: path.display().to_string(),
                message: err.to_string(),
            })?;
    }
    writer.flush().map_err(|err| CliError::Write {
        path: path.display().to_string(),
        source: err,
    })?;
    return Ok(());
}

/// Reads the first worksheet of an Excel workbook into a JSON object map.
#[cfg(feature = "excel")]
fn read_excel(path: &Path) -> Result<Value, CliError> {
    use calamine::{Reader, open_workbook_auto};

    let mut workbook = open_workbook_auto(path).map_err(|err| CliError::Convert {
        path: path.display().to_string(),
        message: err.to_string(),
    })?;
    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| CliError::Convert {
            path: path.display().to_string(),
            message: "workbook has no sheets".to_string(),
        })?;
    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|err| CliError::Convert {
            path: path.display().to_string(),
            message: err.to_string(),
        })?;

    let mut map = HashMap::<String, Decimal>::new();
    for row in range.rows() {
        if row.len() < 2 {
            continue;
        }
        let key = cell_to_string(&row[0]);
        if key.is_empty() {
            continue;
        }
        let raw = cell_to_string(&row[1]);
        if (key == "node" || key == "metric") && raw == "value" {
            continue;
        }
        let value = raw.parse::<Decimal>().map_err(|err| CliError::Convert {
            path: path.display().to_string(),
            message: format!("invalid decimal for '{key}': {err}"),
        })?;
        map.insert(key, value);
    }

    return serde_json::to_value(map).map_err(|err| CliError::Json {
        path: path.display().to_string(),
        source: err,
    });
}

/// Writes a single value into an Excel worksheet cell.
#[cfg(feature = "excel")]
fn write_excel_cell(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    col: u16,
    value: &str,
    path: &Path,
) -> Result<(), CliError> {
    worksheet
        .write(row, col, value)
        .map_err(|err: rust_xlsxwriter::XlsxError| CliError::Convert {
            path: path.display().to_string(),
            message: err.to_string(),
        })?;
    return Ok(());
}

/// Writes a JSON value to the first worksheet of an Excel workbook.
#[cfg(feature = "excel")]
fn write_excel(path: &Path, value: &Value) -> Result<(), CliError> {
    use rust_xlsxwriter::Workbook;

    let rows = rows_from_value(value, path)?;
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    write_excel_cell(worksheet, 0, 0, rows.header.0, path)?;
    write_excel_cell(worksheet, 0, 1, rows.header.1, path)?;

    for (idx, (key, value_string)) in rows.entries.iter().enumerate() {
        let row = u32::try_from(idx + 1).map_err(|_| CliError::Convert {
            path: path.display().to_string(),
            message: "too many rows for Excel worksheet".to_string(),
        })?;
        write_excel_cell(worksheet, row, 0, key.as_str(), path)?;
        write_excel_cell(worksheet, row, 1, value_string.as_str(), path)?;
    }

    workbook.save(path).map_err(|err| CliError::Write {
        path: path.display().to_string(),
        source: std::io::Error::other(err.to_string()),
    })?;
    return Ok(());
}

/// Converts a calamine cell to a trimmed string representation.
#[cfg(feature = "excel")]
fn cell_to_string(cell: &calamine::Data) -> String {
    return match cell {
        calamine::Data::Float(v) => format!("{v}"),
        calamine::Data::Int(v) => format!("{v}"),
        calamine::Data::String(s) => s.trim().to_string(),
        _ => cell.to_string().trim().to_string(),
    };
}

/// A normalized set of rows ready for tabular output.
struct Rows {
    /// CSV/Excel column header tuple.
    header: (&'static str, &'static str),
    /// Ordered key/value pairs as strings.
    entries: Vec<(String, String)>,
}

/// Extracts a tabular representation from a known JSON response shape.
fn rows_from_value(value: &Value, path: &Path) -> Result<Rows, CliError> {
    if let Some(outputs) = value.get("outputs").and_then(Value::as_object) {
        let mut entries = Vec::with_capacity(outputs.len());
        for (key, value) in outputs {
            entries.push((key.clone(), decimal_string(value, path)?));
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        return Ok(Rows {
            header: ("node", "value"),
            entries,
        });
    }

    if value.get("count").is_some() {
        let fields = [
            ("count", "count"),
            ("mean", "mean"),
            ("median", "median"),
            ("min", "min"),
            ("max", "max"),
        ];
        let mut entries = Vec::new();
        for (json_key, label) in fields {
            let field_value = value.get(json_key).ok_or_else(|| CliError::Convert {
                path: path.display().to_string(),
                message: format!("missing simulation field '{json_key}'"),
            })?;
            entries.push((label.to_string(), decimal_string(field_value, path)?));
        }
        return Ok(Rows {
            header: ("metric", "value"),
            entries,
        });
    }

    let object = value.as_object().ok_or_else(|| CliError::Convert {
        path: path.display().to_string(),
        message: "expected a JSON object".to_string(),
    })?;
    let mut entries = Vec::with_capacity(object.len());
    for (key, value) in object {
        entries.push((key.clone(), decimal_string(value, path)?));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    return Ok(Rows {
        header: ("node", "value"),
        entries,
    });
}

/// Extracts a clean decimal string from a JSON number or string value.
fn decimal_string(value: &Value, path: &Path) -> Result<String, CliError> {
    let text = match value {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        _ => {
            return Err(CliError::Convert {
                path: path.display().to_string(),
                message: format!("expected a numeric value, got {value}"),
            });
        }
    };
    let _decimal = text.parse::<Decimal>().map_err(|err| CliError::Convert {
        path: path.display().to_string(),
        message: format!("invalid decimal '{text}': {err}"),
    })?;
    return Ok(text);
}

/// Returns the lower-cased extension of `path`, or an error if it is missing.
fn extension_lower(path: &Path) -> Result<String, CliError> {
    return path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_lowercase)
        .ok_or_else(|| CliError::Convert {
            path: path.display().to_string(),
            message: "could not determine file extension".to_string(),
        });
}
