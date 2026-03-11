# OSDAT — Open Data Portal Quality Assessment Tool

A Rust command-line application that crawls government website URLs, discovers downloadable data files (PDF, XLSX, CSV, XML, DOCX, etc.), extracts tabular dataset metadata, supplements extraction with local LLM-based analysis (via Ollama), interactively collects human input for fields that cannot be auto-detected, and outputs a structured JSON quality-assessment report.

## Features

- **Web Crawling**: Fetches HTML pages and discovers downloadable data file links (.pdf, .xlsx, .xls, .csv, .xml, .docx, .doc, .pptx, .ppt, .txt)
- **File Download**: Downloads discovered files asynchronously
- **Data Extraction**: Extracts metadata (row counts, column names, headers) from CSV, Excel, XML, and text files
- **LLM-Assisted Analysis**: Uses Ollama (`qwen3:2b` model) to analyze website features and generate dataset descriptions
- **Interactive CLI**: Prompts for platform-level fields with AI-assisted defaults
- **Structured JSON Output**: Generates a quality-assessment report conforming to the `sparrso.json` schema

## Architecture

The application consists of six sequential pipeline stages:

```
[CLI Input] → [Web Crawling] → [File Download] → [Data Extraction] → [LLM-Assisted Analysis] → [JSON Generation]
```

### Modules

| Module | Description |
|---|---|
| `cli` | Interactive prompts and user input collection |
| `crawler` | HTML fetching, link parsing, and data file discovery |
| `downloader` | Async file downloading with deduplication |
| `extractor` | Metadata extraction from CSV, Excel, XML, and text files |
| `llm` | Ollama integration for AI-assisted website analysis |
| `output` | JSON report generation and file writing |
| `models` | Data structures for the entire pipeline |

## Prerequisites

- **Rust** 1.75+ (2024 edition)
- **Ollama** (optional, for LLM-assisted analysis) — install from [ollama.com](https://ollama.com)
  - Pull the model: `ollama pull qwen3:2b`

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run --release
```

The tool will interactively prompt for:

1. **Root website URL** (e.g., `https://sparrso.gov.bd/`)
2. **Page URLs to crawl** (one per line, blank line to finish)
3. **Output JSON filename** (e.g., `sparrso.json`)
4. **Category name** (e.g., `গবেষণা`)
5. **Platform-level fields** (with AI-detected defaults when Ollama is available)

## Example Output

```json
{
  "website": {
    "url": "https://sparrso.gov.bd/"
  },
  "platform-level": {
    "user-accessibility": {
      "necessity-of-login": 0,
      "multiple-language-support": 1,
      "request-for-datasets": 0,
      "languages": {
        "bangla": 1,
        "english": 1
      }
    },
    "user-usability": {
      "browse-data-sets-by-category": 1,
      "filter-and/or-sort-datasets": 0,
      "search-for-dataset": 1,
      "user-guideline": 0
    },
    "diversity": {
      "number-of-dataset": 3,
      "number-of-category": 2
    }
  },
  "categories": [
    {
      "name": "গবেষণা",
      "datasets": [...]
    }
  ]
}
```

## Testing

```bash
cargo test
```

## License

MIT
