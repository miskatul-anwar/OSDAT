use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::models::DiscoveredFile;

/// Maximum allowed file size for downloads (500 MB).
const MAX_FILE_SIZE: u64 = 500 * 1024 * 1024;

/// Download a file from a URL and save it to the specified directory.
/// Returns the local file path and the file size in bytes.
/// Rejects files that exceed MAX_FILE_SIZE.
pub async fn download_file(
    file: &DiscoveredFile,
    download_dir: &Path,
    client: &reqwest::Client,
) -> Result<(PathBuf, u64), Box<dyn std::error::Error>> {
    let filename = generate_filename(&file.download_url, &file.file_extension);
    let file_path = download_dir.join(&filename);

    let response = client.get(&file.download_url).send().await?;

    if !response.status().is_success() {
        return Err(format!("HTTP {} for {}", response.status(), file.download_url).into());
    }

    // Check Content-Length header if available
    if let Some(content_length) = response.content_length() {
        if content_length > MAX_FILE_SIZE {
            return Err(format!(
                "File too large ({} bytes, max {})",
                content_length, MAX_FILE_SIZE
            )
            .into());
        }
    }

    let bytes = response.bytes().await?;
    let size = bytes.len() as u64;

    if size > MAX_FILE_SIZE {
        return Err(format!("Downloaded file too large ({size} bytes, max {MAX_FILE_SIZE})").into());
    }

    let mut out_file = fs::File::create(&file_path).await?;
    out_file.write_all(&bytes).await?;

    Ok((file_path, size))
}

/// Download all discovered files into a temporary directory.
/// Returns a vector of (DiscoveredFile, local_path, file_size) tuples.
pub async fn download_all_files(
    files: &[DiscoveredFile],
    download_dir: &Path,
    client: &reqwest::Client,
) -> Vec<(DiscoveredFile, PathBuf, u64)> {
    let mut results = Vec::new();

    for file in files {
        println!("  Downloading: {}", file.download_url);
        match download_file(file, download_dir, client).await {
            Ok((path, size)) => {
                println!("    Saved: {} ({} bytes)", path.display(), size);
                results.push((file.clone(), path, size));
            }
            Err(e) => {
                eprintln!("    Error downloading {}: {e}", file.download_url);
            }
        }
    }

    results
}

/// Generate a safe local filename from a URL.
/// Prevents path traversal by sanitizing the filename.
fn generate_filename(url: &str, extension: &str) -> String {
    use uuid::Uuid;

    // Try to extract filename from URL path
    if let Ok(parsed) = url::Url::parse(url) {
        let path = parsed.path();
        if let Some(segment) = path.rsplit('/').next() {
            let decoded = urldecode(segment);
            // Reject if it contains path traversal patterns
            if !decoded.is_empty()
                && decoded.contains('.')
                && !decoded.contains("..")
                && !decoded.contains('/')
                && !decoded.contains('\\')
            {
                // Sanitize: only allow safe characters
                let safe: String = decoded
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect();
                if !safe.is_empty() {
                    return safe;
                }
            }
        }
    }

    // Fallback: use UUID
    format!("{}{}", Uuid::new_v4(), extension)
}

/// Simple URL decoding for common percent-encoded characters.
fn urldecode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            } else {
                result.push('%');
                result.push_str(&hex);
            }
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_filename_from_url() {
        let name = generate_filename("https://example.com/files/report.pdf", ".pdf");
        assert_eq!(name, "report.pdf");
    }

    #[test]
    fn test_generate_filename_with_special_chars() {
        let name = generate_filename("https://example.com/files/my report (1).csv", ".csv");
        assert_eq!(name, "my_report__1_.csv");
    }

    #[test]
    fn test_generate_filename_fallback() {
        let name = generate_filename("https://example.com/", ".csv");
        assert!(name.ends_with(".csv"));
        assert!(name.len() > 4); // UUID + .csv
    }

    #[test]
    fn test_urldecode() {
        assert_eq!(urldecode("hello%20world"), "hello world");
        assert_eq!(urldecode("data%2Ffile.csv"), "data/file.csv");
        assert_eq!(urldecode("normal"), "normal");
    }
}
