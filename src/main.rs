mod cli;
mod crawler;
mod downloader;
mod extractor;
mod llm;
mod models;
mod output;

use std::path::PathBuf;

use models::{Category, QualityReport, Website};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Stage 1: CLI Input
    let config = cli::collect_app_config();

    let client = reqwest::Client::builder()
        .user_agent("OSDAT/0.1.0 (Open Data Portal Quality Assessment Tool)")
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    // Stage 1 (continued): LLM-assisted platform-level analysis
    println!("\n=== Analyzing root website with AI ===");
    let root_html = match crawler::fetch_page_html(&config.root_url, &client).await {
        Ok(html) => html,
        Err(e) => {
            eprintln!("Warning: Could not fetch root URL: {e}");
            String::new()
        }
    };

    let llm_analysis = llm::analyze_website(&root_html, &client).await;
    let platform_level = cli::collect_platform_level_with_defaults(&llm_analysis);

    // Stage 2: Web Crawling & File Discovery
    println!("\n=== Stage 2: Crawling pages for data files ===");
    let crawl_results = crawler::crawl_pages(&config.page_urls, &client).await?;

    // Flatten all discovered files
    let all_files: Vec<models::DiscoveredFile> = crawl_results
        .values()
        .flat_map(|files| files.iter().cloned())
        .collect();

    println!("\nTotal unique data files discovered: {}", all_files.len());

    if all_files.is_empty() {
        println!("No data files found. Generating report with empty dataset list.");
    }

    // Stage 3: File Download
    println!("\n=== Stage 3: Downloading discovered files ===");
    let download_dir = PathBuf::from("osdat_downloads");
    tokio::fs::create_dir_all(&download_dir).await?;

    let downloaded = downloader::download_all_files(&all_files, &download_dir, &client).await;
    println!(
        "Successfully downloaded: {}/{}",
        downloaded.len(),
        all_files.len()
    );

    // Stage 4: Data Extraction
    println!("\n=== Stage 4: Extracting metadata from files ===");
    let mut datasets = Vec::new();

    for (file_info, local_path, file_size) in &downloaded {
        println!("  Processing: {}", local_path.display());
        let dataset = extractor::extract_metadata(
            local_path,
            &file_info.download_url,
            &file_info.source_page_url,
            &file_info.file_extension,
            *file_size,
        );
        datasets.push(dataset);
    }

    // Stage 5: LLM-Assisted Analysis for dataset descriptions
    println!("\n=== Stage 5: Enhancing dataset descriptions with AI ===");
    for dataset in &mut datasets {
        if let Some(description) = llm::analyze_dataset_file(
            &dataset.download_url,
            &dataset.file_type,
            &dataset.metadata.column_names,
            &client,
        )
        .await
        {
            dataset.metadata.description = description;
        }
    }

    // Stage 6: JSON Generation
    println!("\n=== Stage 6: Generating quality assessment report ===");

    let mut final_platform_level = platform_level;
    final_platform_level.diversity.number_of_dataset = datasets.len() as u32;

    let report = QualityReport {
        website: Website {
            url: config.root_url.clone(),
        },
        platform_level: final_platform_level,
        categories: vec![Category {
            name: config.category_name.clone(),
            datasets,
        }],
    };

    let output_path = PathBuf::from(&config.output_filename);
    output::write_report(&report, &output_path)?;

    // Cleanup downloaded files
    if download_dir.exists() {
        println!("Cleaning up temporary downloads...");
        tokio::fs::remove_dir_all(&download_dir).await.ok();
    }

    println!("\n=== Assessment complete! ===");
    println!("Output saved to: {}", config.output_filename);

    Ok(())
}
