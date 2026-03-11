use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::LlmAnalysis;

const OLLAMA_API_URL: &str = "http://localhost:11434/api/generate";
const MODEL_NAME: &str = "qwen3:2b";
const MAX_HTML_LENGTH: usize = 4000;

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
pub async fn analyze_website(
    html_content: &str,
    client: &reqwest::Client,
) -> LlmAnalysis {
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

    match query_ollama(&prompt, client).await {
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
    client: &reqwest::Client,
) -> Result<String, Box<dyn std::error::Error>> {
    let request = OllamaRequest {
        model: MODEL_NAME.to_string(),
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
                    let mut map = HashMap::new();
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

/// Analyze a dataset file using LLM to generate a description.
pub async fn analyze_dataset_file(
    file_path: &str,
    file_type: &str,
    column_names: &[String],
    client: &reqwest::Client,
) -> Option<String> {
    let prompt = format!(
        r#"This is a {file_type} data file at path "{file_path}".
The columns are: {columns}.
Write a brief one-sentence description of what this dataset likely contains.
Respond with ONLY the description, nothing else."#,
        columns = if column_names.is_empty() {
            "unknown".to_string()
        } else {
            column_names.join(", ")
        }
    );

    match query_ollama(&prompt, client).await {
        Ok(description) => Some(description.trim().to_string()),
        Err(_) => None,
    }
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
        assert_eq!(json, r#"{"necessity_of_login": 0, "search_for_dataset": 1}"#);
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
}
