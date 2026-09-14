use crate::tui::theme::Palette;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::backend::Backend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Terminal;
use std::io;
use std::time::Duration;

pub struct Field {
    buffer: String,
    cursor: usize,
}

impl Field {
    fn new(initial: &str) -> Self {
        Field {
            buffer: initial.to_string(),
            cursor: initial.chars().count(),
        }
    }

    fn byte_at(&self, char_index: usize) -> usize {
        self.buffer
            .char_indices()
            .nth(char_index)
            .map(|(b, _)| b)
            .unwrap_or(self.buffer.len())
    }

    fn insert(&mut self, c: char) {
        let at = self.byte_at(self.cursor);
        self.buffer.insert(at, c);
        self.cursor += 1;
    }

    fn insert_str(&mut self, s: &str) {
        for c in s.chars().filter(|c| !c.is_control()) {
            self.insert(c);
        }
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let at = self.byte_at(self.cursor - 1);
        self.buffer.remove(at);
        self.cursor -= 1;
    }

    fn delete(&mut self) {
        if self.cursor >= self.buffer.chars().count() {
            return;
        }
        let at = self.byte_at(self.cursor);
        self.buffer.remove(at);
    }

    fn kill_word(&mut self) {
        while self.cursor > 0 {
            let prev: char = self.buffer.chars().nth(self.cursor - 1).unwrap_or(' ');
            self.backspace();
            if prev.is_whitespace() {
                break;
            }
        }
    }

    fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
    }
}

pub struct Prompt<'a> {
    pub title: &'a str,
    pub label: &'a str,
    pub hint: &'a str,
    pub paste_hint: &'a str,
    pub initial: &'a str,
    pub mask: bool,
}

impl<'a> Prompt<'a> {
    pub fn new(title: &'a str, label: &'a str) -> Self {
        Prompt {
            title,
            label,
            hint: "",
            paste_hint: "",
            initial: "",
            mask: false,
        }
    }

    pub fn hint(mut self, h: &'a str) -> Self {
        self.hint = h;
        self
    }

    pub fn paste_hint(mut self, h: &'a str) -> Self {
        self.paste_hint = h;
        self
    }

    pub fn masked(mut self) -> Self {
        self.mask = true;
        self
    }
}

pub fn ask<B: Backend>(
    terminal: &mut Terminal<B>,
    palette: &Palette,
    prompt: Prompt,
) -> io::Result<Option<String>> {
    let mut field = Field::new(prompt.initial);
    loop {
        terminal.draw(|f| {
            let area = f.area();
            f.render_widget(
                Block::default().style(Style::default().bg(palette.bg)),
                area,
            );
            let height = 7;
            let popup = centered(74, height, area);
            f.render_widget(Clear, popup);

            let shown: String = if prompt.mask {
                "•".repeat(field.buffer.chars().count())
            } else {
                field.buffer.clone()
            };
            let before: String = shown.chars().take(field.cursor).collect();
            let under: String = shown.chars().skip(field.cursor).take(1).collect();
            let after: String = shown.chars().skip(field.cursor + 1).collect();

            let mut lines = vec![
                Line::from(Span::styled(prompt.label.to_string(), palette.label())),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  ", palette.value()),
                    Span::styled(before, palette.value()),
                    Span::styled(
                        if under.is_empty() { " ".into() } else { under },
                        palette.selected(),
                    ),
                    Span::styled(after, palette.value()),
                ]),
            ];
            lines.push(Line::from(Span::styled(
                if prompt.hint.is_empty() {
                    prompt.paste_hint.to_string()
                } else {
                    format!("{}  ·  {}", prompt.hint, prompt.paste_hint)
                },
                palette.dim(),
            )));

            let block = Block::default()
                .title(Line::from(Span::styled(
                    format!(" {} ", prompt.title),
                    palette.title(),
                )))
                .borders(Borders::ALL)
                .border_style(palette.accent_style())
                .style(Style::default().bg(palette.bg_panel));
            f.render_widget(Paragraph::new(lines).block(block), popup);
        })?;

        if !event::poll(Duration::from_millis(120))? {
            continue;
        }
        match event::read()? {
            Event::Paste(s) => field.insert_str(&s),
            Event::Key(k) if k.kind != KeyEventKind::Release => {
                let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
                match (ctrl, k.code) {
                    (_, KeyCode::Enter) => return Ok(Some(field.buffer)),
                    (_, KeyCode::Esc) => return Ok(None),
                    (true, KeyCode::Char('u')) => field.clear(),
                    (true, KeyCode::Char('w')) => field.kill_word(),
                    (true, KeyCode::Char('a')) => field.cursor = 0,
                    (true, KeyCode::Char('e')) => field.cursor = field.buffer.chars().count(),
                    (true, KeyCode::Char('c')) => return Ok(None),
                    (true, KeyCode::Char('v')) => {
                        if let Some(text) = crate::clipboard::paste() {
                            field.insert_str(&text);
                        }
                    }
                    (false, KeyCode::Backspace) => field.backspace(),
                    (false, KeyCode::Delete) => field.delete(),
                    (false, KeyCode::Left) => field.cursor = field.cursor.saturating_sub(1),
                    (false, KeyCode::Right) => {
                        field.cursor = (field.cursor + 1).min(field.buffer.chars().count())
                    }
                    (false, KeyCode::Home) => field.cursor = 0,
                    (false, KeyCode::End) => field.cursor = field.buffer.chars().count(),
                    (false, KeyCode::Char(c)) => field.insert(c),
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

pub fn confirm<B: Backend>(
    terminal: &mut Terminal<B>,
    palette: &Palette,
    title: &str,
    question: &str,
    yes_hint: &str,
) -> io::Result<bool> {
    loop {
        terminal.draw(|f| {
            let area = f.area();
            f.render_widget(
                Block::default().style(Style::default().bg(palette.bg)),
                area,
            );
            let popup = centered(70, 7, area);
            f.render_widget(Clear, popup);
            let block = Block::default()
                .title(Line::from(Span::styled(format!(" {title} "), palette.title())))
                .borders(Borders::ALL)
                .border_style(palette.warn())
                .style(Style::default().bg(palette.bg_panel));
            let lines = vec![
                Line::from(Span::styled(question.to_string(), palette.value())),
                Line::from(""),
                Line::from(Span::styled(yes_hint.to_string(), palette.dim())),
            ];
            f.render_widget(
                Paragraph::new(lines)
                    .wrap(ratatui::widgets::Wrap { trim: true })
                    .block(block),
                popup,
            );
        })?;

        if !event::poll(Duration::from_millis(120))? {
            continue;
        }
        if let Event::Key(k) = event::read()? {
            if k.kind == KeyEventKind::Release {
                continue;
            }
            match k.code {
                KeyCode::Enter | KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Char('y')
                | KeyCode::Char('Y') | KeyCode::Char('д') | KeyCode::Char('Д') => {
                    return Ok(true)
                }
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char('q')
                | KeyCode::Char('н') => return Ok(false),
                _ => {}
            }
        }
    }
}

fn centered(percent_x: u16, height: u16, r: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
