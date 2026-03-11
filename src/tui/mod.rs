pub mod widgets;

use std::io;
use std::path::PathBuf;

use color_eyre::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Tabs, Wrap},
    Terminal,
};

use crate::models::*;
use widgets::{
    BinaryFieldState, ProgressItem, ProgressList, ProgressListState, ProgressStatus,
    TextInput, TextInputState,
};

// ──────────────────────────────────────────────
// App State Machine
// ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum AppScreen {
    Welcome,
    PlatformAnalysis,
    CrawlProgress,
    DownloadProgress,
    DatasetEditor,
    ReviewAndExport,
}

// ──────────────────────────────────────────────
// App struct
// ──────────────────────────────────────────────

pub struct App {
    pub screen: AppScreen,
    pub should_quit: bool,

    // Welcome screen state
    pub root_url: TextInputState,
    pub page_urls: Vec<String>,
    pub current_page_url: TextInputState,
    pub output_filename: TextInputState,
    pub category_name: TextInputState,
    pub welcome_focus: usize, // 0=root_url, 1=page_url, 2=output, 3=category, 4=submit

    // Platform analysis state
    pub platform_fields: Vec<PlatformFieldEntry>,
    pub platform_focus: usize,

    // Crawl progress state
    pub crawl_progress: ProgressListState,
    pub crawl_total: usize,
    pub crawl_done: usize,

    // Download progress state
    pub download_progress: ProgressListState,
    pub download_total: usize,
    pub download_done: usize,

    // Dataset editor state
    pub datasets: Vec<DatasetEditorEntry>,
    pub current_dataset: usize,
    pub dataset_field_focus: usize,

    // Review state
    pub json_preview: String,
    pub json_scroll: u16,

    // Final report
    pub report: Option<QualityReport>,
    pub output_path: Option<PathBuf>,
}

pub struct PlatformFieldEntry {
    pub name: String,
    pub description: String,
    pub ai_suggestion: Option<String>,
    pub value: FieldValue,
}

pub enum FieldValue {
    Binary(BinaryFieldState),
    Text(TextInputState),
    Number(u32),
}

pub struct DatasetEditorEntry {
    pub name: String,
    pub url: String,
    pub fields: Vec<DatasetFieldEntry>,
    pub data_summary: String,
}

pub struct DatasetFieldEntry {
    pub section: String,
    pub name: String,
    pub description: String,
    pub value: FieldValue,
    pub ai_suggestion: Option<String>,
}

// ──────────────────────────────────────────────
// Color theme constants
// ──────────────────────────────────────────────

const HEADER_STYLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
const AI_VALUE_STYLE: Style = Style::new().fg(Color::Green);
const USER_REQUIRED_STYLE: Style = Style::new().fg(Color::Yellow);
const ERROR_STYLE: Style = Style::new().fg(Color::Red);
const BORDER_STYLE: Style = Style::new().fg(Color::DarkGray);
const MIN_WIDTH: u16 = 80;
const MIN_HEIGHT: u16 = 24;

impl App {
    pub fn new() -> Self {
        Self {
            screen: AppScreen::Welcome,
            should_quit: false,
            root_url: TextInputState::new(String::new()),
            page_urls: Vec::new(),
            current_page_url: TextInputState::new(String::new()),
            output_filename: TextInputState::new("output.json".to_string()),
            category_name: TextInputState::new(String::new()),
            welcome_focus: 0,
            platform_fields: Vec::new(),
            platform_focus: 0,
            crawl_progress: ProgressListState::new(),
            crawl_total: 0,
            crawl_done: 0,
            download_progress: ProgressListState::new(),
            download_total: 0,
            download_done: 0,
            datasets: Vec::new(),
            current_dataset: 0,
            dataset_field_focus: 0,
            json_preview: String::new(),
            json_scroll: 0,
            report: None,
            output_path: None,
        }
    }

    /// Get the AppConfig from the welcome screen state.
    pub fn get_config(&self) -> AppConfig {
        AppConfig {
            root_url: self.root_url.value.clone(),
            page_urls: self.page_urls.clone(),
            output_filename: self.output_filename.value.clone(),
            category_name: self.category_name.value.clone(),
        }
    }
}

// ──────────────────────────────────────────────
// Terminal setup and teardown
// ──────────────────────────────────────────────

pub fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

pub fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

// ──────────────────────────────────────────────
// Drawing
// ──────────────────────────────────────────────

pub fn draw(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    terminal.draw(|frame| {
        let size = frame.area();

        // Check minimum terminal size
        if size.width < MIN_WIDTH || size.height < MIN_HEIGHT {
            let msg = Paragraph::new(format!(
                "Terminal too small ({} x {}). Minimum: {} x {}",
                size.width, size.height, MIN_WIDTH, MIN_HEIGHT
            ))
            .style(ERROR_STYLE);
            frame.render_widget(msg, size);
            return;
        }

        match app.screen {
            AppScreen::Welcome => draw_welcome(frame, app, size),
            AppScreen::PlatformAnalysis => draw_platform_analysis(frame, app, size),
            AppScreen::CrawlProgress => draw_crawl_progress(frame, app, size),
            AppScreen::DownloadProgress => draw_download_progress(frame, app, size),
            AppScreen::DatasetEditor => draw_dataset_editor(frame, app, size),
            AppScreen::ReviewAndExport => draw_review(frame, app, size),
        }
    })?;
    Ok(())
}

fn draw_welcome(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Root URL
            Constraint::Length(3), // Page URL input
            Constraint::Min(5),   // URL list
            Constraint::Length(3), // Output filename
            Constraint::Length(3), // Category name
            Constraint::Length(3), // Submit button
        ])
        .split(area);

    // Title
    let title = Paragraph::new(Line::from(vec![
        Span::styled("OSDAT ", HEADER_STYLE),
        Span::raw("— Open Data Portal Quality Assessment Tool"),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).border_style(BORDER_STYLE));
    frame.render_widget(title, chunks[0]);

    // Root URL input
    frame.render_stateful_widget(
        TextInput::new("Root URL (e.g. https://sparrso.gov.bd/)", app.welcome_focus == 0),
        chunks[1],
        &mut app.root_url,
    );

    // Page URL input
    frame.render_stateful_widget(
        TextInput::new(
            "Add Page URL (Enter to add, blank to skip)",
            app.welcome_focus == 1,
        ),
        chunks[2],
        &mut app.current_page_url,
    );

    // URL list
    let urls: Vec<ListItem> = app
        .page_urls
        .iter()
        .enumerate()
        .map(|(i, url)| {
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {}. ", i + 1), Style::default().fg(Color::DarkGray)),
                Span::raw(url),
            ]))
        })
        .collect();
    let url_list = List::new(urls).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(BORDER_STYLE)
            .title("Page URLs to crawl"),
    );
    frame.render_widget(url_list, chunks[3]);

    // Output filename
    frame.render_stateful_widget(
        TextInput::new("Output JSON filename", app.welcome_focus == 2),
        chunks[4],
        &mut app.output_filename,
    );

    // Category name
    frame.render_stateful_widget(
        TextInput::new("Category name (e.g. গবেষণা)", app.welcome_focus == 3),
        chunks[5],
        &mut app.category_name,
    );

    // Submit
    let submit_style = if app.welcome_focus == 4 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };
    let submit = Paragraph::new("  [ Start Assessment → ]")
        .style(submit_style)
        .block(Block::default().borders(Borders::ALL).border_style(BORDER_STYLE));
    frame.render_widget(submit, chunks[6]);
}

fn draw_platform_analysis(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),  // Field table
            Constraint::Length(3), // Footer
        ])
        .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled("Platform-Level Analysis ", HEADER_STYLE),
        Span::raw("— ↑↓ navigate, 0/1 toggle, Enter to edit"),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).border_style(BORDER_STYLE));
    frame.render_widget(header, chunks[0]);

    // Fields
    let items: Vec<ListItem> = app
        .platform_fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let focused = i == app.platform_focus;
            let value_str = match &field.value {
                FieldValue::Binary(b) => format!("[{}]", b.value),
                FieldValue::Text(t) => t.value.clone(),
                FieldValue::Number(n) => n.to_string(),
            };

            let ai_str = field
                .ai_suggestion
                .as_deref()
                .map(|s| format!(" (AI: {s})"))
                .unwrap_or_default();

            let style = if focused {
                USER_REQUIRED_STYLE.add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(if focused { "▸ ".to_string() } else { "  ".to_string() }, style),
                Span::styled(field.name.clone(), style),
                Span::raw(": "),
                Span::styled(value_str, AI_VALUE_STYLE),
                Span::styled(ai_str, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(BORDER_STYLE)
            .title("Fields"),
    );
    frame.render_widget(list, chunks[1]);

    // Footer
    let footer = Paragraph::new("  Tab: Next field | Enter: Confirm & proceed")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[2]);
}

fn draw_crawl_progress(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(3), // Progress bar
            Constraint::Min(10),  // URL list
        ])
        .split(area);

    let header = Paragraph::new(Line::from(Span::styled(
        "Crawling Pages for Data Files...",
        HEADER_STYLE,
    )))
    .block(Block::default().borders(Borders::BOTTOM).border_style(BORDER_STYLE));
    frame.render_widget(header, chunks[0]);

    // Progress gauge
    let progress = if app.crawl_total > 0 {
        app.crawl_done as f64 / app.crawl_total as f64
    } else {
        0.0
    };
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).border_style(BORDER_STYLE))
        .gauge_style(Style::default().fg(Color::Cyan))
        .ratio(progress)
        .label(format!("{}/{}", app.crawl_done, app.crawl_total));
    frame.render_widget(gauge, chunks[1]);

    // URL list
    frame.render_stateful_widget(
        ProgressList::new("URLs"),
        chunks[2],
        &mut app.crawl_progress,
    );
}

fn draw_download_progress(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(3), // Progress bar
            Constraint::Min(10),  // File list
        ])
        .split(area);

    let header = Paragraph::new(Line::from(Span::styled(
        "Downloading Discovered Files...",
        HEADER_STYLE,
    )))
    .block(Block::default().borders(Borders::BOTTOM).border_style(BORDER_STYLE));
    frame.render_widget(header, chunks[0]);

    let progress = if app.download_total > 0 {
        app.download_done as f64 / app.download_total as f64
    } else {
        0.0
    };
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).border_style(BORDER_STYLE))
        .gauge_style(Style::default().fg(Color::Green))
        .ratio(progress)
        .label(format!("{}/{}", app.download_done, app.download_total));
    frame.render_widget(gauge, chunks[1]);

    frame.render_stateful_widget(
        ProgressList::new("Files"),
        chunks[2],
        &mut app.download_progress,
    );
}

fn draw_dataset_editor(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    if app.datasets.is_empty() {
        let msg = Paragraph::new("No datasets to edit.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Tabs
            Constraint::Min(10),  // Editor
            Constraint::Length(3), // Footer
        ])
        .split(area);

    // Dataset tabs
    let tab_titles: Vec<Line> = app
        .datasets
        .iter()
        .enumerate()
        .map(|(_i, d)| {
            Line::from(format!(
                " {} ",
                if d.name.len() > 20 {
                    &d.name[..20]
                } else {
                    &d.name
                }
            ))
        })
        .collect();

    let tabs = Tabs::new(tab_titles)
        .select(app.current_dataset)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(HEADER_STYLE)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(BORDER_STYLE)
                .title("Datasets"),
        );
    frame.render_widget(tabs, chunks[0]);

    // Current dataset fields
    if let Some(dataset) = app.datasets.get(app.current_dataset) {
        let items: Vec<ListItem> = dataset
            .fields
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let focused = i == app.dataset_field_focus;
                let value_str = match &field.value {
                    FieldValue::Binary(b) => format!("[{}]", b.value),
                    FieldValue::Text(t) => t.value.clone(),
                    FieldValue::Number(n) => n.to_string(),
                };

                let ai_str = field
                    .ai_suggestion
                    .as_deref()
                    .map(|s| format!(" (AI: {s})"))
                    .unwrap_or_default();

                let section_marker = if i == 0
                    || dataset
                        .fields
                        .get(i.wrapping_sub(1))
                        .map(|prev| prev.section != field.section)
                        .unwrap_or(true)
                {
                    format!("── {} ──\n", field.section)
                } else {
                    String::new()
                };

                let style = if focused {
                    USER_REQUIRED_STYLE.add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(vec![
                    if !section_marker.is_empty() {
                        Line::from(Span::styled(
                            section_marker,
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ))
                    } else {
                        Line::from("")
                    },
                    Line::from(vec![
                        Span::styled(if focused { "▸ " } else { "  " }, style),
                        Span::styled(&field.name, style),
                        Span::raw(": "),
                        Span::styled(value_str, AI_VALUE_STYLE),
                        Span::styled(ai_str, Style::default().fg(Color::DarkGray)),
                    ]),
                ])
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(BORDER_STYLE)
                .title(format!("Fields — {}", dataset.data_summary)),
        );
        frame.render_widget(list, chunks[1]);
    }

    // Footer
    let footer = Paragraph::new(
        "  ↑↓: Navigate | 0/1: Toggle | Enter: Edit | Tab/Shift+Tab: Dataset | PgDn: Next section | Ctrl+S: Save",
    )
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[2]);
}

fn draw_review(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),  // JSON preview
            Constraint::Length(3), // Footer
        ])
        .split(area);

    let header = Paragraph::new(Line::from(Span::styled(
        "Review & Export",
        HEADER_STYLE,
    )))
    .block(Block::default().borders(Borders::BOTTOM).border_style(BORDER_STYLE));
    frame.render_widget(header, chunks[0]);

    // JSON preview with syntax highlighting
    let lines: Vec<Line> = app
        .json_preview
        .lines()
        .skip(app.json_scroll as usize)
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('"') && trimmed.contains(':') {
                // Key-value line
                Line::from(Span::styled(line, Style::default().fg(Color::Cyan)))
            } else if trimmed.starts_with('{') || trimmed.starts_with('}') || trimmed.starts_with('[') || trimmed.starts_with(']') {
                Line::from(Span::styled(line, Style::default().fg(Color::DarkGray)))
            } else {
                Line::from(Span::styled(line, Style::default().fg(Color::Green)))
            }
        })
        .collect();

    let json_view = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(BORDER_STYLE)
                .title("JSON Preview"),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(json_view, chunks[1]);

    let footer = Paragraph::new(
        "  ↑↓: Scroll | Enter: Save | e: Edit | Esc: Cancel",
    )
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[2]);
}

// ──────────────────────────────────────────────
// Input handling
// ──────────────────────────────────────────────

pub fn handle_key(app: &mut App, key: event::KeyEvent) -> bool {
    // Global quit: Ctrl+C or Ctrl+Q
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && (key.code == KeyCode::Char('c') || key.code == KeyCode::Char('q'))
    {
        app.should_quit = true;
        return true;
    }

    match app.screen {
        AppScreen::Welcome => handle_welcome_key(app, key),
        AppScreen::PlatformAnalysis => handle_platform_key(app, key),
        AppScreen::CrawlProgress => false, // Auto-advancing
        AppScreen::DownloadProgress => false,
        AppScreen::DatasetEditor => handle_dataset_editor_key(app, key),
        AppScreen::ReviewAndExport => handle_review_key(app, key),
    }
}

fn handle_welcome_key(app: &mut App, key: event::KeyEvent) -> bool {
    match key.code {
        KeyCode::Tab => {
            app.welcome_focus = (app.welcome_focus + 1) % 5;
            true
        }
        KeyCode::BackTab => {
            app.welcome_focus = if app.welcome_focus == 0 {
                4
            } else {
                app.welcome_focus - 1
            };
            true
        }
        KeyCode::Enter => {
            match app.welcome_focus {
                1 => {
                    // Add page URL
                    if !app.current_page_url.value.is_empty() {
                        app.page_urls.push(app.current_page_url.value.clone());
                        app.current_page_url = TextInputState::new(String::new());
                    }
                }
                4 => {
                    // Submit - advance to next screen
                    if !app.root_url.value.is_empty() {
                        return true; // Signal to advance screen
                    }
                }
                _ => {
                    app.welcome_focus = (app.welcome_focus + 1) % 5;
                }
            }
            true
        }
        KeyCode::Char(c) => {
            match app.welcome_focus {
                0 => app.root_url.insert(c),
                1 => app.current_page_url.insert(c),
                2 => app.output_filename.insert(c),
                3 => app.category_name.insert(c),
                _ => {}
            }
            true
        }
        KeyCode::Backspace => {
            match app.welcome_focus {
                0 => app.root_url.delete_back(),
                1 => app.current_page_url.delete_back(),
                2 => app.output_filename.delete_back(),
                3 => app.category_name.delete_back(),
                _ => {}
            }
            true
        }
        KeyCode::Left => {
            match app.welcome_focus {
                0 => app.root_url.move_left(),
                1 => app.current_page_url.move_left(),
                2 => app.output_filename.move_left(),
                3 => app.category_name.move_left(),
                _ => {}
            }
            true
        }
        KeyCode::Right => {
            match app.welcome_focus {
                0 => app.root_url.move_right(),
                1 => app.current_page_url.move_right(),
                2 => app.output_filename.move_right(),
                3 => app.category_name.move_right(),
                _ => {}
            }
            true
        }
        KeyCode::Delete => {
            // Delete current URL from list if focus is on URL list area
            if app.welcome_focus == 1 && app.current_page_url.value.is_empty() {
                app.page_urls.pop();
            }
            true
        }
        _ => false,
    }
}

fn handle_platform_key(app: &mut App, key: event::KeyEvent) -> bool {
    if app.platform_fields.is_empty() {
        return false;
    }

    match key.code {
        KeyCode::Up => {
            if app.platform_focus > 0 {
                app.platform_focus -= 1;
            }
            true
        }
        KeyCode::Down => {
            if app.platform_focus < app.platform_fields.len() - 1 {
                app.platform_focus += 1;
            }
            true
        }
        KeyCode::Char('0') | KeyCode::Char('1') => {
            if let Some(field) = app.platform_fields.get_mut(app.platform_focus) {
                if let FieldValue::Binary(ref mut b) = field.value {
                    b.value = if key.code == KeyCode::Char('0') { 0 } else { 1 };
                }
            }
            true
        }
        KeyCode::Enter => {
            // Advance to next screen
            true
        }
        KeyCode::Tab => {
            if app.platform_focus < app.platform_fields.len() - 1 {
                app.platform_focus += 1;
            }
            true
        }
        _ => false,
    }
}

fn handle_dataset_editor_key(app: &mut App, key: event::KeyEvent) -> bool {
    if app.datasets.is_empty() {
        return false;
    }

    // Ctrl+S to save/proceed
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
        app.screen = AppScreen::ReviewAndExport;
        return true;
    }

    match key.code {
        KeyCode::Up => {
            if app.dataset_field_focus > 0 {
                app.dataset_field_focus -= 1;
            }
            true
        }
        KeyCode::Down => {
            if let Some(dataset) = app.datasets.get(app.current_dataset) {
                if app.dataset_field_focus < dataset.fields.len().saturating_sub(1) {
                    app.dataset_field_focus += 1;
                }
            }
            true
        }
        KeyCode::Tab => {
            // Next dataset
            if app.current_dataset < app.datasets.len() - 1 {
                app.current_dataset += 1;
                app.dataset_field_focus = 0;
            }
            true
        }
        KeyCode::BackTab => {
            // Previous dataset
            if app.current_dataset > 0 {
                app.current_dataset -= 1;
                app.dataset_field_focus = 0;
            }
            true
        }
        KeyCode::Char('0') | KeyCode::Char('1') => {
            if let Some(dataset) = app.datasets.get_mut(app.current_dataset) {
                if let Some(field) = dataset.fields.get_mut(app.dataset_field_focus) {
                    if let FieldValue::Binary(ref mut b) = field.value {
                        b.value = if key.code == KeyCode::Char('0') { 0 } else { 1 };
                    }
                }
            }
            true
        }
        KeyCode::PageDown => {
            // Jump to next section
            if let Some(dataset) = app.datasets.get(app.current_dataset) {
                if let Some(current_field) = dataset.fields.get(app.dataset_field_focus) {
                    let current_section = current_field.section.clone();
                    for (i, field) in dataset.fields.iter().enumerate().skip(app.dataset_field_focus + 1) {
                        if field.section != current_section {
                            app.dataset_field_focus = i;
                            break;
                        }
                    }
                }
            }
            true
        }
        KeyCode::PageUp => {
            // Jump to previous section
            if let Some(dataset) = app.datasets.get(app.current_dataset) {
                if let Some(current_field) = dataset.fields.get(app.dataset_field_focus) {
                    let current_section = current_field.section.clone();
                    for i in (0..app.dataset_field_focus).rev() {
                        if let Some(field) = dataset.fields.get(i) {
                            if field.section != current_section {
                                app.dataset_field_focus = i;
                                break;
                            }
                        }
                    }
                }
            }
            true
        }
        KeyCode::Enter => {
            // Inline edit for text/number fields
            true
        }
        _ => false,
    }
}

fn handle_review_key(app: &mut App, key: event::KeyEvent) -> bool {
    match key.code {
        KeyCode::Up => {
            if app.json_scroll > 0 {
                app.json_scroll -= 1;
            }
            true
        }
        KeyCode::Down => {
            app.json_scroll += 1;
            true
        }
        KeyCode::Enter => {
            // Save
            true
        }
        KeyCode::Char('e') => {
            app.screen = AppScreen::DatasetEditor;
            true
        }
        KeyCode::Esc => {
            app.should_quit = true;
            true
        }
        _ => false,
    }
}

// ──────────────────────────────────────────────
// Main TUI entry point
// ──────────────────────────────────────────────

/// Run the TUI application. Returns the completed QualityReport on success.
pub async fn run_tui() -> Result<Option<QualityReport>> {
    color_eyre::install()?;
    let mut terminal = setup_terminal()?;
    let mut app = App::new();

    let result = run_tui_loop(&mut terminal, &mut app).await;

    restore_terminal(&mut terminal)?;
    result
}

async fn run_tui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<Option<QualityReport>> {
    let client = reqwest::Client::builder()
        .user_agent("OSDAT/0.1.0 (Open Data Portal Quality Assessment Tool)")
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    loop {
        draw(terminal, app)?;

        if app.should_quit {
            return Ok(app.report.clone());
        }

        // Poll for events with a timeout so async tasks can progress
        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                let should_advance = handle_key(app, key);

                // Handle screen transitions
                if should_advance && app.screen == AppScreen::Welcome && key.code == KeyCode::Enter && app.welcome_focus == 4 {
                    if !app.root_url.value.is_empty() {
                        // Transition to platform analysis
                        app.screen = AppScreen::PlatformAnalysis;

                        // Fetch HTML and analyze
                        let config = app.get_config();
                        let root_html = match crate::crawler::fetch_page_html(&config.root_url, &client).await {
                            Ok(html) => html,
                            Err(_) => String::new(),
                        };

                        let llm_analysis = crate::llm::analyze_website(&root_html, &client).await;

                        // Populate platform fields
                        app.platform_fields = build_platform_fields(&llm_analysis);
                    }
                }

                if should_advance && app.screen == AppScreen::PlatformAnalysis && key.code == KeyCode::Enter {
                    // Run crawl, download, extract pipeline
                    let config = app.get_config();

                    // Set up crawl progress
                    app.screen = AppScreen::CrawlProgress;
                    app.crawl_total = config.page_urls.len();
                    app.crawl_progress.items = config
                        .page_urls
                        .iter()
                        .map(|url| ProgressItem {
                            label: url.clone(),
                            status: ProgressStatus::Pending,
                            detail: String::new(),
                        })
                        .collect();

                    draw(terminal, app)?;

                    // Run crawl
                    let crawl_results = crate::crawler::crawl_pages(&config.page_urls, &client).await;
                    match crawl_results {
                        Ok(results) => {
                            for (i, item) in app.crawl_progress.items.iter_mut().enumerate() {
                                item.status = ProgressStatus::Done;
                                if let Some(files) = results.get(&config.page_urls[i]) {
                                    item.detail = format!("{} files", files.len());
                                }
                            }
                            app.crawl_done = app.crawl_total;

                            // Flatten files
                            let all_files: Vec<DiscoveredFile> = results
                                .values()
                                .flat_map(|f| f.iter().cloned())
                                .collect();

                            draw(terminal, app)?;

                            // Download
                            app.screen = AppScreen::DownloadProgress;
                            app.download_total = all_files.len();
                            app.download_progress.items = all_files
                                .iter()
                                .map(|f| ProgressItem {
                                    label: f.download_url.clone(),
                                    status: ProgressStatus::Pending,
                                    detail: String::new(),
                                })
                                .collect();

                            draw(terminal, app)?;

                            let download_dir = std::path::PathBuf::from("osdat_downloads");
                            tokio::fs::create_dir_all(&download_dir).await?;

                            let downloaded = crate::downloader::download_all_files(
                                &all_files,
                                &download_dir,
                                &client,
                            )
                            .await;

                            app.download_done = downloaded.len();
                            for (i, item) in app.download_progress.items.iter_mut().enumerate() {
                                if i < downloaded.len() {
                                    item.status = ProgressStatus::Done;
                                } else {
                                    item.status = ProgressStatus::Failed;
                                }
                            }

                            draw(terminal, app)?;

                            // Extract metadata and set up dataset editor
                            let mut page_htmls = std::collections::HashMap::new();
                            for page_url in &config.page_urls {
                                if let Ok(html) = crate::crawler::fetch_page_html(page_url, &client).await {
                                    page_htmls.insert(page_url.clone(), html);
                                }
                            }

                            let mut all_extracted = Vec::new();
                            for (file_info, local_path, file_size) in &downloaded {
                                let mut datasets = crate::extractor::extract_metadata(
                                    local_path,
                                    &file_info.download_url,
                                    &file_info.source_page_url,
                                    &file_info.file_extension,
                                    *file_size,
                                );
                                for data in &mut datasets {
                                    if let Some(html) = page_htmls.get(&file_info.source_page_url) {
                                        if let Some(name) = crate::llm::extract_dataset_name_from_html(html, &file_info.download_url) {
                                            data.title = name;
                                        }
                                    }
                                }
                                for data in datasets {
                                    all_extracted.push((data, local_path.clone()));
                                }
                            }

                            // Build dataset editor entries
                            let site_name = url::Url::parse(&config.root_url)
                                .map(|u| u.host_str().unwrap_or("Unknown").to_string())
                                .unwrap_or_else(|_| "Unknown".to_string());

                            for (data, local_path) in &all_extracted {
                                let rag = crate::llm::analyze_dataset_with_rag(
                                    local_path,
                                    &data.file_type,
                                    &data.column_names,
                                    &client,
                                )
                                .await;

                                let dataset_name = rag
                                    .dataset_name
                                    .clone()
                                    .filter(|n| !n.is_empty())
                                    .unwrap_or_else(|| data.title.clone());

                                let auto = auto_detect_dataset_level(data);
                                let auto_data = auto_detect_data_level(data);

                                let entry = build_dataset_editor_entry(
                                    &dataset_name,
                                    &data.source_url,
                                    &auto,
                                    &auto_data,
                                    &rag,
                                    &site_name,
                                );
                                app.datasets.push(entry);
                            }

                            // Cleanup
                            if download_dir.exists() {
                                tokio::fs::remove_dir_all(&download_dir).await.ok();
                            }

                            app.screen = AppScreen::DatasetEditor;
                        }
                        Err(_) => {
                            for item in &mut app.crawl_progress.items {
                                item.status = ProgressStatus::Failed;
                            }
                        }
                    }
                }

                if should_advance && app.screen == AppScreen::ReviewAndExport && key.code == KeyCode::Enter {
                    // Save and exit
                    if let Some(ref report) = app.report {
                        let output_path = app.output_path.clone().unwrap_or_else(|| std::path::PathBuf::from("output.json"));
                        crate::output::write_report(report, &output_path).ok();
                    }
                    app.should_quit = true;
                }
            }
        }
    }
}

// ──────────────────────────────────────────────
// Helper builders
// ──────────────────────────────────────────────

fn build_platform_fields(llm: &LlmAnalysis) -> Vec<PlatformFieldEntry> {
    vec![
        PlatformFieldEntry {
            name: "necessity-of-login".to_string(),
            description: "Does the site require login to access data?".to_string(),
            ai_suggestion: llm.necessity_of_login.map(|v| v.to_string()),
            value: FieldValue::Binary(BinaryFieldState::new(
                llm.necessity_of_login.unwrap_or(0),
            )),
        },
        PlatformFieldEntry {
            name: "multiple-language-support".to_string(),
            description: "Does the site offer multiple languages?".to_string(),
            ai_suggestion: llm.multiple_language_support.map(|v| v.to_string()),
            value: FieldValue::Binary(BinaryFieldState::new(
                llm.multiple_language_support.unwrap_or(0),
            )),
        },
        PlatformFieldEntry {
            name: "request-for-datasets".to_string(),
            description: "Is there a mechanism to request new datasets?".to_string(),
            ai_suggestion: llm.request_for_datasets.map(|v| v.to_string()),
            value: FieldValue::Binary(BinaryFieldState::new(
                llm.request_for_datasets.unwrap_or(0),
            )),
        },
        PlatformFieldEntry {
            name: "browse-data-sets-by-category".to_string(),
            description: "Can users browse datasets by category?".to_string(),
            ai_suggestion: llm.browse_datasets_by_category.map(|v| v.to_string()),
            value: FieldValue::Binary(BinaryFieldState::new(
                llm.browse_datasets_by_category.unwrap_or(0),
            )),
        },
        PlatformFieldEntry {
            name: "filter-and/or-sort-datasets".to_string(),
            description: "Are filter/sort options available?".to_string(),
            ai_suggestion: llm.filter_sort_datasets.map(|v| v.to_string()),
            value: FieldValue::Binary(BinaryFieldState::new(
                llm.filter_sort_datasets.unwrap_or(0),
            )),
        },
        PlatformFieldEntry {
            name: "search-for-dataset".to_string(),
            description: "Is there a search feature?".to_string(),
            ai_suggestion: llm.search_for_dataset.map(|v| v.to_string()),
            value: FieldValue::Binary(BinaryFieldState::new(
                llm.search_for_dataset.unwrap_or(0),
            )),
        },
        PlatformFieldEntry {
            name: "user-guideline".to_string(),
            description: "Are usage guidelines provided?".to_string(),
            ai_suggestion: llm.user_guideline.map(|v| v.to_string()),
            value: FieldValue::Binary(BinaryFieldState::new(
                llm.user_guideline.unwrap_or(0),
            )),
        },
        PlatformFieldEntry {
            name: "number-of-category".to_string(),
            description: "Total number of dataset categories".to_string(),
            ai_suggestion: llm.number_of_category.map(|v| v.to_string()),
            value: FieldValue::Number(llm.number_of_category.unwrap_or(1)),
        },
    ]
}

fn build_dataset_editor_entry(
    name: &str,
    url: &str,
    auto: &DatasetLevel,
    auto_data: &DataLevel,
    rag: &DatasetRagAnalysis,
    _site_name: &str,
) -> DatasetEditorEntry {
    let data_summary = format!(
        "{}×{}, {}, {} empty",
        auto_data.data_volume.number_of_rows,
        auto_data.data_volume.number_of_columns,
        auto_data.data_volume.file_size,
        auto_data.data_level_completeness.number_of_empty_cells,
    );

    let mut fields = Vec::new();

    // Openness section
    fields.push(DatasetFieldEntry {
        section: "openness".to_string(),
        name: "complete.descriptive".to_string(),
        description: "Is the dataset descriptive?".to_string(),
        value: FieldValue::Binary(BinaryFieldState::new(auto.openness.complete.descriptive)),
        ai_suggestion: None,
    });
    fields.push(DatasetFieldEntry {
        section: "openness".to_string(),
        name: "complete.linked-data".to_string(),
        description: "Is linked data available?".to_string(),
        value: FieldValue::Binary(BinaryFieldState::new(auto.openness.complete.linked_data)),
        ai_suggestion: None,
    });
    fields.push(DatasetFieldEntry {
        section: "openness".to_string(),
        name: "primary".to_string(),
        description: "Is this primary/original data?".to_string(),
        value: FieldValue::Binary(BinaryFieldState::new(auto.openness.primary)),
        ai_suggestion: None,
    });
    fields.push(DatasetFieldEntry {
        section: "openness".to_string(),
        name: "non-discriminatory".to_string(),
        description: "Is data available without discrimination?".to_string(),
        value: FieldValue::Binary(BinaryFieldState::new(auto.openness.non_discriminatory)),
        ai_suggestion: None,
    });
    fields.push(DatasetFieldEntry {
        section: "openness".to_string(),
        name: "accessible".to_string(),
        description: "Is data publicly accessible?".to_string(),
        value: FieldValue::Binary(BinaryFieldState::new(auto.openness.accessible)),
        ai_suggestion: None,
    });
    fields.push(DatasetFieldEntry {
        section: "openness".to_string(),
        name: "timely".to_string(),
        description: "Is data published timely?".to_string(),
        value: FieldValue::Binary(BinaryFieldState::new(auto.openness.timely)),
        ai_suggestion: None,
    });
    fields.push(DatasetFieldEntry {
        section: "openness".to_string(),
        name: "license-free".to_string(),
        description: "Is the data license-free?".to_string(),
        value: FieldValue::Binary(BinaryFieldState::new(auto.openness.license_free)),
        ai_suggestion: None,
    });

    // Transparency section
    fields.push(DatasetFieldEntry {
        section: "transparency".to_string(),
        name: "understandability.faq".to_string(),
        description: "Is there a FAQ?".to_string(),
        value: FieldValue::Binary(BinaryFieldState::new(auto.transparency.understandability.faq)),
        ai_suggestion: None,
    });
    fields.push(DatasetFieldEntry {
        section: "transparency".to_string(),
        name: "understandability.textual-description".to_string(),
        description: "Is there a textual description?".to_string(),
        value: FieldValue::Binary(BinaryFieldState::new(auto.transparency.understandability.textual_description)),
        ai_suggestion: None,
    });
    fields.push(DatasetFieldEntry {
        section: "transparency".to_string(),
        name: "meta-data".to_string(),
        description: "Is metadata provided?".to_string(),
        value: FieldValue::Binary(BinaryFieldState::new(auto.transparency.meta_data)),
        ai_suggestion: None,
    });

    // Provenance section
    fields.push(DatasetFieldEntry {
        section: "provenance".to_string(),
        name: "time-period".to_string(),
        description: "Time period covered".to_string(),
        value: FieldValue::Text(TextInputState::new(
            rag.time_period.clone().unwrap_or_default(),
        )),
        ai_suggestion: rag.time_period.clone(),
    });
    fields.push(DatasetFieldEntry {
        section: "provenance".to_string(),
        name: "update-activity".to_string(),
        description: "Update frequency".to_string(),
        value: FieldValue::Text(TextInputState::new(
            rag.update_activity.clone().unwrap_or_default(),
        )),
        ai_suggestion: rag.update_activity.clone(),
    });
    fields.push(DatasetFieldEntry {
        section: "provenance".to_string(),
        name: "last-update".to_string(),
        description: "Last update date".to_string(),
        value: FieldValue::Text(TextInputState::new(
            rag.last_update.clone().unwrap_or_default(),
        )),
        ai_suggestion: rag.last_update.clone(),
    });
    fields.push(DatasetFieldEntry {
        section: "provenance".to_string(),
        name: "collection-method".to_string(),
        description: "Data collection method".to_string(),
        value: FieldValue::Text(TextInputState::new(
            rag.collection_method.clone().unwrap_or_default(),
        )),
        ai_suggestion: rag.collection_method.clone(),
    });

    // Semantic consistency
    fields.push(DatasetFieldEntry {
        section: "semantic-consistency".to_string(),
        name: "external-vocabulary".to_string(),
        description: "Uses external vocabulary?".to_string(),
        value: FieldValue::Binary(BinaryFieldState::new(auto.semantic_consistency.external_vocabulary)),
        ai_suggestion: None,
    });

    // Granularity
    fields.push(DatasetFieldEntry {
        section: "granularity".to_string(),
        name: "time-dimension.day".to_string(),
        description: "Day-level time granularity".to_string(),
        value: FieldValue::Binary(BinaryFieldState::new(
            rag.granularity_day.unwrap_or(0),
        )),
        ai_suggestion: rag.granularity_day.map(|v| v.to_string()),
    });
    fields.push(DatasetFieldEntry {
        section: "granularity".to_string(),
        name: "time-dimension.month".to_string(),
        description: "Month-level time granularity".to_string(),
        value: FieldValue::Binary(BinaryFieldState::new(
            rag.granularity_month.unwrap_or(0),
        )),
        ai_suggestion: rag.granularity_month.map(|v| v.to_string()),
    });
    fields.push(DatasetFieldEntry {
        section: "granularity".to_string(),
        name: "time-dimension.year".to_string(),
        description: "Year-level time granularity".to_string(),
        value: FieldValue::Binary(BinaryFieldState::new(
            rag.granularity_year.unwrap_or(0),
        )),
        ai_suggestion: rag.granularity_year.map(|v| v.to_string()),
    });

    DatasetEditorEntry {
        name: name.to_string(),
        url: url.to_string(),
        fields,
        data_summary,
    }
}
