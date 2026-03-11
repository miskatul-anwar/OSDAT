mod cli;
mod crawler;
mod downloader;
mod extractor;
mod llm;
mod models;
mod output;

use indexmap::IndexMap;
use std::collections::HashMap;
use std::path::PathBuf;

use models::{
    DatasetEntry, PortalQualityAssessment, QualityReport, WebsiteReport,
};

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

    // Store page HTML for dataset name extraction
    let mut page_htmls: HashMap<String, String> = HashMap::new();
    for page_url in &config.page_urls {
        if let Ok(html) = crawler::fetch_page_html(page_url, &client).await {
            page_htmls.insert(page_url.clone(), html);
        }
    }

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

    // Stage 4: Data Extraction (returns Vec per file for multi-dataset support)
    println!("\n=== Stage 4: Extracting metadata from files ===");
    let mut all_extracted = Vec::new();

    for (file_info, local_path, file_size) in &downloaded {
        println!("  Processing: {}", local_path.display());
        let mut datasets = extractor::extract_metadata(
            local_path,
            &file_info.download_url,
            &file_info.source_page_url,
            &file_info.file_extension,
            *file_size,
        );

        // Try to extract dataset name from the source page HTML
        for data in &mut datasets {
            if let Some(html) = page_htmls.get(&file_info.source_page_url) {
                if let Some(name) =
                    llm::extract_dataset_name_from_html(html, &file_info.download_url)
                {
                    data.title = name;
                }
            }
        }

        all_extracted.extend(datasets);
    }

    // Stage 5: RAG-Assisted Analysis for dataset metadata
    println!("\n=== Stage 5: Enhancing dataset metadata with AI (RAG) ===");
    let mut dataset_entries = IndexMap::new();

    let site_name = url::Url::parse(&config.root_url)
        .map(|u| u.host_str().unwrap_or("Unknown").to_string())
        .unwrap_or_else(|_| "Unknown".to_string());

    for (idx, data) in all_extracted.iter().enumerate() {
        println!("  Analyzing dataset {}: {}", idx + 1, data.title);

        // Run RAG analysis
        let local_path = download_dir.join(format!(
            "{}",
            data.download_url
                .rsplit('/')
                .next()
                .unwrap_or("unknown")
        ));
        let rag_analysis = llm::analyze_dataset_with_rag(
            &local_path,
            &data.file_type,
            &data.column_names,
            &client,
        )
        .await;

        // Use RAG-detected dataset name if available, otherwise keep existing
        let dataset_name = rag_analysis
            .dataset_name
            .clone()
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| data.title.clone());

        // Auto-detect dataset-level and data-level fields
        let auto_dataset_level = models::auto_detect_dataset_level(data);
        let mut data_level = models::auto_detect_data_level(data);

        // Apply RAG granularity results
        if let Some(v) = rag_analysis.granularity_day {
            data_level.granularity.time_dimension.day = v;
        }
        if let Some(v) = rag_analysis.granularity_month {
            data_level.granularity.time_dimension.month = v;
        }
        if let Some(v) = rag_analysis.granularity_year {
            data_level.granularity.time_dimension.year = v;
        }
        if let Some(v) = rag_analysis.granularity_union {
            data_level.granularity.geo_dimension.union = v;
        }
        if let Some(v) = rag_analysis.granularity_upazila {
            data_level.granularity.geo_dimension.upazila = v;
        }
        if let Some(v) = rag_analysis.granularity_zila {
            data_level.granularity.geo_dimension.zila = v;
        }

        // Collect per-dataset fields from user (merging auto + RAG + manual)
        let dataset_level = cli::collect_dataset_level_fields(
            &dataset_name,
            &auto_dataset_level,
            &rag_analysis,
            &site_name,
        );

        let key = format!("dataset{}", idx + 1);
        dataset_entries.insert(
            key,
            DatasetEntry {
                dataset_name,
                url: data.source_url.clone(),
                dataset_level,
                data_level,
            },
        );
    }

    // Stage 6: JSON Generation
    println!("\n=== Stage 6: Generating quality assessment report ===");

    let mut final_platform_level = platform_level;
    final_platform_level.diversity.number_of_dataset = all_extracted.len() as u32;

    let mut category_map = HashMap::new();
    category_map.insert(config.category_name.clone(), dataset_entries);

    let report = QualityReport {
        website: WebsiteReport {
            url: config.root_url.clone(),
            portal_quality_assessment: PortalQualityAssessment {
                platform_level: final_platform_level,
            },
            category: category_map,
        },
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
