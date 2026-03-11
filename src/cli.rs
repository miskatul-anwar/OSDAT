use std::io::{self, BufRead, Write};

use indexmap::IndexMap;

use crate::models::{
    AppConfig, DatasetLevel, DatasetRagAnalysis, FiveStar, Openness, OpennessComplete, Provenance,
    SemanticConsistency, Transparency, Understandability,
};

/// Read a single line of user input with a prompt message.
pub fn prompt(message: &str) -> String {
    print!("{}", message);
    io::stdout().flush().expect("Failed to flush stdout");
    let mut input = String::new();
    io::stdin()
        .lock()
        .read_line(&mut input)
        .expect("Failed to read input");
    input.trim().to_string()
}

/// Read a 0 or 1 value from the user, with a description of the field.
pub fn prompt_binary(field_name: &str, description: &str) -> u8 {
    println!("\n  Field: {field_name}");
    println!("  Description: {description}");
    loop {
        let input = prompt("  Enter 0 or 1: ");
        match input.as_str() {
            "0" => return 0,
            "1" => return 1,
            _ => println!("  Invalid input. Please enter 0 or 1."),
        }
    }
}

/// Read a u32 value from the user.
pub fn prompt_u32(field_name: &str, description: &str) -> u32 {
    println!("\n  Field: {field_name}");
    println!("  Description: {description}");
    loop {
        let input = prompt("  Enter a number: ");
        match input.parse::<u32>() {
            Ok(n) => return n,
            Err(_) => println!("  Invalid input. Please enter a valid number."),
        }
    }
}

/// Collect basic application configuration from the user interactively.
pub fn collect_app_config() -> AppConfig {
    println!("=== OSDAT — Open Data Portal Quality Assessment Tool ===\n");

    let root_url = prompt("Enter root website URL (e.g. https://sparrso.gov.bd/): ");

    println!("\nEnter page URLs to crawl (one per line, blank line to finish):");
    let mut page_urls = Vec::new();
    loop {
        let url = prompt("> ");
        if url.is_empty() {
            break;
        }
        page_urls.push(url);
    }

    let output_filename = loop {
        let name = prompt("\nEnter output JSON filename (e.g. sparrso.json): ");
        if is_safe_filename(&name) {
            break name;
        }
        println!("Invalid filename. Must not contain path separators or '..' sequences.");
    };
    let category_name = prompt("Enter category name (e.g. গবেষণা): ");

    AppConfig {
        root_url,
        page_urls,
        output_filename,
        category_name,
    }
}

/// Check if a filename is safe (no path traversal).
fn is_safe_filename(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..")
        && !name.starts_with('.')
}

/// Collect platform-level fields from the user, using LLM suggestions as defaults.
pub fn collect_platform_level_with_defaults(
    llm_analysis: &crate::models::LlmAnalysis,
) -> crate::models::PlatformLevel {
    println!("\n=== Platform-Level Data Collection ===");
    println!("(AI-detected values shown as defaults; press Enter to accept or type a new value)\n");

    let necessity_of_login = prompt_with_default_binary(
        "necessity-of-login",
        "Does the site require login to access data?",
        llm_analysis.necessity_of_login,
    );

    let multiple_language_support = prompt_with_default_binary(
        "multiple-language-support",
        "Does the site offer multiple languages?",
        llm_analysis.multiple_language_support,
    );

    let request_for_datasets = prompt_with_default_binary(
        "request-for-datasets",
        "Is there a mechanism to request new datasets?",
        llm_analysis.request_for_datasets,
    );

    let languages = if let Some(ref detected) = llm_analysis.languages {
        println!("\n  AI detected languages: {:?}", detected);
        let accept = prompt("  Accept detected languages? (y/n): ");
        if accept.to_lowercase() == "y" {
            detected.clone()
        } else {
            collect_languages()
        }
    } else {
        collect_languages()
    };

    let browse_datasets_by_category = prompt_with_default_binary(
        "browse-data-sets-by-category",
        "Can users browse datasets by category?",
        llm_analysis.browse_datasets_by_category,
    );

    let filter_sort_datasets = prompt_with_default_binary(
        "filter-and/or-sort-datasets",
        "Are filter/sort options available?",
        llm_analysis.filter_sort_datasets,
    );

    let search_for_dataset = prompt_with_default_binary(
        "search-for-dataset",
        "Is there a search feature?",
        llm_analysis.search_for_dataset,
    );

    let user_guideline = prompt_with_default_binary(
        "user-guideline",
        "Are usage guidelines provided?",
        llm_analysis.user_guideline,
    );

    let number_of_category = prompt_with_default_u32(
        "number-of-category",
        "Total number of dataset categories on the site",
        llm_analysis.number_of_category,
    );

    crate::models::PlatformLevel {
        user_accessibility: crate::models::UserAccessibility {
            necessity_of_login,
            multiple_language_support,
            request_for_datasets,
            languages,
        },
        user_usability: crate::models::UserUsability {
            browse_datasets_by_category,
            filter_sort_datasets,
            search_for_dataset,
            user_guideline,
        },
        diversity: crate::models::Diversity {
            number_of_dataset: 0, // Will be updated after extraction
            number_of_category,
        },
    }
}

/// Collect per-dataset fields that cannot be auto-detected.
/// Merges auto-detected values with RAG analysis and user input.
pub fn collect_dataset_level_fields(
    dataset_name: &str,
    auto: &DatasetLevel,
    rag: &DatasetRagAnalysis,
    site_name: &str,
) -> DatasetLevel {
    println!(
        "\n  === Dataset-Level Fields for: {} ===",
        dataset_name
    );
    println!("  (Press Enter to accept auto-detected/AI values)\n");

    // --- openness.complete ---
    let descriptive = prompt_with_default_binary(
        "openness.complete.descriptive",
        "Is the dataset descriptive?",
        Some(auto.openness.complete.descriptive),
    );
    let linked_data = prompt_with_default_binary(
        "openness.complete.linked-data",
        "Is the dataset available as linked data?",
        Some(auto.openness.complete.linked_data),
    );

    // --- openness top-level ---
    let primary = prompt_with_default_binary(
        "openness.primary",
        "Is this primary/original data?",
        Some(auto.openness.primary),
    );
    let timely = prompt_with_default_binary(
        "openness.timely",
        "Is the data published in a timely manner?",
        Some(auto.openness.timely),
    );
    let license_free = prompt_with_default_binary(
        "openness.license-free",
        "Is the data license-free?",
        Some(auto.openness.license_free),
    );
    let non_discriminatory = prompt_with_default_binary(
        "openness.non-discriminatory",
        "Is the data available without discrimination?",
        Some(auto.openness.non_discriminatory),
    );
    let accessible = prompt_with_default_binary(
        "openness.accessible",
        "Is the data publicly accessible?",
        Some(auto.openness.accessible),
    );

    // --- transparency ---
    let transparency_source = if !auto.transparency.source.is_empty() {
        auto.transparency.source.clone()
    } else {
        site_name.to_string()
    };

    let number_of_downloads = prompt_with_default_u64(
        "transparency.number-of-downloads",
        "Number of downloads (if known)",
        Some(auto.transparency.number_of_downloads),
    );
    let faq = prompt_with_default_binary(
        "transparency.understandability.FAQ",
        "Is there a FAQ for this dataset?",
        Some(auto.transparency.understandability.faq),
    );
    let textual_description = prompt_with_default_binary(
        "transparency.understandability.textual-description",
        "Is there a textual description?",
        Some(auto.transparency.understandability.textual_description),
    );
    let category_tag = prompt_with_default_binary(
        "transparency.understandability.category-tag",
        "Is there a category tag?",
        Some(auto.transparency.understandability.category_tag),
    );
    let meta_data = prompt_with_default_binary(
        "transparency.meta-data",
        "Is metadata provided?",
        Some(auto.transparency.meta_data),
    );

    // --- 5* ---
    let open_standard = prompt_with_default_binary(
        "transparency.5*.open-standard",
        "Is the data in an open standard format (URIs)?",
        Some(auto.transparency.five_star.open_standard),
    );
    let five_star_linked = prompt_with_default_binary(
        "transparency.5*.linked-data",
        "Is the data linked to other data (5* linked data)?",
        Some(auto.transparency.five_star.linked_data),
    );

    // --- provenance (with RAG defaults) ---
    let prov_source = transparency_source.clone();

    let time_period = prompt_with_default_string(
        "provenance.time-period",
        "Time period covered by this dataset",
        rag.time_period.as_deref(),
    );
    let update_activity = prompt_with_default_string(
        "provenance.update-activity",
        "Update activity (e.g., yearly, monthly, one-time)",
        rag.update_activity.as_deref(),
    );
    let last_update = prompt_with_default_string(
        "provenance.last-update",
        "Last update date",
        rag.last_update.as_deref(),
    );
    let collection_method = prompt_with_default_string(
        "provenance.collection-method",
        "Data collection method",
        rag.collection_method.as_deref(),
    );

    // --- semantic consistency ---
    let external_vocabulary = prompt_with_default_binary(
        "semantic-consistency.external-vocabulary",
        "Does the dataset use an external vocabulary/ontology?",
        Some(auto.semantic_consistency.external_vocabulary),
    );

    DatasetLevel {
        openness: Openness {
            complete: OpennessComplete {
                descriptive,
                downloadable: auto.openness.complete.downloadable,
                machine_readable: auto.openness.complete.machine_readable,
                linked_data,
            },
            primary,
            non_discriminatory,
            accessible,
            timely,
            non_proprietary: auto.openness.non_proprietary,
            license_free,
            machine_readable: auto.openness.machine_readable.clone(),
        },
        transparency: Transparency {
            source: transparency_source,
            number_of_downloads,
            understandability: Understandability {
                faq,
                textual_description,
                category_tag,
            },
            meta_data,
            five_star: FiveStar {
                available_online: auto.transparency.five_star.available_online,
                machine_readable: auto.transparency.five_star.machine_readable,
                non_proprietary_format: auto.transparency.five_star.non_proprietary_format,
                open_standard,
                linked_data: five_star_linked,
            },
        },
        provenance: Provenance {
            source: prov_source,
            time_period,
            update_activity,
            last_update,
            collection_method,
        },
        semantic_consistency: SemanticConsistency {
            external_vocabulary,
        },
    }
}

fn prompt_with_default_binary(field_name: &str, description: &str, default: Option<u8>) -> u8 {
    println!("\n  Field: {field_name}");
    println!("  Description: {description}");
    if let Some(d) = default {
        println!("  AI suggestion: {d}");
        loop {
            let input = prompt("  Enter 0 or 1 (or press Enter to accept suggestion): ");
            if input.is_empty() {
                return d;
            }
            match input.as_str() {
                "0" => return 0,
                "1" => return 1,
                _ => println!("  Invalid input. Please enter 0 or 1."),
            }
        }
    } else {
        prompt_binary(field_name, description)
    }
}

fn prompt_with_default_u32(field_name: &str, description: &str, default: Option<u32>) -> u32 {
    println!("\n  Field: {field_name}");
    println!("  Description: {description}");
    if let Some(d) = default {
        println!("  AI suggestion: {d}");
        loop {
            let input = prompt("  Enter a number (or press Enter to accept suggestion): ");
            if input.is_empty() {
                return d;
            }
            match input.parse::<u32>() {
                Ok(n) => return n,
                Err(_) => println!("  Invalid input. Please enter a valid number."),
            }
        }
    } else {
        prompt_u32(field_name, description)
    }
}

fn prompt_with_default_u64(field_name: &str, description: &str, default: Option<u64>) -> u64 {
    println!("\n  Field: {field_name}");
    println!("  Description: {description}");
    if let Some(d) = default {
        println!("  Current value: {d}");
        loop {
            let input = prompt("  Enter a number (or press Enter to accept): ");
            if input.is_empty() {
                return d;
            }
            match input.parse::<u64>() {
                Ok(n) => return n,
                Err(_) => println!("  Invalid input. Please enter a valid number."),
            }
        }
    } else {
        println!("\n  Field: {field_name}");
        println!("  Description: {description}");
        loop {
            let input = prompt("  Enter a number: ");
            match input.parse::<u64>() {
                Ok(n) => return n,
                Err(_) => println!("  Invalid input. Please enter a valid number."),
            }
        }
    }
}

fn prompt_with_default_string(field_name: &str, description: &str, default: Option<&str>) -> String {
    println!("\n  Field: {field_name}");
    println!("  Description: {description}");
    if let Some(d) = default {
        if !d.is_empty() {
            println!("  AI suggestion: {d}");
            let input = prompt("  Enter value (or press Enter to accept suggestion): ");
            if input.is_empty() {
                return d.to_string();
            }
            return input;
        }
    }
    prompt("  Enter value: ")
}

fn collect_languages() -> IndexMap<String, u8> {
    let mut languages = IndexMap::new();
    println!("\n  Enter languages (name and 0/1, blank name to finish):");
    loop {
        let name = prompt("    Language name (e.g. bangla, english): ");
        if name.is_empty() {
            break;
        }
        let value = prompt_binary(&name, "Is this language supported?");
        languages.insert(name, value);
    }
    languages
}
