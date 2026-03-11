use std::fs;
use std::path::Path;

use crate::models::QualityReport;

/// Write the quality report as formatted JSON to a file.
pub fn write_report(
    report: &QualityReport,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
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
    use indexmap::IndexMap;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn sample_report() -> QualityReport {
        let mut languages = HashMap::new();
        languages.insert("bangla".to_string(), 1);
        languages.insert("english".to_string(), 1);

        let mut datasets = IndexMap::new();
        datasets.insert(
            "dataset1".to_string(),
            DatasetEntry {
                dataset_name: "২০১৯-২০২০ অর্থ বছরের গবেষণা কর্মের তালিকা".to_string(),
                url: "https://sparrso.gov.bd/research".to_string(),
                dataset_level: DatasetLevel {
                    openness: Openness {
                        complete: OpennessComplete {
                            descriptive: 0,
                            downloadable: 1,
                            machine_readable: 1,
                            linked_data: 0,
                        },
                        primary: 0,
                        non_discriminatory: 1,
                        accessible: 1,
                        timely: 0,
                        non_proprietary: 1,
                        license_free: 0,
                        machine_readable: MachineReadableFormats {
                            pdf: 0,
                            csv: 1,
                            rdf: 0,
                            xml: 0,
                        },
                    },
                    transparency: Transparency {
                        source: "SPARRSO".to_string(),
                        number_of_downloads: 0,
                        understandability: Understandability {
                            faq: 0,
                            textual_description: 0,
                            category_tag: 0,
                        },
                        meta_data: 0,
                        five_star: FiveStar {
                            available_online: 1,
                            machine_readable: 1,
                            non_proprietary_format: 1,
                            open_standard: 0,
                            linked_data: 0,
                        },
                    },
                    provenance: Provenance {
                        source: "SPARRSO".to_string(),
                        time_period: "2019-2020".to_string(),
                        update_activity: "yearly".to_string(),
                        last_update: "2020-06-30".to_string(),
                        collection_method: "research".to_string(),
                    },
                    semantic_consistency: SemanticConsistency {
                        external_vocabulary: 0,
                    },
                },
                data_level: DataLevel {
                    granularity: Granularity {
                        time_dimension: TimeDimension {
                            day: 0,
                            month: 0,
                            year: 1,
                        },
                        geo_dimension: GeoDimension {
                            union: 0,
                            upazila: 0,
                            zila: 0,
                        },
                    },
                    data_level_completeness: DataLevelCompleteness {
                        number_of_empty_cells: 5,
                    },
                    data_volume: DataVolume {
                        number_of_rows: 100,
                        number_of_columns: 5,
                        file_size: "1kb".to_string(),
                        download_link: "https://sparrso.gov.bd/files/test.csv".to_string(),
                    },
                },
            },
        );

        let mut category = HashMap::new();
        category.insert("গবেষণা".to_string(), datasets);

        QualityReport {
            website: WebsiteReport {
                url: "https://sparrso.gov.bd/".to_string(),
                portal_quality_assessment: PortalQualityAssessment {
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
                },
                category,
            },
        }
    }

    #[test]
    fn test_report_to_json() {
        let report = sample_report();
        let json = report_to_json(&report).unwrap();

        assert!(json.contains("\"url\": \"https://sparrso.gov.bd/\""));
        assert!(json.contains("\"portal-quality-assesment\""));
        assert!(json.contains("\"necessity-of-login\": 0"));
        assert!(json.contains("\"multiple-language-support\": 1"));
        assert!(json.contains("\"browse-data-sets-by-category\": 1"));
        assert!(json.contains("\"number-of-dataset\": 3"));
        assert!(json.contains("\"dataset-name\""));
        assert!(json.contains("\"dataset-level\""));
        assert!(json.contains("\"data-level\""));
        assert!(json.contains("\"number-of-empty-cells\": 5"));
        assert!(json.contains("\"file-size\": \"1kb\""));
        assert!(json.contains("\"download-link\""));
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
        assert_eq!(
            parsed
                .website
                .portal_quality_assessment
                .platform_level
                .diversity
                .number_of_dataset,
            3
        );
    }

    #[test]
    fn test_json_field_names_match_schema() {
        let report = sample_report();
        let json = report_to_json(&report).unwrap();

        // Verify the JSON uses the exact field names from the sparrso.json schema
        assert!(json.contains("\"portal-quality-assesment\""));
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

        // New dataset-level fields
        assert!(json.contains("\"dataset-name\""));
        assert!(json.contains("\"dataset-level\""));
        assert!(json.contains("\"data-level\""));
        assert!(json.contains("\"machine-readable\""));
        assert!(json.contains("\"linked-data\""));
        assert!(json.contains("\"non-discriminatory\""));
        assert!(json.contains("\"non-proprietary\""));
        assert!(json.contains("\"license-free\""));
        assert!(json.contains("\"number-of-downloads\""));
        assert!(json.contains("\"textual-description\""));
        assert!(json.contains("\"category-tag\""));
        assert!(json.contains("\"meta-data\""));
        assert!(json.contains("\"5*\""));
        assert!(json.contains("\"available-online\""));
        assert!(json.contains("\"non-proprietary-format\""));
        assert!(json.contains("\"open-standard\""));
        assert!(json.contains("\"time-period\""));
        assert!(json.contains("\"update-activity\""));
        assert!(json.contains("\"last-update\""));
        assert!(json.contains("\"collection-method\""));
        assert!(json.contains("\"external-vocabulary\""));
        assert!(json.contains("\"time-dimension\""));
        assert!(json.contains("\"geo-dimension\""));
        assert!(json.contains("\"data-level-completeness\""));
        assert!(json.contains("\"number-of-empty-cells\""));
        assert!(json.contains("\"data-volume\""));
        assert!(json.contains("\"number-of-rows\""));
        assert!(json.contains("\"number-of-columns\""));
        assert!(json.contains("\"file-size\""));
        assert!(json.contains("\"download-link\""));
    }
}
