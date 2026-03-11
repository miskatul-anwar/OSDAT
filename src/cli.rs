use std::collections::HashMap;
use std::io::{self, BufRead, Write};

use crate::models::AppConfig;

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

fn collect_languages() -> HashMap<String, u8> {
    let mut languages = HashMap::new();
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
