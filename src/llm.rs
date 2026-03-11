use serde::{Deserialize, Serialize};

use indexmap::IndexMap;

use crate::models::{DatasetRagAnalysis, LlmAnalysis};

const OLLAMA_API_URL: &str = "http://localhost:11434/api/generate";
const QWEN_MODEL: &str = "qwen3:2b";
const MISTRAL_MODEL: &str = "mistral:3b";
const MAX_HTML_LENGTH: usize = 8000;

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
}

/// Analyze a website's HTML content using the local Ollama LLM.
/// Returns an LlmAnalysis with detected values (or None for uncertain fields).
pub async fn analyze_website(html_content: &str, client: &reqwest::Client) -> LlmAnalysis {
    // Truncate HTML to avoid overwhelming the model
    let truncated = if html_content.len() > MAX_HTML_LENGTH {
        &html_content[..MAX_HTML_LENGTH]
    } else {
        html_content
    };

    let prompt = format!(
        r#"Analyze this government website HTML and answer these questions with ONLY a JSON object.
No explanation, just JSON.

HTML content:
{truncated}

Answer these questions about the website:
1. Does the site require login to access data? (0=no, 1=yes)
2. Does the site support multiple languages? (0=no, 1=yes)
3. Is there a mechanism to request new datasets? (0=no, 1=yes)
4. Which languages are supported? List them.
5. Can users browse datasets by category? (0=no, 1=yes)
6. Are filter/sort options available? (0=no, 1=yes)
7. Is there a search feature? (0=no, 1=yes)
8. Are usage guidelines provided? (0=no, 1=yes)
9. How many dataset categories exist? (number)

Respond with ONLY this JSON format:
{{
  "necessity_of_login": 0,
  "multiple_language_support": 0,
  "request_for_datasets": 0,
  "languages": {{"bangla": 0, "english": 1}},
  "browse_datasets_by_category": 0,
  "filter_sort_datasets": 0,
  "search_for_dataset": 0,
  "user_guideline": 0,
  "number_of_category": 1
}}"#
    );

    match query_ollama(&prompt, QWEN_MODEL, client).await {
        Ok(response_text) => parse_llm_response(&response_text),
        Err(e) => {
            eprintln!("  LLM analysis unavailable ({e}). Will prompt user for all fields.");
            LlmAnalysis::default()
        }
    }
}

/// Send a prompt to the local Ollama API and return the response text.
async fn query_ollama(
    prompt: &str,
    model: &str,
    client: &reqwest::Client,
) -> Result<String, Box<dyn std::error::Error>> {
    let request = OllamaRequest {
        model: model.to_string(),
        prompt: prompt.to_string(),
        stream: false,
    };

    let response = client
        .post(OLLAMA_API_URL)
        .json(&request)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Ollama returned HTTP {}", response.status()).into());
    }

    let ollama_response: OllamaResponse = response.json().await?;
    Ok(ollama_response.response)
}

/// Parse the LLM's JSON response into an LlmAnalysis struct.
fn parse_llm_response(response: &str) -> LlmAnalysis {
    // Try to extract JSON from the response (the model may include extra text)
    let json_str = extract_json_from_text(response);

    match serde_json::from_str::<serde_json::Value>(&json_str) {
        Ok(value) => LlmAnalysis {
            necessity_of_login: value
                .get("necessity_of_login")
                .and_then(|v| v.as_u64())
                .map(|v| v as u8),
            multiple_language_support: value
                .get("multiple_language_support")
                .and_then(|v| v.as_u64())
                .map(|v| v as u8),
            request_for_datasets: value
                .get("request_for_datasets")
                .and_then(|v| v.as_u64())
                .map(|v| v as u8),
            languages: value.get("languages").and_then(|v| {
                if let Some(obj) = v.as_object() {
                    let mut map = IndexMap::new();
                    for (k, val) in obj {
                        if let Some(n) = val.as_u64() {
                            map.insert(k.clone(), n as u8);
                        }
                    }
                    Some(map)
                } else {
                    None
                }
            }),
            browse_datasets_by_category: value
                .get("browse_datasets_by_category")
                .and_then(|v| v.as_u64())
                .map(|v| v as u8),
            filter_sort_datasets: value
                .get("filter_sort_datasets")
                .and_then(|v| v.as_u64())
                .map(|v| v as u8),
            search_for_dataset: value
                .get("search_for_dataset")
                .and_then(|v| v.as_u64())
                .map(|v| v as u8),
            user_guideline: value
                .get("user_guideline")
                .and_then(|v| v.as_u64())
                .map(|v| v as u8),
            number_of_category: value
                .get("number_of_category")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
        },
        Err(e) => {
            eprintln!("  Warning: Could not parse LLM response as JSON: {e}");
            LlmAnalysis::default()
        }
    }
}

/// Extract the first JSON object from text that may contain surrounding prose.
fn extract_json_from_text(text: &str) -> String {
    // Find the first '{' and last '}' to extract JSON
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            if end > start {
                return text[start..=end].to_string();
            }
        }
    }
    text.to_string()
}

/// Analyze a dataset file using RAG to extract structured metadata.
/// Reads file content and sends it along with structured prompts to the LLM.
pub async fn analyze_dataset_with_rag(
    file_path: &std::path::Path,
    file_type: &str,
    column_names: &[String],
    client: &reqwest::Client,
) -> DatasetRagAnalysis {
    // Read file content for RAG
    let content_snippet = read_file_content_snippet(file_path, file_type);

    let columns_str = if column_names.is_empty() {
        "unknown".to_string()
    } else {
        column_names.join(", ")
    };

    let prompt = format!(
        r#"Analyze this {file_type} data file and extract structured metadata.

Column names: {columns_str}

Content snippet:
{content_snippet}

Respond with ONLY a JSON object containing these fields:
{{
  "dataset_name": "A descriptive name for this dataset (in the language of the content if non-English)",
  "time_period": "Time period covered (e.g. '2019-2020') or empty string if unknown",
  "update_activity": "How often updated (e.g. 'yearly', 'monthly', 'one-time') or empty string if unknown",
  "last_update": "Last update date or empty string if unknown",
  "collection_method": "How data was collected or empty string if unknown",
  "granularity_day": 0,
  "granularity_month": 0,
  "granularity_year": 0,
  "granularity_union": "Name of union if data is at union level, or empty string",
  "granularity_upazila": "Name of upazila if data is at upazila level, or empty string",
  "granularity_zila": "Name of zila if data is at zila level, or empty string"
}}

For time granularity fields, use 1 if the data has that time dimension, 0 otherwise.
For geo granularity fields, use the name of the administrative area or empty string if not applicable."#
    );

    match query_ollama(&prompt, QWEN_MODEL, client).await {
        Ok(response_text) => parse_rag_response(&response_text),
        Err(_) => DatasetRagAnalysis::default(),
    }
}

/// Read a snippet of file content for RAG analysis.
fn read_file_content_snippet(file_path: &std::path::Path, file_type: &str) -> String {
    let max_chars = 3000;

    match file_type {
        "CSV" | "TXT" => std::fs::read_to_string(file_path)
            .map(|s| {
                if s.len() > max_chars {
                    s[..max_chars].to_string()
                } else {
                    s
                }
            })
            .unwrap_or_default(),
        "XML" | "RDF" => std::fs::read_to_string(file_path)
            .map(|s| {
                if s.len() > max_chars {
                    s[..max_chars].to_string()
                } else {
                    s
                }
            })
            .unwrap_or_default(),
        "PDF" => read_pdf_text(file_path, max_chars),
        "XLSX" | "XLS" => read_xlsx_text(file_path, max_chars),
        _ => format!("[Binary {file_type} file — column names used for analysis]"),
    }
}

/// Extract text from a PDF file using pdftotext subprocess.
fn read_pdf_text(file_path: &std::path::Path, max_chars: usize) -> String {
    use std::process::Command;

    // Try pdftotext first (poppler-utils)
    if let Ok(output) = Command::new("pdftotext")
        .args([
            file_path.display().to_string().as_str(),
            "-",
        ])
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).to_string();
            if !text.trim().is_empty() {
                if text.len() > max_chars {
                    return text[..max_chars].to_string();
                }
                return text;
            }
        }
    }

    format!("[Binary PDF file — column names used for analysis]")
}

/// Read content from an XLSX file using calamine.
fn read_xlsx_text(file_path: &std::path::Path, max_chars: usize) -> String {
    use calamine::{open_workbook_auto, DataType, Reader};

    match open_workbook_auto(file_path) {
        Ok(mut workbook) => {
            let mut text = String::new();
            if let Some(sheet_name) = workbook.sheet_names().first().cloned() {
                if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                    for row in range.rows() {
                        let row_text: Vec<String> = row
                            .iter()
                            .map(|cell| {
                                if cell.is_empty() {
                                    String::new()
                                } else {
                                    format!("{cell}")
                                }
                            })
                            .collect();
                        text.push_str(&row_text.join("\t"));
                        text.push('\n');
                        if text.len() > max_chars {
                            return text[..max_chars].to_string();
                        }
                    }
                }
            }
            if text.is_empty() {
                format!("[Binary XLSX file — column names used for analysis]")
            } else {
                text
            }
        }
        Err(_) => format!("[Binary XLSX file — could not read content]"),
    }
}

/// Parse RAG analysis response into DatasetRagAnalysis.
fn parse_rag_response(response: &str) -> DatasetRagAnalysis {
    let json_str = extract_json_from_text(response);

    match serde_json::from_str::<serde_json::Value>(&json_str) {
        Ok(value) => DatasetRagAnalysis {
            dataset_name: value
                .get("dataset_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            time_period: value
                .get("time_period")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            update_activity: value
                .get("update_activity")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            last_update: value
                .get("last_update")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            collection_method: value
                .get("collection_method")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            granularity_day: value
                .get("granularity_day")
                .and_then(|v| v.as_u64())
                .map(|v| v as u8),
            granularity_month: value
                .get("granularity_month")
                .and_then(|v| v.as_u64())
                .map(|v| v as u8),
            granularity_year: value
                .get("granularity_year")
                .and_then(|v| v.as_u64())
                .map(|v| v as u8),
            granularity_union: value
                .get("granularity_union")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            granularity_upazila: value
                .get("granularity_upazila")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            granularity_zila: value
                .get("granularity_zila")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        },
        Err(_) => DatasetRagAnalysis::default(),
    }
}

/// Use mistral:3b to verify and potentially merge PDF table extraction results.
/// Returns a suggested number of distinct datasets and their table groupings.
pub async fn verify_pdf_tables_with_mistral(
    table_summaries: &str,
    client: &reqwest::Client,
) -> Option<Vec<Vec<usize>>> {
    let prompt = format!(
        r#"These are tables extracted from a PDF file. Determine how many distinct datasets exist
and which tables should be merged together.

Tables:
{table_summaries}

Respond with ONLY a JSON object:
{{
  "datasets": [[0, 1], [2, 3]]
}}

Where each inner array contains the table indices that form one logical dataset.
If all tables are one dataset, respond: {{"datasets": [[0, 1, 2, ...]]}}"#
    );

    match query_ollama(&prompt, MISTRAL_MODEL, client).await {
        Ok(response_text) => {
            let json_str = extract_json_from_text(&response_text);
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json_str) {
                if let Some(datasets) = value.get("datasets").and_then(|v| v.as_array()) {
                    let result: Vec<Vec<usize>> = datasets
                        .iter()
                        .filter_map(|d| {
                            d.as_array().map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_u64().map(|n| n as usize))
                                    .collect()
                            })
                        })
                        .collect();
                    if !result.is_empty() {
                        return Some(result);
                    }
                }
            }
            None
        }
        Err(_) => None,
    }
}

/// Extract dataset name from HTML page near the download link.
pub fn extract_dataset_name_from_html(html: &str, download_url: &str) -> Option<String> {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);

    // Try to find an <a> tag linking to this URL and get nearby heading text
    if let Ok(selector) = Selector::parse("a[href]") {
        for element in document.select(&selector) {
            if let Some(href) = element.value().attr("href") {
                if href.contains(download_url) || download_url.contains(href) {
                    // Check for text content of the link itself
                    let link_text: String = element.text().collect::<String>().trim().to_string();
                    if !link_text.is_empty() && link_text.len() > 3 {
                        return Some(link_text);
                    }

                    // Try to find the nearest parent with heading content
                    break;
                }
            }
        }
    }

    // Fallback: try to find the page title
    if let Ok(selector) = Selector::parse("h1, h2, h3, title") {
        for element in document.select(&selector) {
            let text: String = element.text().collect::<String>().trim().to_string();
            if !text.is_empty() && text.len() > 3 {
                return Some(text);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_text() {
        let text = r#"Here is the analysis:
{"necessity_of_login": 0, "search_for_dataset": 1}
That's the result."#;
        let json = extract_json_from_text(text);
        assert_eq!(
            json,
            r#"{"necessity_of_login": 0, "search_for_dataset": 1}"#
        );
    }

    #[test]
    fn test_extract_json_no_json() {
        let text = "No JSON here";
        let json = extract_json_from_text(text);
        assert_eq!(json, "No JSON here");
    }

    #[test]
    fn test_parse_llm_response_valid() {
        let response = r#"{"necessity_of_login": 0, "multiple_language_support": 1, "request_for_datasets": 0, "languages": {"bangla": 1, "english": 1}, "browse_datasets_by_category": 1, "filter_sort_datasets": 0, "search_for_dataset": 1, "user_guideline": 0, "number_of_category": 5}"#;
        let analysis = parse_llm_response(response);

        assert_eq!(analysis.necessity_of_login, Some(0));
        assert_eq!(analysis.multiple_language_support, Some(1));
        assert_eq!(analysis.search_for_dataset, Some(1));
        assert_eq!(analysis.number_of_category, Some(5));
        assert!(analysis.languages.is_some());
        let langs = analysis.languages.unwrap();
        assert_eq!(langs.get("bangla"), Some(&1));
        assert_eq!(langs.get("english"), Some(&1));
    }

    #[test]
    fn test_parse_llm_response_invalid() {
        let response = "This is not JSON at all";
        let analysis = parse_llm_response(response);
        // All fields should be None when parsing fails
        assert!(analysis.necessity_of_login.is_none());
        assert!(analysis.languages.is_none());
    }

    #[test]
    fn test_parse_llm_response_partial() {
        let response = r#"{"necessity_of_login": 1}"#;
        let analysis = parse_llm_response(response);
        assert_eq!(analysis.necessity_of_login, Some(1));
        assert!(analysis.multiple_language_support.is_none());
    }

    #[test]
    fn test_parse_rag_response() {
        let response = r#"{"dataset_name": "Test Dataset", "time_period": "2020-2021", "update_activity": "yearly", "last_update": "2021-12-31", "collection_method": "survey", "granularity_day": 0, "granularity_month": 1, "granularity_year": 1, "granularity_union": "", "granularity_upazila": "", "granularity_zila": "Dhaka"}"#;
        let rag = parse_rag_response(response);
        assert_eq!(rag.dataset_name, Some("Test Dataset".to_string()));
        assert_eq!(rag.time_period, Some("2020-2021".to_string()));
        assert_eq!(rag.granularity_month, Some(1));
        assert_eq!(rag.granularity_zila, Some("Dhaka".to_string()));
        assert_eq!(rag.granularity_union, Some(String::new()));
    }

    #[test]
    fn test_extract_dataset_name_from_html() {
        let html = r#"
            <html><body>
            <h1>গবেষণা কর্মের তালিকা</h1>
            <a href="https://example.com/data.csv">২০১৯-২০২০ অর্থ বছরের গবেষণা কর্মের তালিকা</a>
            </body></html>
        "#;
        let name =
            extract_dataset_name_from_html(html, "https://example.com/data.csv");
        assert!(name.is_some());
        assert!(name.unwrap().contains("গবেষণা"));
    }

    #[test]
    fn test_extract_dataset_name_from_html_fallback_heading() {
        let html = r#"
            <html><body>
            <h2>Research Data Portal</h2>
            <a href="https://example.com/other.csv">Download</a>
            </body></html>
        "#;
        // URL doesn't match any link, but heading should be found
        let name =
            extract_dataset_name_from_html(html, "https://example.com/data.csv");
        assert!(name.is_some());
        assert_eq!(name.unwrap(), "Research Data Portal");
    }
}
