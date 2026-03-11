use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ──────────────────────────────────────────────
// Top-level output structure matching sparrso.json
// ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityReport {
    pub website: WebsiteReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteReport {
    pub url: String,
    #[serde(rename = "portal-quality-assesment")]
    pub portal_quality_assessment: PortalQualityAssessment,
    pub category: HashMap<String, IndexMap<String, DatasetEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalQualityAssessment {
    #[serde(rename = "platform-level")]
    pub platform_level: PlatformLevel,
}

// ──────────────────────────────────────────────
// Platform-level (unchanged field names)
// ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformLevel {
    #[serde(rename = "user-accessibility")]
    pub user_accessibility: UserAccessibility,
    #[serde(rename = "user-usability")]
    pub user_usability: UserUsability,
    pub diversity: Diversity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccessibility {
    #[serde(rename = "necessity-of-login")]
    pub necessity_of_login: u8,
    #[serde(rename = "multiple-language-support")]
    pub multiple_language_support: u8,
    #[serde(rename = "request-for-datasets")]
    pub request_for_datasets: u8,
    pub languages: HashMap<String, u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserUsability {
    #[serde(rename = "browse-data-sets-by-category")]
    pub browse_datasets_by_category: u8,
    #[serde(rename = "filter-and/or-sort-datasets")]
    pub filter_sort_datasets: u8,
    #[serde(rename = "search-for-dataset")]
    pub search_for_dataset: u8,
    #[serde(rename = "user-guideline")]
    pub user_guideline: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diversity {
    #[serde(rename = "number-of-dataset")]
    pub number_of_dataset: u32,
    #[serde(rename = "number-of-category")]
    pub number_of_category: u32,
}

// ──────────────────────────────────────────────
// Per-dataset entry (keyed as "dataset1", "dataset2", …)
// ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetEntry {
    #[serde(rename = "dataset-name")]
    pub dataset_name: String,
    pub url: String,
    #[serde(rename = "dataset-level")]
    pub dataset_level: DatasetLevel,
    #[serde(rename = "data-level")]
    pub data_level: DataLevel,
}

// ──────────────────────────────────────────────
// dataset-level
// ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetLevel {
    pub openness: Openness,
    pub transparency: Transparency,
    pub provenance: Provenance,
    #[serde(rename = "semantic-consistency")]
    pub semantic_consistency: SemanticConsistency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Openness {
    pub complete: OpennessComplete,
    pub primary: u8,
    #[serde(rename = "non-discriminatory")]
    pub non_discriminatory: u8,
    pub accessible: u8,
    pub timely: u8,
    #[serde(rename = "non-proprietary")]
    pub non_proprietary: u8,
    #[serde(rename = "license-free")]
    pub license_free: u8,
    #[serde(rename = "machine-readable")]
    pub machine_readable: MachineReadableFormats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpennessComplete {
    pub descriptive: u8,
    pub downloadable: u8,
    #[serde(rename = "machine-readable")]
    pub machine_readable: u8,
    #[serde(rename = "linked-data")]
    pub linked_data: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineReadableFormats {
    pub pdf: u8,
    pub csv: u8,
    pub rdf: u8,
    pub xml: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transparency {
    pub source: String,
    #[serde(rename = "number-of-downloads")]
    pub number_of_downloads: u64,
    pub understandability: Understandability,
    #[serde(rename = "meta-data")]
    pub meta_data: u8,
    #[serde(rename = "5*")]
    pub five_star: FiveStar,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Understandability {
    #[serde(rename = "FAQ")]
    pub faq: u8,
    #[serde(rename = "textual-description")]
    pub textual_description: u8,
    #[serde(rename = "category-tag")]
    pub category_tag: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiveStar {
    #[serde(rename = "available-online")]
    pub available_online: u8,
    #[serde(rename = "machine-readable")]
    pub machine_readable: u8,
    #[serde(rename = "non-proprietary-format")]
    pub non_proprietary_format: u8,
    #[serde(rename = "open-standard")]
    pub open_standard: u8,
    #[serde(rename = "linked-data")]
    pub linked_data: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub source: String,
    #[serde(rename = "time-period")]
    pub time_period: String,
    #[serde(rename = "update-activity")]
    pub update_activity: String,
    #[serde(rename = "last-update")]
    pub last_update: String,
    #[serde(rename = "collection-method")]
    pub collection_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticConsistency {
    #[serde(rename = "external-vocabulary")]
    pub external_vocabulary: u8,
}

// ──────────────────────────────────────────────
// data-level
// ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLevel {
    pub granularity: Granularity,
    #[serde(rename = "data-level-completeness")]
    pub data_level_completeness: DataLevelCompleteness,
    #[serde(rename = "data-volume")]
    pub data_volume: DataVolume,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Granularity {
    #[serde(rename = "time-dimension")]
    pub time_dimension: TimeDimension,
    #[serde(rename = "geo-dimension")]
    pub geo_dimension: GeoDimension,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeDimension {
    pub day: u8,
    pub month: u8,
    pub year: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoDimension {
    pub union: u8,
    pub upazila: u8,
    pub zila: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLevelCompleteness {
    #[serde(rename = "number-of-empty-cells")]
    pub number_of_empty_cells: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataVolume {
    #[serde(rename = "number-of-rows")]
    pub number_of_rows: u64,
    #[serde(rename = "number-of-columns")]
    pub number_of_columns: u64,
    #[serde(rename = "file-size")]
    pub file_size: String,
    #[serde(rename = "download-link")]
    pub download_link: String,
}

// ──────────────────────────────────────────────
// Internal / working structures (not in output JSON)
// ──────────────────────────────────────────────

/// Internal structure tracking discovered file links from crawling.
#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    pub source_page_url: String,
    pub download_url: String,
    pub file_extension: String,
}

/// Configuration collected from CLI input.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub root_url: String,
    pub page_urls: Vec<String>,
    pub output_filename: String,
    pub category_name: String,
}

/// Represents analysis results from the LLM for platform-level.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmAnalysis {
    pub necessity_of_login: Option<u8>,
    pub multiple_language_support: Option<u8>,
    pub request_for_datasets: Option<u8>,
    pub languages: Option<HashMap<String, u8>>,
    pub browse_datasets_by_category: Option<u8>,
    pub filter_sort_datasets: Option<u8>,
    pub search_for_dataset: Option<u8>,
    pub user_guideline: Option<u8>,
    pub number_of_category: Option<u32>,
}

/// Represents RAG analysis results from the LLM for per-dataset metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatasetRagAnalysis {
    pub dataset_name: Option<String>,
    pub time_period: Option<String>,
    pub update_activity: Option<String>,
    pub last_update: Option<String>,
    pub collection_method: Option<String>,
    pub granularity_day: Option<u8>,
    pub granularity_month: Option<u8>,
    pub granularity_year: Option<u8>,
    pub granularity_union: Option<u8>,
    pub granularity_upazila: Option<u8>,
    pub granularity_zila: Option<u8>,
}

/// Extracted file metrics from a downloaded file.
#[derive(Debug, Clone)]
pub struct ExtractedFileData {
    pub title: String,
    pub source_url: String,
    pub download_url: String,
    pub file_type: String,
    pub file_size_bytes: u64,
    pub rows: Option<u64>,
    pub columns: Option<u64>,
    pub empty_cells: u64,
    pub column_names: Vec<String>,
    pub machine_readable: u8,
    pub open_format: u8,
}

// ──────────────────────────────────────────────
// Utility functions
// ──────────────────────────────────────────────

/// Format a byte count into a human-readable string (e.g., "1.3M", "928kb").
pub fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{}kb", bytes / KB)
    } else {
        format!("{}B", bytes)
    }
}

/// Determine per-file-type machine-readable format flags based on extension.
pub fn machine_readable_formats(extension: &str) -> MachineReadableFormats {
    MachineReadableFormats {
        pdf: if extension == ".pdf" { 1 } else { 0 },
        csv: if extension == ".csv" { 1 } else { 0 },
        rdf: if extension == ".rdf" { 1 } else { 0 },
        xml: if extension == ".xml" { 1 } else { 0 },
    }
}

/// Determine if a file format is non-proprietary.
pub fn is_non_proprietary(extension: &str) -> u8 {
    match extension {
        ".pdf" | ".csv" | ".xml" | ".rdf" | ".txt" => 1,
        _ => 0,
    }
}

/// Build default DatasetLevel auto-detecting fields from the extracted file data.
pub fn auto_detect_dataset_level(data: &ExtractedFileData) -> DatasetLevel {
    let ext = format!(".{}", data.file_type.to_lowercase());
    let mr = data.machine_readable;

    DatasetLevel {
        openness: Openness {
            complete: OpennessComplete {
                descriptive: 0,
                downloadable: 1, // download succeeded
                machine_readable: mr,
                linked_data: 0,
            },
            primary: 0,
            non_discriminatory: 1, // publicly available
            accessible: 1,        // publicly reachable
            timely: 0,
            non_proprietary: is_non_proprietary(&ext),
            license_free: 0,
            machine_readable: machine_readable_formats(&ext),
        },
        transparency: Transparency {
            source: String::new(),
            number_of_downloads: 0,
            understandability: Understandability {
                faq: 0,
                textual_description: 0,
                category_tag: 0,
            },
            meta_data: 0,
            five_star: FiveStar {
                available_online: 1, // it's on the web
                machine_readable: mr,
                non_proprietary_format: is_non_proprietary(&ext),
                open_standard: 0,
                linked_data: 0,
            },
        },
        provenance: Provenance {
            source: String::new(),
            time_period: String::new(),
            update_activity: String::new(),
            last_update: String::new(),
            collection_method: String::new(),
        },
        semantic_consistency: SemanticConsistency {
            external_vocabulary: 0,
        },
    }
}

/// Build default DataLevel from extracted file data.
pub fn auto_detect_data_level(data: &ExtractedFileData) -> DataLevel {
    DataLevel {
        granularity: Granularity {
            time_dimension: TimeDimension {
                day: 0,
                month: 0,
                year: 0,
            },
            geo_dimension: GeoDimension {
                union: 0,
                upazila: 0,
                zila: 0,
            },
        },
        data_level_completeness: DataLevelCompleteness {
            number_of_empty_cells: data.empty_cells,
        },
        data_volume: DataVolume {
            number_of_rows: data.rows.unwrap_or(0),
            number_of_columns: data.columns.unwrap_or(0),
            file_size: format_file_size(data.file_size_bytes),
            download_link: data.download_url.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(500), "500B");
        assert_eq!(format_file_size(1024), "1kb");
        assert_eq!(format_file_size(1536), "1kb");
        assert_eq!(format_file_size(1048576), "1.0M");
        assert_eq!(format_file_size(1365000), "1.3M");
        assert_eq!(format_file_size(1073741824), "1.0G");
    }

    #[test]
    fn test_machine_readable_formats() {
        let pdf = machine_readable_formats(".pdf");
        assert_eq!(pdf.pdf, 1);
        assert_eq!(pdf.csv, 0);

        let csv = machine_readable_formats(".csv");
        assert_eq!(csv.csv, 1);
        assert_eq!(csv.pdf, 0);
    }

    #[test]
    fn test_is_non_proprietary() {
        assert_eq!(is_non_proprietary(".csv"), 1);
        assert_eq!(is_non_proprietary(".pdf"), 1);
        assert_eq!(is_non_proprietary(".xlsx"), 0);
    }

    #[test]
    fn test_auto_detect_dataset_level() {
        let data = ExtractedFileData {
            title: "test".into(),
            source_url: "http://example.com".into(),
            download_url: "http://example.com/f.csv".into(),
            file_type: "CSV".into(),
            file_size_bytes: 1024,
            rows: Some(10),
            columns: Some(3),
            empty_cells: 2,
            column_names: vec![],
            machine_readable: 1,
            open_format: 1,
        };
        let dl = auto_detect_dataset_level(&data);
        assert_eq!(dl.openness.complete.downloadable, 1);
        assert_eq!(dl.openness.complete.machine_readable, 1);
        assert_eq!(dl.openness.non_proprietary, 1);
        assert_eq!(dl.openness.machine_readable.csv, 1);
        assert_eq!(dl.openness.machine_readable.pdf, 0);
    }

    #[test]
    fn test_auto_detect_data_level() {
        let data = ExtractedFileData {
            title: "test".into(),
            source_url: "http://example.com".into(),
            download_url: "http://example.com/f.csv".into(),
            file_type: "CSV".into(),
            file_size_bytes: 1365000,
            rows: Some(100),
            columns: Some(5),
            empty_cells: 7,
            column_names: vec![],
            machine_readable: 1,
            open_format: 1,
        };
        let dl = auto_detect_data_level(&data);
        assert_eq!(dl.data_level_completeness.number_of_empty_cells, 7);
        assert_eq!(dl.data_volume.number_of_rows, 100);
        assert_eq!(dl.data_volume.number_of_columns, 5);
        assert_eq!(dl.data_volume.file_size, "1.3M");
        assert_eq!(dl.data_volume.download_link, "http://example.com/f.csv");
    }
}
