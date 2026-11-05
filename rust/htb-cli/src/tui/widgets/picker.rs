use crate::tui::event::Input;
use crate::tui::theme::Palette;
use ratatui::layout::{Margin, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, List, ListItem, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::Frame;

pub struct Item {
    pub title: Line<'static>,
    pub detail: Option<Line<'static>>,
}

impl Item {
    pub fn plain(text: impl Into<String>) -> Self {
        Item {
            title: Line::from(text.into()),
            detail: None,
        }
    }

    pub fn with_detail(title: Line<'static>, detail: Line<'static>) -> Self {
        let blank = detail.spans.iter().all(|s| s.content.trim().is_empty());
        Item {
            title,
            detail: if blank { None } else { Some(detail) },
        }
    }

    pub fn plain_line(title: Line<'static>) -> Self {
        Item { title, detail: None }
    }
}

pub enum Outcome {
    Moved,
    Chosen(usize),
    Leave,
    Ignored,
}

pub struct Picker {
    pub items: Vec<Item>,
    state: ListState,
    area: Rect,
    row_height: u16,
    filter: String,
    filtering: bool,
    visible: Vec<usize>,
}

fn plain(line: &Line<'static>) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect::<String>().to_lowercase()
}

impl Picker {
    pub fn new(items: Vec<Item>) -> Self {
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        let row_height = if items.iter().any(|i| i.detail.is_some()) {
            2
        } else {
            1
        };
        let visible = (0..items.len()).collect();
        Picker {
            items,
            state,
            area: Rect::default(),
            row_height,
            filter: String::new(),
            filtering: false,
            visible,
        }
    }

    fn refilter(&mut self) {
        let needle = self.filter.to_lowercase();
        let words: Vec<&str> = needle.split_whitespace().collect();
        self.visible = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, it)| {
                if words.is_empty() {
                    return true;
                }
                let mut text = plain(&it.title);
                if let Some(d) = &it.detail {
                    text.push(' ');
                    text.push_str(&plain(d));
                }
                words.iter().all(|w| text.contains(w))
            })
            .map(|(i, _)| i)
            .collect();
        if self.visible.is_empty() {
            self.state.select(None);
        } else {
            let cur = self.state.selected().unwrap_or(0);
            self.state.select(Some(cur.min(self.visible.len() - 1)));
        }
    }

    pub fn selected(&self) -> usize {
        let pos = self.state.selected().unwrap_or(0);
        self.visible.get(pos).copied().unwrap_or(0)
    }

    pub fn select(&mut self, index: usize) {
        if self.items.is_empty() {
            return;
        }
        let index = index.min(self.items.len() - 1);
        match self.visible.iter().position(|&i| i == index) {
            Some(pos) => self.state.select(Some(pos)),
            None if !self.visible.is_empty() => self.state.select(Some(0)),
            None => self.state.select(None),
        }
    }

    pub fn replace(&mut self, items: Vec<Item>) {
        let keep = self.selected();
        self.row_height = if items.iter().any(|i| i.detail.is_some()) {
            2
        } else {
            1
        };
        self.items = items;
        self.refilter();
        if self.items.is_empty() {
            self.state.select(None);
        } else {
            self.select(keep);
        }
    }

    pub fn filter_text(&self) -> &str {
        &self.filter
    }

    pub fn is_filtering(&self) -> bool {
        self.filtering
    }

    pub fn capture(&mut self, input: &Input) -> bool {
        if self.filtering {
            match input {
                Input::Key(c) if !c.is_control() => {
                    self.filter.push(*c);
                    self.refilter();
                    true
                }
                Input::Back => {
                    if self.filter.pop().is_none() {
                        self.filtering = false;
                    }
                    self.refilter();
                    true
                }
                Input::Select => {
                    self.filtering = false;
                    true
                }
                Input::Quit | Input::Help | Input::ThemeToggle => false,
                Input::Copy => {
                    self.filter.push('c');
                    self.refilter();
                    true
                }
                Input::Refresh => {
                    self.filter.push('r');
                    self.refilter();
                    true
                }
                Input::PasteRequest => {
                    self.filter.push('v');
                    self.refilter();
                    true
                }
                Input::Paste(text) => {
                    self.filter.push_str(text.trim());
                    self.refilter();
                    true
                }
                _ => false,
            }
        } else {
            match input {
                Input::Key('/') => {
                    self.filtering = true;
                    true
                }
                Input::Back if !self.filter.is_empty() => {
                    self.filter.clear();
                    self.refilter();
                    true
                }
                _ => false,
            }
        }
    }

    fn page(&self) -> usize {
        let rows = self.area.height.saturating_sub(2) / self.row_height.max(1);
        (rows as usize).max(1)
    }

    fn step(&mut self, delta: isize) {
        let len = self.visible.len();
        if len == 0 {
            return;
        }
        let cur = self.state.selected().unwrap_or(0) as isize;
        let next = cur + delta;
        let next = if next < 0 {
            if delta == -1 {
                len as isize - 1
            } else {
                0
            }
        } else if next >= len as isize {
            if delta == 1 {
                0
            } else {
                len as isize - 1
            }
        } else {
            next
        };
        self.state.select(Some(next as usize));
    }

    pub fn handle(&mut self, input: &Input) -> Outcome {
        match input {
            Input::Up => {
                self.step(-1);
                Outcome::Moved
            }
            Input::Down => {
                self.step(1);
                Outcome::Moved
            }
            Input::PageUp => {
                let p = self.page() as isize;
                self.step(-p);
                Outcome::Moved
            }
            Input::PageDown => {
                let p = self.page() as isize;
                self.step(p);
                Outcome::Moved
            }
            Input::Home => {
                if !self.visible.is_empty() {
                    self.state.select(Some(0));
                }
                Outcome::Moved
            }
            Input::End => {
                if !self.visible.is_empty() {
                    self.state.select(Some(self.visible.len() - 1));
                }
                Outcome::Moved
            }
            Input::Select => {
                if self.visible.is_empty() {
                    Outcome::Ignored
                } else {
                    Outcome::Chosen(self.selected())
                }
            }
            Input::Back => Outcome::Leave,
            Input::Click(col, row) => match self.index_at(*col, *row) {
                Some(pos) if Some(pos) == self.state.selected() => Outcome::Chosen(self.selected()),
                Some(pos) => {
                    self.state.select(Some(pos));
                    Outcome::Moved
                }
                None => Outcome::Ignored,
            },
            _ => Outcome::Ignored,
        }
    }

    fn index_at(&self, col: u16, row: u16) -> Option<usize> {
        let inner_top = self.area.y + 1;
        let inner_bottom = self.area.y + self.area.height.saturating_sub(1);
        if col <= self.area.x
            || col >= self.area.x + self.area.width.saturating_sub(1)
            || row < inner_top
            || row >= inner_bottom
        {
            return None;
        }
        let offset = self.state.offset();
        let rel = (row - inner_top) / self.row_height.max(1);
        let idx = offset + rel as usize;
        if idx < self.visible.len() {
            Some(idx)
        } else {
            None
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, title: &str, palette: &Palette) {
        self.area = area;
        let items: Vec<ListItem> = self
            .visible
            .iter()
            .map(|&i| &self.items[i])
            .map(|it| {
                let mut lines = vec![it.title.clone()];
                if let Some(d) = &it.detail {
                    lines.push(d.clone());
                }
                ListItem::new(lines)
            })
            .collect();

        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_style(palette.border_style())
            .style(Style::default().bg(palette.bg));
        if !title.is_empty() {
            block = block.title(Line::from(Span::styled(
                format!(" {title} "),
                palette.label(),
            )));
        }
        if self.filtering || !self.filter.is_empty() {
            let cursor = if self.filtering { "▏" } else { "" };
            block = block.title(
                Line::from(Span::styled(
                    format!(" /{}{} {}/{} ", self.filter, cursor, self.visible.len(), self.items.len()),
                    palette.accent_style(),
                ))
                .right_aligned(),
            );
        }

        let list = List::new(items)
            .block(block)
            .style(Style::default().bg(palette.bg).fg(palette.text))
            .highlight_style(palette.selected())
            .highlight_symbol("▌ ");

        f.render_stateful_widget(list, area, &mut self.state);

        let visible = self.page();
        if self.visible.len() > visible {
            let mut sb_state = ScrollbarState::new(self.visible.len()).position(self.state.selected().unwrap_or(0));
            f.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None)
                    .track_symbol(Some("░"))
                    .thumb_symbol("▐")
                    .track_style(palette.border_style())
                    .thumb_style(palette.accent_style()),
                area.inner(Margin {
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut sb_state,
            );
        }
    }

    pub fn position_label(&self) -> String {
        if self.visible.is_empty() {
            format!("0/{}", self.items.len())
        } else {
            format!("{}/{}", self.state.selected().unwrap_or(0) + 1, self.visible.len())
        }
    }
}
