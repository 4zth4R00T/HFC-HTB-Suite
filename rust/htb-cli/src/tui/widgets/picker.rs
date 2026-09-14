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
        Item {
            title,
            detail: Some(detail),
        }
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
        Picker {
            items,
            state,
            area: Rect::default(),
            row_height,
        }
    }

    pub fn selected(&self) -> usize {
        self.state.selected().unwrap_or(0)
    }

    pub fn select(&mut self, index: usize) {
        if self.items.is_empty() {
            return;
        }
        self.state.select(Some(index.min(self.items.len() - 1)));
    }

    pub fn replace(&mut self, items: Vec<Item>) {
        let keep = self.selected();
        self.row_height = if items.iter().any(|i| i.detail.is_some()) {
            2
        } else {
            1
        };
        self.items = items;
        if self.items.is_empty() {
            self.state.select(None);
        } else {
            self.state.select(Some(keep.min(self.items.len() - 1)));
        }
    }

    fn page(&self) -> usize {
        let rows = self.area.height.saturating_sub(2) / self.row_height.max(1);
        (rows as usize).max(1)
    }

    fn step(&mut self, delta: isize) {
        let len = self.items.len();
        if len == 0 {
            return;
        }
        let cur = self.selected() as isize;
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
                self.select(0);
                Outcome::Moved
            }
            Input::End => {
                let last = self.items.len().saturating_sub(1);
                self.select(last);
                Outcome::Moved
            }
            Input::Select => {
                if self.items.is_empty() {
                    Outcome::Ignored
                } else {
                    Outcome::Chosen(self.selected())
                }
            }
            Input::Back => Outcome::Leave,
            Input::Click(col, row) => match self.index_at(*col, *row) {
                Some(i) if i == self.selected() => Outcome::Chosen(i),
                Some(i) => {
                    self.select(i);
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
        if idx < self.items.len() {
            Some(idx)
        } else {
            None
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, title: &str, palette: &Palette) {
        self.area = area;
        let items: Vec<ListItem> = self
            .items
            .iter()
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

        let list = List::new(items)
            .block(block)
            .style(Style::default().bg(palette.bg).fg(palette.text))
            .highlight_style(palette.selected())
            .highlight_symbol("▌ ");

        f.render_stateful_widget(list, area, &mut self.state);

        let visible = self.page();
        if self.items.len() > visible {
            let mut sb_state = ScrollbarState::new(self.items.len()).position(self.selected());
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
        if self.items.is_empty() {
            "0/0".to_string()
        } else {
            format!("{}/{}", self.selected() + 1, self.items.len())
        }
    }
}
