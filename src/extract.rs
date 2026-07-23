use calamine::{open_workbook_auto, Data, Reader};
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use thiserror::Error;
use zip::ZipArchive;

pub const MAX_INPUT_SIZE: u64 = 50 * 1024 * 1024;
const MAX_EXTRACTED_TEXT: usize = 20 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum ExtractError {
    #[error("fichier introuvable ou illisible : {0}")]
    Io(#[from] io::Error),
    #[error("format non pris en charge : {0}")]
    Unsupported(String),
    #[error("les PDF ne sont pas pris en charge ; exportez-les en DOCX, CSV ou texte")]
    PdfUnsupported,
    #[error("fichier trop volumineux (maximum 50 Mio)")]
    TooLarge,
    #[error("contenu extrait trop volumineux (maximum 20 Mio)")]
    ExtractedTooLarge,
    #[error("contenu texte non UTF-8")]
    InvalidUtf8,
    #[error("classeur illisible : {0}")]
    Spreadsheet(String),
    #[error("document DOCX invalide : {0}")]
    Docx(String),
    #[error("CSV invalide : {0}")]
    Csv(String),
}

#[derive(Clone, Debug)]
pub struct SourceDocument {
    pub label: String,
    pub text: String,
}

pub fn extract_batch(paths: &[PathBuf]) -> Result<Vec<SourceDocument>, ExtractError> {
    paths
        .iter()
        .enumerate()
        .map(|(index, path)| {
            Ok(SourceDocument {
                label: format!("SOURCE_{:03}", index + 1),
                text: extract(path)?,
            })
        })
        .collect()
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
        "txt" | "md" | "json" => read_utf8(path),
        "csv" => extract_csv(path),
        "xlsx" | "xls" | "xlsb" | "ods" => extract_workbook(path),
        "docx" => extract_docx(path),
        "pdf" => Err(ExtractError::PdfUnsupported),
        _ => Err(ExtractError::Unsupported(extension)),
    }
}

fn read_utf8(path: &Path) -> Result<String, ExtractError> {
    let bytes = std::fs::read(path)?;
    if bytes.len() > MAX_EXTRACTED_TEXT {
        return Err(ExtractError::ExtractedTooLarge);
    }
    String::from_utf8(bytes).map_err(|_| ExtractError::InvalidUtf8)
}

fn extract_csv(path: &Path) -> Result<String, ExtractError> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(path)
        .map_err(|error| ExtractError::Csv(error.to_string()))?;
    let headers = reader
        .headers()
        .map_err(|error| ExtractError::Csv(error.to_string()))?
        .clone();
    let mut output = String::new();

    for (row_index, row) in reader.records().enumerate() {
        let row = row.map_err(|error| ExtractError::Csv(error.to_string()))?;
        output.push_str(&format!("ROW_{}\n", row_index + 1));
        for (column, value) in row.iter().enumerate() {
            let header = headers
                .get(column)
                .filter(|value| !value.trim().is_empty())
                .map(str::trim)
                .unwrap_or("COLONNE");
            output.push_str(header);
            output.push_str(": ");
            output.push_str(value);
            output.push('\n');
        }
        ensure_text_limit(&output)?;
    }
    Ok(output)
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

fn extract_docx(path: &Path) -> Result<String, ExtractError> {
    let file = File::open(path)?;
    let mut archive =
        ZipArchive::new(file).map_err(|error| ExtractError::Docx(error.to_string()))?;
    let mut entry = archive
        .by_name("word/document.xml")
        .map_err(|error| ExtractError::Docx(error.to_string()))?;
    if entry.size() as usize > MAX_EXTRACTED_TEXT {
        return Err(ExtractError::ExtractedTooLarge);
    }

    let mut xml = String::new();
    entry
        .read_to_string(&mut xml)
        .map_err(|error| ExtractError::Docx(error.to_string()))?;
    let text = docx_xml_to_text(&xml);
    ensure_text_limit(&text)?;
    Ok(text)
}

fn docx_xml_to_text(xml: &str) -> String {
    let with_breaks = xml
        .replace("</w:p>", "\n")
        .replace("<w:tab/>", "\t")
        .replace("<w:br/>", "\n")
        .replace("<w:br />", "\n");
    let mut output = String::with_capacity(with_breaks.len());
    let mut inside_tag = false;
    for character in with_breaks.chars() {
        match character {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if !inside_tag => output.push(character),
            _ => {}
        }
    }
    output
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
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
    fn rejects_pdf_explicitly() {
        let file = tempfile::Builder::new().suffix(".pdf").tempfile().unwrap();
        assert!(matches!(
            extract(file.path()),
            Err(ExtractError::PdfUnsupported)
        ));
    }

    #[test]
    fn rejects_unknown_extension() {
        let file = tempfile::Builder::new().suffix(".bin").tempfile().unwrap();
        assert!(matches!(
            extract(file.path()),
            Err(ExtractError::Unsupported(_))
        ));
    }

    #[test]
    fn extracts_utf8_json() {
        let mut file = tempfile::Builder::new().suffix(".json").tempfile().unwrap();
        write!(file, "{{\"client\":\"Acme\"}}").unwrap();
        assert_eq!(extract(file.path()).unwrap(), "{\"client\":\"Acme\"}");
    }

    #[test]
    fn strips_docx_xml_tags_and_decodes_entities() {
        let xml = r#"<w:p><w:r><w:t>Acme &amp; Fils</w:t></w:r></w:p>"#;
        assert_eq!(docx_xml_to_text(xml).trim(), "Acme & Fils");
    }
}
