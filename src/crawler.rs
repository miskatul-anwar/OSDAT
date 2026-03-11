use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet};
use url::Url;

use crate::models::DiscoveredFile;

/// Recognized data file extensions.
const DATA_EXTENSIONS: &[&str] = &[
    ".pdf", ".xlsx", ".xls", ".csv", ".xml", ".docx", ".doc", ".pptx", ".ppt", ".txt",
];

/// Crawl a list of page URLs and discover downloadable data file links.
pub async fn crawl_pages(
    page_urls: &[String],
    client: &reqwest::Client,
) -> Result<HashMap<String, Vec<DiscoveredFile>>, Box<dyn std::error::Error>> {
    let mut results: HashMap<String, Vec<DiscoveredFile>> = HashMap::new();
    let mut seen_urls: HashSet<String> = HashSet::new();

    for page_url in page_urls {
        println!("  Crawling: {page_url}");
        match client.get(page_url).send().await {
            Ok(response) => {
                if !response.status().is_success() {
                    eprintln!(
                        "  Warning: HTTP {} for {page_url}",
                        response.status()
                    );
                    continue;
                }
                let html_text = response.text().await.unwrap_or_default();
                let discovered = extract_file_links(&html_text, page_url, &mut seen_urls);
                println!("    Found {} data file link(s)", discovered.len());
                results.insert(page_url.clone(), discovered);
            }
            Err(e) => {
                eprintln!("  Error fetching {page_url}: {e}");
            }
        }
    }

    Ok(results)
}

/// Extract file download links from HTML content.
pub fn extract_file_links(
    html: &str,
    base_url: &str,
    seen: &mut HashSet<String>,
) -> Vec<DiscoveredFile> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a[href]").expect("Invalid selector");
    let mut files = Vec::new();

    let base = match Url::parse(base_url) {
        Ok(u) => u,
        Err(_) => return files,
    };

    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            let href = href.trim();
            if href.is_empty() {
                continue;
            }

            // Resolve relative URLs
            let absolute_url = match base.join(href) {
                Ok(u) => u.to_string(),
                Err(_) => continue,
            };

            // Check if URL ends with a recognized data file extension
            if let Some(ext) = get_data_extension(&absolute_url) {
                if seen.insert(absolute_url.clone()) {
                    files.push(DiscoveredFile {
                        source_page_url: base_url.to_string(),
                        download_url: absolute_url,
                        file_extension: ext.to_string(),
                    });
                }
            }
        }
    }

    files
}

/// Check if a URL path ends with a recognized data file extension.
/// Returns the extension (with dot) if found.
fn get_data_extension(url_str: &str) -> Option<&'static str> {
    // Parse URL to get just the path (ignoring query strings, fragments)
    let path = if let Ok(url) = Url::parse(url_str) {
        url.path().to_lowercase()
    } else {
        url_str.to_lowercase()
    };

    for ext in DATA_EXTENSIONS {
        if path.ends_with(ext) {
            return Some(ext);
        }
    }
    None
}

/// Fetch and return the HTML content of a page.
pub async fn fetch_page_html(
    url: &str,
    client: &reqwest::Client,
) -> Result<String, Box<dyn std::error::Error>> {
    let response = client.get(url).send().await?;
    let html = response.text().await?;
    Ok(html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_data_extension() {
        assert_eq!(get_data_extension("https://example.com/data.csv"), Some(".csv"));
        assert_eq!(get_data_extension("https://example.com/report.pdf"), Some(".pdf"));
        assert_eq!(get_data_extension("https://example.com/data.xlsx"), Some(".xlsx"));
        assert_eq!(get_data_extension("https://example.com/page.html"), None);
        assert_eq!(get_data_extension("https://example.com/file.CSV"), Some(".csv"));
        assert_eq!(
            get_data_extension("https://example.com/data.csv?download=true"),
            Some(".csv")
        );
    }

    #[test]
    fn test_extract_file_links_basic() {
        let html = r#"
            <html>
            <body>
                <a href="data.csv">CSV Data</a>
                <a href="report.pdf">PDF Report</a>
                <a href="/files/info.xlsx">Excel File</a>
                <a href="page.html">Regular Page</a>
                <a href="">Empty Link</a>
            </body>
            </html>
        "#;

        let mut seen = HashSet::new();
        let files = extract_file_links(html, "https://example.com/downloads/", &mut seen);

        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|f| f.download_url == "https://example.com/downloads/data.csv"));
        assert!(files.iter().any(|f| f.download_url == "https://example.com/downloads/report.pdf"));
        assert!(files.iter().any(|f| f.download_url == "https://example.com/files/info.xlsx"));
    }

    #[test]
    fn test_extract_file_links_deduplication() {
        let html = r#"
            <html>
            <body>
                <a href="data.csv">Link 1</a>
                <a href="data.csv">Link 2 (duplicate)</a>
            </body>
            </html>
        "#;

        let mut seen = HashSet::new();
        let files = extract_file_links(html, "https://example.com/", &mut seen);

        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_extract_file_links_all_extensions() {
        let html = r#"
            <html><body>
                <a href="a.pdf">1</a>
                <a href="b.xlsx">2</a>
                <a href="c.xls">3</a>
                <a href="d.csv">4</a>
                <a href="e.xml">5</a>
                <a href="f.docx">6</a>
                <a href="g.doc">7</a>
                <a href="h.pptx">8</a>
                <a href="i.ppt">9</a>
                <a href="j.txt">10</a>
            </body></html>
        "#;

        let mut seen = HashSet::new();
        let files = extract_file_links(html, "https://example.com/", &mut seen);
        assert_eq!(files.len(), 10);
    }
}
