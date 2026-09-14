use crate::brand;
use crate::tui::screens;
use crate::tui::Ctx;
use ratatui::backend::Backend;
use ratatui::text::{Line, Span};
use ratatui::Terminal;
use std::io;

pub fn run<B: Backend>(terminal: &mut Terminal<B>, ctx: &Ctx) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();

    let mut lines: Vec<Line<'static>> = vec![Line::from("")];
    lines.extend(super::boot::block_lines(&brand::HFC, &palette));
    lines.push(Line::from(""));
    lines.push(screens::field(
        strings.credits_author,
        brand::AUTHOR,
        &palette,
    ));
    lines.push(screens::field("", brand::AUTHOR_INSTAGRAM, &palette));
    lines.push(screens::field("", brand::AUTHOR_GITHUB, &palette));
    lines.push(Line::from(""));
    lines.push(screens::field_styled(
        "",
        brand::CREW,
        &palette,
        palette.accent_style(),
    ));
    lines.push(screens::field("", brand::CREW_INSTAGRAM, &palette));
    lines.push(screens::field("", brand::CREW_BLOG, &palette));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("  {}", strings.credits_crew),
        palette.value(),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(format!("  {} ", strings.credits_thanks), palette.label()),
        Span::styled(brand::thanks_line(), palette.warm_style()),
    ]));
    lines.push(Line::from(Span::styled(
        format!("  {}", strings.credits_thanks_note),
        palette.dim(),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("  {}", strings.credits_license),
        palette.dim(),
    )));

    screens::page(terminal, ctx, strings.credits_title, lines, &screens::nav_text(ctx))
}
