use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use std::io;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    Select,
    Back,
    Quit,
    Help,
    ThemeToggle,
    Refresh,
    Key(char),
    Copy,
    PasteRequest,
    Click(u16, u16),
    Paste(String),
    Redraw,
}

pub const POLL: Duration = Duration::from_millis(120);

pub fn read(timeout: Duration) -> io::Result<Option<Input>> {
    if !event::poll(timeout)? {
        return Ok(None);
    }
    Ok(match event::read()? {
        Event::Key(k) => from_key(k),
        Event::Mouse(m) => from_mouse(m),
        Event::Paste(s) => Some(Input::Paste(s)),
        Event::Resize(_, _) => Some(Input::Redraw),
        _ => None,
    })
}

pub fn wait() -> io::Result<Input> {
    loop {
        if let Some(i) = read(POLL)? {
            if i != Input::Redraw {
                return Ok(i);
            }
        }
    }
}

fn from_key(k: KeyEvent) -> Option<Input> {
    if k.kind == KeyEventKind::Release {
        return None;
    }
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    if ctrl {
        return match k.code {
            KeyCode::Char('c') => Some(Input::Copy),
            KeyCode::Char('v') => Some(Input::PasteRequest),
            KeyCode::Char('q') | KeyCode::Char('d') => Some(Input::Quit),
            KeyCode::Char('b') => Some(Input::PageUp),
            KeyCode::Char('f') => Some(Input::PageDown),
            KeyCode::Char('u') => Some(Input::PageUp),
            KeyCode::Char('n') => Some(Input::Down),
            KeyCode::Char('p') => Some(Input::Up),
            KeyCode::Char('l') => Some(Input::Redraw),
            KeyCode::Char('r') => Some(Input::Refresh),
            _ => None,
        };
    }
    match k.code {
        KeyCode::Up | KeyCode::Char('k') => Some(Input::Up),
        KeyCode::Down | KeyCode::Char('j') => Some(Input::Down),
        KeyCode::PageUp => Some(Input::PageUp),
        KeyCode::PageDown | KeyCode::Char(' ') => Some(Input::PageDown),
        KeyCode::Home => Some(Input::Home),
        KeyCode::End => Some(Input::End),
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => Some(Input::Select),
        KeyCode::Esc | KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => Some(Input::Back),
        KeyCode::Char('q') | KeyCode::Char('Q') => Some(Input::Back),
        KeyCode::Char('?') | KeyCode::F(1) => Some(Input::Help),
        KeyCode::F(2) => Some(Input::ThemeToggle),
        KeyCode::F(5) => Some(Input::Refresh),
        KeyCode::Char(c) => Some(Input::Key(c)),
        _ => None,
    }
}

fn from_mouse(m: MouseEvent) -> Option<Input> {
    match m.kind {
        MouseEventKind::ScrollUp => Some(Input::Up),
        MouseEventKind::ScrollDown => Some(Input::Down),
        MouseEventKind::Down(MouseButton::Left) => Some(Input::Click(m.column, m.row)),
        MouseEventKind::Down(MouseButton::Right) => Some(Input::Back),
        _ => None,
    }
}

pub fn drain() {
    while event::poll(Duration::from_millis(0)).unwrap_or(false) {
        let _ = event::read();
    }
}
