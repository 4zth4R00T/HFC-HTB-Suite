pub mod event;
pub mod i18n;
pub mod screens;
pub mod theme;
pub mod widgets;

use crate::config::Config;
use crate::http::Client;
use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use i18n::Lang;
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::Terminal;
use std::io::{self, Stdout};
use theme::ThemeMode;

pub struct Ctx<'a> {
    pub client: &'a Client,
    pub cfg: &'a Config,
    pub lang: Lang,
    pub theme: ThemeMode,
}

impl<'a> Ctx<'a> {
    pub fn strings(&self) -> i18n::Strings {
        i18n::strings(self.lang)
    }

    pub fn palette(&self) -> theme::Palette {
        theme::palette(self.theme)
    }

    pub fn flip_theme(&mut self) {
        self.theme = self.theme.toggled();
        let _ = self.cfg.write_theme(self.theme.label());
    }
}

fn init_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut out = io::stdout();
    execute!(
        out,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    Terminal::new(CrosstermBackend::new(out))
}

fn restore_terminal<B: Backend + io::Write>(terminal: &mut Terminal<B>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    Ok(())
}

pub fn run(client: &Client, cfg: &Config) -> io::Result<()> {
    let theme = cfg
        .read_theme()
        .map(|s| ThemeMode::parse(&s))
        .unwrap_or(ThemeMode::Dark);
    let lang = cfg.read_lang().map(|s| Lang::parse(&s)).unwrap_or(Lang::Es);
    let first_run = cfg.read_lang().is_none();

    let mut terminal = init_terminal()?;
    let mut ctx = Ctx {
        client,
        cfg,
        lang,
        theme,
    };

    let result = (|| -> io::Result<()> {
        screens::boot::run(&mut terminal, &ctx)?;
        if first_run {
            if let Some(chosen) = screens::language::run(&mut terminal, &ctx)? {
                ctx.lang = chosen;
                let _ = cfg.write_lang(chosen.code());
            }
        }
        screens::main_menu::run(&mut terminal, &mut ctx)
    })();

    restore_terminal(&mut terminal)?;
    result
}
