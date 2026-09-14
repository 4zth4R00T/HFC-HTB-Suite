use crate::tui::event::{self, Input};
use crate::tui::i18n::Lang;
use crate::tui::screens;
use crate::tui::widgets::picker::{Item, Outcome, Picker};
use crate::tui::Ctx;
use ratatui::backend::Backend;
use ratatui::Terminal;
use std::io;

pub fn run<B: Backend>(terminal: &mut Terminal<B>, ctx: &Ctx) -> io::Result<Option<Lang>> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    let mut picker = Picker::new(
        Lang::all()
            .iter()
            .map(|l| Item::plain(l.label()))
            .collect(),
    );

    loop {
        let ctx_ref: &Ctx = ctx;
        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, strings.lang_title, &palette);
            let inner = screens::centered(46, body_a.height, body_a);
            picker.render(f, inner, strings.lang_prompt, &palette);
            screens::render_bars(f, footer_a, &[], ctx_ref);
        })?;

        let Some(input) = event::read(event::POLL)? else {
            continue;
        };
        if input == Input::Quit {
            return Ok(None);
        }
        match picker.handle(&input) {
            Outcome::Chosen(i) => return Ok(Some(Lang::all()[i])),
            Outcome::Leave => return Ok(None),
            _ => {}
        }
    }
}
