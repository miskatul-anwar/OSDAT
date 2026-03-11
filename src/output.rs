use std::fs;
use std::path::Path;

use crate::models::QualityReport;

/// Write the quality report as formatted JSON to a file.
pub fn write_report(report: &QualityReport, output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(report)?;
    fs::write(output_path, &json)?;
    println!("\nReport written to: {}", output_path.display());
    Ok(())
}

/// Serialize the quality report to a JSON string.
pub fn report_to_json(report: &QualityReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn sample_report() -> QualityReport {
        let mut languages = HashMap::new();
        languages.insert("bangla".to_string(), 1);
        languages.insert("english".to_string(), 1);

        QualityReport {
            website: Website {
                url: "https://sparrso.gov.bd/".to_string(),
            },
            platform_level: PlatformLevel {
                user_accessibility: UserAccessibility {
                    necessity_of_login: 0,
                    multiple_language_support: 1,
                    request_for_datasets: 0,
                    languages,
                },
                user_usability: UserUsability {
                    browse_datasets_by_category: 1,
                    filter_sort_datasets: 0,
                    search_for_dataset: 1,
                    user_guideline: 0,
                },
                diversity: Diversity {
                    number_of_dataset: 3,
                    number_of_category: 2,
                },
            },
            categories: vec![Category {
                name: "গবেষণা".to_string(),
                datasets: vec![Dataset {
                    title: "test_data".to_string(),
                    source_url: "https://sparrso.gov.bd/research".to_string(),
                    download_url: "https://sparrso.gov.bd/files/test.csv".to_string(),
                    file_type: "CSV".to_string(),
                    file_size_bytes: Some(1024),
                    metadata: DatasetMetadata {
                        description: "Test dataset".to_string(),
                        rows: Some(100),
                        columns: Some(5),
                        column_names: vec![
                            "id".to_string(),
                            "name".to_string(),
                            "value".to_string(),
                        ],
                        format_quality: FormatQuality {
                            machine_readable: 1,
                            open_format: 1,
                        },
                    },
                }],
            }],
        }
    }

    #[test]
    fn test_report_to_json() {
        let report = sample_report();
        let json = report_to_json(&report).unwrap();

        assert!(json.contains("\"url\": \"https://sparrso.gov.bd/\""));
        assert!(json.contains("\"necessity-of-login\": 0"));
        assert!(json.contains("\"multiple-language-support\": 1"));
        assert!(json.contains("\"browse-data-sets-by-category\": 1"));
        assert!(json.contains("\"number-of-dataset\": 3"));
        assert!(json.contains("\"machine-readable\": 1"));
        assert!(json.contains("\"open-format\": 1"));
        assert!(json.contains("গবেষণা"));
    }

    #[test]
    fn test_write_report_to_file() {
        let dir = TempDir::new().unwrap();
        let output = dir.path().join("test_output.json");
        let report = sample_report();

        write_report(&report, &output).unwrap();

        let content = fs::read_to_string(&output).unwrap();
        assert!(content.contains("sparrso.gov.bd"));

        // Verify it's valid JSON by parsing it back
        let parsed: QualityReport = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.website.url, "https://sparrso.gov.bd/");
        assert_eq!(parsed.platform_level.diversity.number_of_dataset, 3);
    }

    #[test]
    fn test_json_field_names_match_schema() {
        let report = sample_report();
        let json = report_to_json(&report).unwrap();

        // Verify the JSON uses the exact field names from the schema
        assert!(json.contains("\"platform-level\""));
        assert!(json.contains("\"user-accessibility\""));
        assert!(json.contains("\"user-usability\""));
        assert!(json.contains("\"necessity-of-login\""));
        assert!(json.contains("\"multiple-language-support\""));
        assert!(json.contains("\"request-for-datasets\""));
        assert!(json.contains("\"browse-data-sets-by-category\""));
        assert!(json.contains("\"filter-and/or-sort-datasets\""));
        assert!(json.contains("\"search-for-dataset\""));
        assert!(json.contains("\"user-guideline\""));
        assert!(json.contains("\"number-of-dataset\""));
        assert!(json.contains("\"number-of-category\""));
        assert!(json.contains("\"machine-readable\""));
        assert!(json.contains("\"open-format\""));
    }
}
