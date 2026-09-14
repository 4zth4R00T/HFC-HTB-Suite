use crate::brand;
use crate::tui::event;
use crate::tui::screens;
use crate::tui::theme::Palette;
use crate::tui::Ctx;
use ratatui::backend::Backend;
use ratatui::layout::Alignment;
use ratatui::style::Stylize;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Terminal;
use std::io;
use std::time::Duration;

const GLITCH: &[char] = &['@', '#', '$', '%', '&', '*', '=', '<', '>', '/', '\\', '|'];

pub struct Rng(u32);

impl Rng {
    pub fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0xdead_beef)
            | 1;
        Rng(seed)
    }

    pub fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
}

pub fn block_lines(art: &[&'static str], palette: &Palette) -> Vec<Line<'static>> {
    art.iter()
        .map(|row| Line::from(Span::styled(*row, palette.accent_style())))
        .collect()
}

fn glitched(art: &[&'static str], palette: &Palette, rng: &mut Rng, density: u32) -> Vec<Line<'static>> {
    art.iter()
        .map(|row| {
            let text: String = row
                .chars()
                .map(|c| {
                    if c != ' ' && density > 0 && rng.next() % density == 0 {
                        GLITCH[(rng.next() as usize) % GLITCH.len()]
                    } else {
                        c
                    }
                })
                .collect();
            Line::from(Span::styled(text, palette.accent_style()))
        })
        .collect()
}

pub fn follow_lines(ctx: &Ctx) -> Vec<Line<'static>> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    vec![
        Line::from(vec![
            Span::styled(format!("{} — ", brand::AUTHOR), palette.value()),
            Span::styled(strings.follow_author, palette.label()),
            Span::styled(format!(" {}", brand::AUTHOR_INSTAGRAM), palette.info_style()),
        ]),
        Line::from(vec![
            Span::styled(format!("{} — ", brand::CREW), palette.value()),
            Span::styled(strings.follow_crew, palette.label()),
            Span::styled(format!(" {}", brand::CREW_INSTAGRAM), palette.info_style()),
        ]),
    ]
}

pub fn run<B: Backend>(terminal: &mut Terminal<B>, ctx: &Ctx) -> io::Result<()> {
    if std::env::var("HTB_NO_ANIM").as_deref() == Ok("1") {
        return Ok(());
    }
    let palette = ctx.palette();
    let strings = ctx.strings();
    let mut rng = Rng::new();
    let welcome = strings.welcome[(rng.next() as usize) % strings.welcome.len()];

    for &density in &[3u32, 8, 30, 0] {
        let mut frame_rng = Rng(rng.next() | 1);
        let hfc = glitched(&brand::HFC, &palette, &mut frame_rng, density);
        let word = glitched(&brand::WORDMARK, &palette, &mut frame_rng, density);
        let settled = density == 0;
        terminal.draw(|f| {
            screens::paint_background(f, &palette);
            let mut lines = hfc.clone();
            lines.push(Line::from(""));
            lines.extend(word.clone());
            lines.push(Line::from(""));
            lines.extend(follow_lines(ctx));
            if settled {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(welcome, palette.dim().italic())));
            }
            f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), f.area());
        })?;
        std::thread::sleep(Duration::from_millis(if settled { 700 } else { 70 }));
    }
    event::drain();
    Ok(())
}
