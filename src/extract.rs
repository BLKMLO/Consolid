use calamine::{open_workbook_auto, Data, Reader};
use std::io;
use std::path::Path;
use thiserror::Error;

pub const MAX_INPUT_SIZE: u64 = 50 * 1024 * 1024;
const MAX_EXTRACTED_TEXT: usize = 20 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum ExtractError {
    #[error("fichier introuvable ou illisible : {0}")]
    Io(#[from] io::Error),
    #[error("format non pris en charge : seuls les classeurs Excel (.xlsx, .xls, .xlsb) sont acceptés ({0})")]
    Unsupported(String),
    #[error("fichier trop volumineux (maximum 50 Mio)")]
    TooLarge,
    #[error("contenu extrait trop volumineux (maximum 20 Mio)")]
    ExtractedTooLarge,
    #[error("classeur illisible : {0}")]
    Spreadsheet(String),
}

pub fn extract(path: &Path) -> Result<String, ExtractError> {
    let metadata = path.metadata()?;
    if !metadata.is_file() {
        return Err(ExtractError::Unsupported(
            "la source n'est pas un fichier".into(),
        ));
    }
    if metadata.len() > MAX_INPUT_SIZE {
        return Err(ExtractError::TooLarge);
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match extension.as_str() {
        "xlsx" | "xls" | "xlsb" => extract_workbook(path),
        _ => Err(ExtractError::Unsupported(extension)),
    }
}

fn extract_workbook(path: &Path) -> Result<String, ExtractError> {
    let mut workbook =
        open_workbook_auto(path).map_err(|error| ExtractError::Spreadsheet(error.to_string()))?;
    let mut output = String::new();
    let sheet_names = workbook.sheet_names().to_vec();

    for (sheet_index, sheet_name) in sheet_names.iter().enumerate() {
        let range = workbook
            .worksheet_range(sheet_name)
            .map_err(|error| ExtractError::Spreadsheet(error.to_string()))?;
        output.push_str(&format!("SHEET_{}\n", sheet_index + 1));
        let mut rows = range.rows();
        let headers: Vec<String> = rows
            .next()
            .unwrap_or_default()
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let value = cell_to_string(value);
                if value.trim().is_empty() {
                    format!("COLONNE_{}", index + 1)
                } else {
                    value
                }
            })
            .collect();

        for (row_index, row) in rows.enumerate() {
            output.push_str(&format!("ROW_{}\n", row_index + 1));
            for (column, value) in row.iter().enumerate() {
                let header = headers.get(column).map(String::as_str).unwrap_or("COLONNE");
                output.push_str(header);
                output.push_str(": ");
                output.push_str(&cell_to_string(value));
                output.push('\n');
            }
            ensure_text_limit(&output)?;
        }
    }
    Ok(output)
}

fn cell_to_string(value: &Data) -> String {
    match value {
        Data::Empty => String::new(),
        other => other.to_string(),
    }
}

fn ensure_text_limit(text: &str) -> Result<(), ExtractError> {
    if text.len() > MAX_EXTRACTED_TEXT {
        Err(ExtractError::ExtractedTooLarge)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn rejects_non_excel_formats() {
        for suffix in [".pdf", ".csv", ".json", ".txt", ".md", ".docx", ".ods", ".bin"] {
            let file = tempfile::Builder::new().suffix(suffix).tempfile().unwrap();
            assert!(
                matches!(extract(file.path()), Err(ExtractError::Unsupported(_))),
                "{suffix} devrait être refusé"
            );
        }
    }

    #[test]
    fn rejects_empty_extension() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("classeur");
        std::fs::write(&path, b"donnees").unwrap();
        assert!(matches!(extract(&path), Err(ExtractError::Unsupported(_))));
    }

    #[test]
    fn reports_io_error_for_missing_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("absent.xlsx");
        assert!(matches!(extract(&path), Err(ExtractError::Io(_))));
    }

    #[test]
    fn rejects_invalid_workbook_content() {
        let mut file = tempfile::Builder::new().suffix(".xlsx").tempfile().unwrap();
        write!(file, "pas un classeur").unwrap();
        assert!(matches!(
            extract(file.path()),
            Err(ExtractError::Spreadsheet(_))
        ));
    }
}
