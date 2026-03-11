use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level output structure conforming to the sparrso.json schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityReport {
    pub website: Website,
    #[serde(rename = "platform-level")]
    pub platform_level: PlatformLevel,
    pub categories: Vec<Category>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Website {
    pub url: String,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub name: String,
    pub datasets: Vec<Dataset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    pub title: String,
    pub source_url: String,
    pub download_url: String,
    pub file_type: String,
    pub file_size_bytes: Option<u64>,
    pub metadata: DatasetMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetMetadata {
    pub description: String,
    pub rows: Option<u64>,
    pub columns: Option<u64>,
    pub column_names: Vec<String>,
    pub format_quality: FormatQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatQuality {
    #[serde(rename = "machine-readable")]
    pub machine_readable: u8,
    #[serde(rename = "open-format")]
    pub open_format: u8,
}

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

/// Represents analysis results from the LLM.
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
