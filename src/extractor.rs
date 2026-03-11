use std::path::Path;

use crate::models::{Dataset, DatasetMetadata, FormatQuality};

/// Extract metadata from a downloaded file based on its extension.
pub fn extract_metadata(
    file_path: &Path,
    download_url: &str,
    source_url: &str,
    extension: &str,
    file_size: u64,
) -> Dataset {
    let title = file_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let (rows, columns, column_names) = match extension {
        ".csv" => extract_csv_metadata(file_path),
        ".xlsx" | ".xls" => extract_excel_metadata(file_path),
        ".xml" => extract_xml_metadata(file_path),
        ".txt" => extract_txt_metadata(file_path),
        _ => (None, None, Vec::new()),
    };

    let (machine_readable, open_format) = classify_format(extension);

    // Description is a placeholder; it will be enhanced by the LLM in Stage 5
    Dataset {
        title,
        source_url: source_url.to_string(),
        download_url: download_url.to_string(),
        file_type: extension.trim_start_matches('.').to_uppercase(),
        file_size_bytes: Some(file_size),
        metadata: DatasetMetadata {
            description: format!(
                "Data file ({}) extracted from {}",
                extension.trim_start_matches('.').to_uppercase(),
                source_url
            ),
            rows,
            columns,
            column_names,
            format_quality: FormatQuality {
                machine_readable,
                open_format,
            },
        },
    }
}

/// Classify format quality based on file extension.
fn classify_format(extension: &str) -> (u8, u8) {
    match extension {
        ".csv" => (1, 1),       // Machine readable + open format
        ".xml" => (1, 1),       // Machine readable + open format
        ".txt" => (1, 1),       // Machine readable + open format
        ".xlsx" | ".xls" => (1, 0), // Machine readable but proprietary
        ".docx" | ".doc" => (0, 0), // Not machine readable tabular data
        ".pdf" => (0, 0),       // Not machine readable
        ".pptx" | ".ppt" => (0, 0), // Not machine readable tabular data
        _ => (0, 0),
    }
}

/// Extract metadata from a CSV file.
fn extract_csv_metadata(file_path: &Path) -> (Option<u64>, Option<u64>, Vec<String>) {
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
            for result in reader.records() {
                if result.is_ok() {
                    row_count += 1;
                }
            }

            let num_rows = if row_count > 0 {
                Some(row_count)
            } else {
                None
            };

            (num_rows, num_columns, headers)
        }
        Err(e) => {
            eprintln!("    Warning: Could not parse CSV {}: {e}", file_path.display());
            (None, None, Vec::new())
        }
    }
}

/// Extract metadata from an Excel file (.xlsx or .xls).
fn extract_excel_metadata(file_path: &Path) -> (Option<u64>, Option<u64>, Vec<String>) {
    use calamine::{Reader, open_workbook_auto};

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
                            .map(|row| {
                                row.iter()
                                    .map(|cell| format!("{cell}"))
                                    .collect()
                            })
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    };

                    let data_rows = if total_rows > 1 {
                        Some((total_rows - 1) as u64) // Exclude header row
                    } else if total_rows == 1 {
                        Some(0)
                    } else {
                        None
                    };

                    return (
                        data_rows,
                        Some(total_cols as u64),
                        headers,
                    );
                }
            }
            (None, None, Vec::new())
        }
        Err(e) => {
            eprintln!(
                "    Warning: Could not parse Excel file {}: {e}",
                file_path.display()
            );
            (None, None, Vec::new())
        }
    }
}

/// Maximum number of XML elements to process before stopping.
const MAX_XML_ELEMENTS: u64 = 100_000;

/// Extract basic metadata from an XML file.
fn extract_xml_metadata(file_path: &Path) -> (Option<u64>, Option<u64>, Vec<String>) {
    use std::collections::HashSet;
    use std::fs::File;
    use std::io::BufReader;
    use xml::reader::{EventReader, XmlEvent};

    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("    Warning: Could not open XML {}: {e}", file_path.display());
            return (None, None, Vec::new());
        }
    };

    let reader = BufReader::new(file);
    let parser = EventReader::new(reader);

    let mut element_names: HashSet<String> = HashSet::new();
    let mut element_count: u64 = 0;
    let mut depth: u32 = 0;
    let mut record_depth: Option<u32> = None;
    let mut record_count: u64 = 0;

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
            }
            Ok(XmlEvent::EndElement { .. }) => {
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

    (rows, cols, column_names)
}

/// Extract basic metadata from a text file.
fn extract_txt_metadata(file_path: &Path) -> (Option<u64>, Option<u64>, Vec<String>) {
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
                    return (Some(num_lines), None, Vec::new());
                };

                let headers: Vec<String> = first_line
                    .split(separator)
                    .map(|s| s.trim().to_string())
                    .collect();

                let data_rows = if num_lines > 1 {
                    Some(num_lines - 1)
                } else {
                    Some(0)
                };

                return (data_rows, Some(headers.len() as u64), headers);
            }

            (Some(num_lines), None, Vec::new())
        }
        Err(e) => {
            eprintln!(
                "    Warning: Could not read text file {}: {e}",
                file_path.display()
            );
            (None, None, Vec::new())
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
        assert_eq!(classify_format(".xlsx"), (1, 0));
        assert_eq!(classify_format(".pdf"), (0, 0));
        assert_eq!(classify_format(".docx"), (0, 0));
    }

    #[test]
    fn test_extract_csv_metadata() {
        let dir = TempDir::new().unwrap();
        let csv_path = dir.path().join("test.csv");
        fs::write(&csv_path, "name,age,city\nAlice,30,NYC\nBob,25,LA\n").unwrap();

        let (rows, cols, headers) = extract_csv_metadata(&csv_path);
        assert_eq!(rows, Some(2));
        assert_eq!(cols, Some(3));
        assert_eq!(headers, vec!["name", "age", "city"]);
    }

    #[test]
    fn test_extract_txt_metadata_tsv() {
        let dir = TempDir::new().unwrap();
        let txt_path = dir.path().join("test.txt");
        fs::write(&txt_path, "col1\tcol2\tcol3\nval1\tval2\tval3\n").unwrap();

        let (rows, cols, headers) = extract_txt_metadata(&txt_path);
        assert_eq!(rows, Some(1));
        assert_eq!(cols, Some(3));
        assert_eq!(headers, vec!["col1", "col2", "col3"]);
    }

    #[test]
    fn test_extract_metadata_creates_dataset() {
        let dir = TempDir::new().unwrap();
        let csv_path = dir.path().join("test_data.csv");
        fs::write(&csv_path, "x,y\n1,2\n3,4\n").unwrap();

        let dataset = extract_metadata(
            &csv_path,
            "https://example.com/test_data.csv",
            "https://example.com/page",
            ".csv",
            50,
        );

        assert_eq!(dataset.title, "test_data");
        assert_eq!(dataset.file_type, "CSV");
        assert_eq!(dataset.file_size_bytes, Some(50));
        assert_eq!(dataset.metadata.rows, Some(2));
        assert_eq!(dataset.metadata.columns, Some(2));
        assert_eq!(dataset.metadata.format_quality.machine_readable, 1);
        assert_eq!(dataset.metadata.format_quality.open_format, 1);
    }
}
