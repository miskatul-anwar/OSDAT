use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};

// ──────────────────────────────────────────────
// BinaryField — Toggleable [0] / [1] widget
// ──────────────────────────────────────────────

pub struct BinaryField<'a> {
    label: &'a str,
    focused: bool,
}

impl<'a> BinaryField<'a> {
    pub fn new(label: &'a str, focused: bool) -> Self {
        Self { label, focused }
    }
}

pub struct BinaryFieldState {
    pub value: u8,
}

impl BinaryFieldState {
    pub fn new(value: u8) -> Self {
        Self { value }
    }

    pub fn toggle(&mut self) {
        self.value = if self.value == 0 { 1 } else { 0 };
    }
}

impl StatefulWidget for BinaryField<'_> {
    type State = BinaryFieldState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let style = if self.focused {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let value_style = if state.value == 1 {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let line = Line::from(vec![
            Span::styled(format!("{}: ", self.label), style),
            Span::styled(format!("[{}]", state.value), value_style),
        ]);

        Paragraph::new(line).render(area, buf);
    }
}

// ──────────────────────────────────────────────
// TextInput — Single-line text input with cursor
// ──────────────────────────────────────────────

pub struct TextInput<'a> {
    label: &'a str,
    focused: bool,
}

impl<'a> TextInput<'a> {
    pub fn new(label: &'a str, focused: bool) -> Self {
        Self { label, focused }
    }
}

pub struct TextInputState {
    pub value: String,
    pub cursor: usize,
}

impl TextInputState {
    pub fn new(value: String) -> Self {
        let cursor = value.len();
        Self { value, cursor }
    }

    pub fn insert(&mut self, c: char) {
        self.value.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn delete_back(&mut self) {
        if self.cursor > 0 {
            let prev = self.value[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.value.remove(prev);
            self.cursor = prev;
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.value[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor += self.value[self.cursor..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
        }
    }
}

impl StatefulWidget for TextInput<'_> {
    type State = TextInputState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let border_style = if self.focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(self.label);

        let inner = block.inner(area);
        block.render(area, buf);

        // Render text with cursor
        let display_text = if self.focused {
            let before = &state.value[..state.cursor];
            let cursor_char = state.value[state.cursor..].chars().next().unwrap_or(' ');
            let after_cursor = if state.cursor < state.value.len() {
                &state.value[state.cursor + cursor_char.len_utf8()..]
            } else {
                ""
            };

            Line::from(vec![
                Span::raw(before.to_string()),
                Span::styled(
                    cursor_char.to_string(),
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White),
                ),
                Span::raw(after_cursor.to_string()),
            ])
        } else {
            Line::from(state.value.clone())
        };

        Paragraph::new(display_text).render(inner, buf);
    }
}

// ──────────────────────────────────────────────
// ProgressList — Items with status icons
// ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ProgressStatus {
    Pending,
    Active,
    Done,
    Failed,
}

#[derive(Debug, Clone)]
pub struct ProgressItem {
    pub label: String,
    pub status: ProgressStatus,
    pub detail: String,
}

pub struct ProgressList<'a> {
    title: &'a str,
}

impl<'a> ProgressList<'a> {
    pub fn new(title: &'a str) -> Self {
        Self { title }
    }
}

pub struct ProgressListState {
    pub items: Vec<ProgressItem>,
}

impl ProgressListState {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }
}

impl StatefulWidget for ProgressList<'_> {
    type State = ProgressListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(self.title);

        let inner = block.inner(area);
        block.render(area, buf);

        let lines: Vec<Line> = state
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                if i as u16 >= inner.height {
                    return None;
                }

                let (icon, icon_style) = match item.status {
                    ProgressStatus::Done => ("✓", Style::default().fg(Color::Green)),
                    ProgressStatus::Active => ("⟳", Style::default().fg(Color::Yellow)),
                    ProgressStatus::Pending => ("○", Style::default().fg(Color::DarkGray)),
                    ProgressStatus::Failed => ("✗", Style::default().fg(Color::Red)),
                };

                Some(Line::from(vec![
                    Span::styled(format!(" {icon} "), icon_style),
                    Span::raw(&item.label),
                    if !item.detail.is_empty() {
                        Span::styled(
                            format!("  {}", item.detail),
                            Style::default().fg(Color::DarkGray),
                        )
                    } else {
                        Span::raw("")
                    },
                ]))
            })
            .collect();

        Paragraph::new(lines).render(inner, buf);
    }
}
