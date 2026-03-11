use std::path::Path;

use crate::models::ExtractedFileData;

/// Extract metadata from a downloaded file based on its extension.
/// Returns a Vec because a single file (especially PDF) may contain multiple datasets.
pub fn extract_metadata(
    file_path: &Path,
    download_url: &str,
    source_url: &str,
    extension: &str,
    file_size: u64,
) -> Vec<ExtractedFileData> {
    let title = file_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let (rows, columns, column_names, empty_cells) = match extension {
        ".csv" => extract_csv_metadata(file_path),
        ".xlsx" | ".xls" => extract_excel_metadata(file_path),
        ".xml" => extract_xml_metadata(file_path),
        ".txt" => extract_txt_metadata(file_path),
        ".pdf" => extract_pdf_metadata(file_path),
        _ => (None, None, Vec::new(), 0),
    };

    let (machine_readable, open_format) = classify_format(extension);

    vec![ExtractedFileData {
        title,
        source_url: source_url.to_string(),
        download_url: download_url.to_string(),
        file_type: extension.trim_start_matches('.').to_uppercase(),
        file_size_bytes: file_size,
        rows,
        columns,
        empty_cells,
        column_names,
        machine_readable,
        open_format,
    }]
}

/// Classify format quality based on file extension.
fn classify_format(extension: &str) -> (u8, u8) {
    match extension {
        ".csv" => (1, 1),           // Machine readable + open format
        ".xml" => (1, 1),           // Machine readable + open format
        ".rdf" => (1, 1),           // Machine readable + open format
        ".txt" => (1, 1),           // Machine readable + open format
        ".xlsx" | ".xls" => (1, 0), // Machine readable but proprietary
        ".docx" | ".doc" => (0, 0), // Not machine readable tabular data
        ".pdf" => (0, 0),           // Not machine readable
        ".pptx" | ".ppt" => (0, 0), // Not machine readable tabular data
        _ => (0, 0),
    }
}

/// Extract metadata from a CSV file, including empty cell count.
fn extract_csv_metadata(file_path: &Path) -> (Option<u64>, Option<u64>, Vec<String>, u64) {
    match csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(file_path)
    {
        Ok(mut reader) => {
            let headers: Vec<String> = reader
                .headers()
                .map(|h| h.iter().map(|s| s.to_string()).collect())
                .unwrap_or_default();

            let num_columns = if headers.is_empty() {
                None
            } else {
                Some(headers.len() as u64)
            };

            let mut row_count: u64 = 0;
            let mut empty_cells: u64 = 0;
            for result in reader.records() {
                if let Ok(record) = result {
                    row_count += 1;
                    for field in record.iter() {
                        if field.trim().is_empty() {
                            empty_cells += 1;
                        }
                    }
                }
            }

            let num_rows = if row_count > 0 {
                Some(row_count)
            } else {
                None
            };

            (num_rows, num_columns, headers, empty_cells)
        }
        Err(e) => {
            eprintln!(
                "    Warning: Could not parse CSV {}: {e}",
                file_path.display()
            );
            (None, None, Vec::new(), 0)
        }
    }
}

/// Extract metadata from an Excel file (.xlsx or .xls), including empty cell count.
fn extract_excel_metadata(file_path: &Path) -> (Option<u64>, Option<u64>, Vec<String>, u64) {
    use calamine::{open_workbook_auto, DataType, Reader};

    match open_workbook_auto(file_path) {
        Ok(mut workbook) => {
            if let Some(sheet_name) = workbook.sheet_names().first().cloned() {
                if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                    let total_rows = range.height();
                    let total_cols = range.width();

                    // Try to get headers from first row
                    let headers: Vec<String> = if total_rows > 0 {
                        range
                            .rows()
                            .next()
                            .map(|row| row.iter().map(|cell| format!("{cell}")).collect())
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    };

                    // Count empty cells (excluding header row)
                    let mut empty_cells: u64 = 0;
                    for row in range.rows().skip(1) {
                        for cell in row {
                            if cell.is_empty() {
                                empty_cells += 1;
                            }
                        }
                    }

                    let data_rows = if total_rows > 1 {
                        Some((total_rows - 1) as u64) // Exclude header row
                    } else if total_rows == 1 {
                        Some(0)
                    } else {
                        None
                    };

                    return (data_rows, Some(total_cols as u64), headers, empty_cells);
                }
            }
            (None, None, Vec::new(), 0)
        }
        Err(e) => {
            eprintln!(
                "    Warning: Could not parse Excel file {}: {e}",
                file_path.display()
            );
            (None, None, Vec::new(), 0)
        }
    }
}

/// Maximum number of XML elements to process before stopping.
const MAX_XML_ELEMENTS: u64 = 100_000;

/// Extract basic metadata from an XML file, including empty element count.
fn extract_xml_metadata(file_path: &Path) -> (Option<u64>, Option<u64>, Vec<String>, u64) {
    use std::collections::HashSet;
    use std::fs::File;
    use std::io::BufReader;
    use xml::reader::{EventReader, XmlEvent};

    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "    Warning: Could not open XML {}: {e}",
                file_path.display()
            );
            return (None, None, Vec::new(), 0);
        }
    };

    let reader = BufReader::new(file);
    let parser = EventReader::new(reader);

    let mut element_names: HashSet<String> = HashSet::new();
    let mut element_count: u64 = 0;
    let mut depth: u32 = 0;
    let mut record_depth: Option<u32> = None;
    let mut record_count: u64 = 0;
    let mut empty_cells: u64 = 0;
    let mut current_text = String::new();
    let mut in_leaf_element = false;

    for event in parser {
        element_count += 1;
        if element_count > MAX_XML_ELEMENTS {
            eprintln!(
                "    Warning: XML file exceeds element limit ({}), stopping",
                MAX_XML_ELEMENTS
            );
            break;
        }

        match event {
            Ok(XmlEvent::StartElement { name, .. }) => {
                depth += 1;
                element_names.insert(name.local_name.clone());

                // Heuristic: elements at depth 2 are likely records
                if depth == 2 && record_depth.is_none() {
                    record_depth = Some(depth);
                }
                if Some(depth) == record_depth {
                    record_count += 1;
                }
                current_text.clear();
                in_leaf_element = true;
            }
            Ok(XmlEvent::Characters(text)) => {
                current_text.push_str(&text);
            }
            Ok(XmlEvent::EndElement { .. }) => {
                if in_leaf_element && current_text.trim().is_empty() {
                    empty_cells += 1;
                }
                in_leaf_element = false;
                current_text.clear();
                depth = depth.saturating_sub(1);
            }
            Err(_) => break,
            _ => {}
        }
    }

    let column_names: Vec<String> = element_names.into_iter().collect();
    let rows = if record_count > 0 {
        Some(record_count)
    } else {
        None
    };
    let cols = if !column_names.is_empty() {
        Some(column_names.len() as u64)
    } else {
        None
    };

    (rows, cols, column_names, empty_cells)
}

/// Extract basic metadata from a text file.
fn extract_txt_metadata(file_path: &Path) -> (Option<u64>, Option<u64>, Vec<String>, u64) {
    use std::fs;

    match fs::read_to_string(file_path) {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().collect();
            let num_lines = lines.len() as u64;

            // Try to detect tab-separated or comma-separated data
            if let Some(first_line) = lines.first() {
                let separator = if first_line.contains('\t') {
                    '\t'
                } else if first_line.contains(',') {
                    ','
                } else {
                    return (Some(num_lines), None, Vec::new(), 0);
                };

                let headers: Vec<String> = first_line
                    .split(separator)
                    .map(|s| s.trim().to_string())
                    .collect();

                let mut empty_cells: u64 = 0;
                for line in lines.iter().skip(1) {
                    for field in line.split(separator) {
                        if field.trim().is_empty() {
                            empty_cells += 1;
                        }
                    }
                }

                let data_rows = if num_lines > 1 {
                    Some(num_lines - 1)
                } else {
                    Some(0)
                };

                return (data_rows, Some(headers.len() as u64), headers, empty_cells);
            }

            (Some(num_lines), None, Vec::new(), 0)
        }
        Err(e) => {
            eprintln!(
                "    Warning: Could not read text file {}: {e}",
                file_path.display()
            );
            (None, None, Vec::new(), 0)
        }
    }
}

/// Extract metadata from a PDF file using Python camelot via subprocess.
/// Falls back to basic metadata if Python/camelot is not available.
fn extract_pdf_metadata(file_path: &Path) -> (Option<u64>, Option<u64>, Vec<String>, u64) {
    use std::process::Command;

    let script = r#"
import json, sys
try:
    import camelot
    path = sys.argv[1]
    tables = camelot.read_pdf(path, pages="all", flavor="stream")
    total_rows = 0
    total_cols = 0
    empty_cells = 0
    headers = []
    for t in tables:
        df = t.df
        total_rows += len(df)
        if df.shape[1] > total_cols:
            total_cols = df.shape[1]
            headers = [str(c) for c in df.iloc[0].tolist()] if len(df) > 0 else []
        for _, row in df.iterrows():
            for val in row:
                if str(val).strip() == "":
                    empty_cells += 1
    print(json.dumps({"rows": total_rows, "cols": total_cols, "empty": empty_cells, "headers": headers}))
except Exception as e:
    print(json.dumps({"error": str(e)}))
"#;

    match Command::new("python3")
        .args(["-c", script, &file_path.display().to_string()])
        .output()
    {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(stdout.trim()) {
                if val.get("error").is_none() {
                    let rows = val
                        .get("rows")
                        .and_then(|v| v.as_u64())
                        .filter(|&r| r > 0);
                    let cols = val
                        .get("cols")
                        .and_then(|v| v.as_u64())
                        .filter(|&c| c > 0);
                    let empty = val.get("empty").and_then(|v| v.as_u64()).unwrap_or(0);
                    let headers: Vec<String> = val
                        .get("headers")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                    return (rows, cols, headers, empty);
                }
            }
            eprintln!(
                "    Warning: camelot extraction failed for {}",
                file_path.display()
            );
            (None, None, Vec::new(), 0)
        }
        _ => {
            eprintln!("    Note: Python/camelot not available for PDF extraction");
            (None, None, Vec::new(), 0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_classify_format() {
        assert_eq!(classify_format(".csv"), (1, 1));
        assert_eq!(classify_format(".xml"), (1, 1));
        assert_eq!(classify_format(".rdf"), (1, 1));
        assert_eq!(classify_format(".xlsx"), (1, 0));
        assert_eq!(classify_format(".pdf"), (0, 0));
        assert_eq!(classify_format(".docx"), (0, 0));
    }

    #[test]
    fn test_extract_csv_metadata() {
        let dir = TempDir::new().unwrap();
        let csv_path = dir.path().join("test.csv");
        fs::write(&csv_path, "name,age,city\nAlice,30,NYC\nBob,25,LA\n").unwrap();

        let (rows, cols, headers, empty) = extract_csv_metadata(&csv_path);
        assert_eq!(rows, Some(2));
        assert_eq!(cols, Some(3));
        assert_eq!(headers, vec!["name", "age", "city"]);
        assert_eq!(empty, 0);
    }

    #[test]
    fn test_extract_csv_empty_cells() {
        let dir = TempDir::new().unwrap();
        let csv_path = dir.path().join("test_empty.csv");
        fs::write(&csv_path, "a,b,c\n1,,3\n,,\n4,5,\n").unwrap();

        let (rows, cols, _headers, empty) = extract_csv_metadata(&csv_path);
        assert_eq!(rows, Some(3));
        assert_eq!(cols, Some(3));
        assert_eq!(empty, 5); // 1 + 3 + 1 empty fields
    }

    #[test]
    fn test_extract_txt_metadata_tsv() {
        let dir = TempDir::new().unwrap();
        let txt_path = dir.path().join("test.txt");
        fs::write(&txt_path, "col1\tcol2\tcol3\nval1\tval2\tval3\n").unwrap();

        let (rows, cols, headers, empty) = extract_txt_metadata(&txt_path);
        assert_eq!(rows, Some(1));
        assert_eq!(cols, Some(3));
        assert_eq!(headers, vec!["col1", "col2", "col3"]);
        assert_eq!(empty, 0);
    }

    #[test]
    fn test_extract_txt_empty_cells() {
        let dir = TempDir::new().unwrap();
        let txt_path = dir.path().join("test_empty.txt");
        fs::write(&txt_path, "a\tb\tc\n1\t\t3\n\t\t\n").unwrap();

        let (_rows, _cols, _headers, empty) = extract_txt_metadata(&txt_path);
        assert_eq!(empty, 4); // 1 + 3 empty fields
    }

    #[test]
    fn test_extract_metadata_creates_dataset_vec() {
        let dir = TempDir::new().unwrap();
        let csv_path = dir.path().join("test_data.csv");
        fs::write(&csv_path, "x,y\n1,2\n3,4\n").unwrap();

        let datasets = extract_metadata(
            &csv_path,
            "https://example.com/test_data.csv",
            "https://example.com/page",
            ".csv",
            50,
        );

        assert_eq!(datasets.len(), 1);
        let dataset = &datasets[0];
        assert_eq!(dataset.title, "test_data");
        assert_eq!(dataset.file_type, "CSV");
        assert_eq!(dataset.file_size_bytes, 50);
        assert_eq!(dataset.rows, Some(2));
        assert_eq!(dataset.columns, Some(2));
        assert_eq!(dataset.machine_readable, 1);
        assert_eq!(dataset.open_format, 1);
        assert_eq!(dataset.empty_cells, 0);
    }
}
