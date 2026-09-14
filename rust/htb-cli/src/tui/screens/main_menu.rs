use crate::brand;
use crate::tui::event::{self, Input};
use crate::tui::screens;
use crate::tui::widgets::picker::{Item, Outcome, Picker};
use crate::tui::Ctx;
use ratatui::backend::Backend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Terminal;
use std::io;

fn entries(ctx: &Ctx) -> Vec<Item> {
    let s = ctx.strings();
    let p = ctx.palette();
    let rows = [
        (s.menu_machines, s.menu_machines_desc),
        (s.menu_catalog, s.menu_catalog_desc),
        (s.menu_vpn, s.menu_vpn_desc),
        (s.menu_profile, s.menu_profile_desc),
        (s.menu_stats, s.menu_stats_desc),
        (s.menu_diagnostics, s.menu_diagnostics_desc),
        (s.menu_settings, s.menu_settings_desc),
        (s.menu_credits, s.menu_credits_desc),
    ];
    let mut items: Vec<Item> = rows
        .iter()
        .map(|(title, desc)| {
            Item::with_detail(
                Line::from(Span::styled(title.to_string(), p.value())),
                Line::from(Span::styled(format!("    {desc}"), p.dim())),
            )
        })
        .collect();
    items.push(Item::with_detail(
        Line::from(Span::styled(s.menu_quit.to_string(), p.warn())),
        Line::from(""),
    ));
    items
}

pub fn run<B: Backend>(terminal: &mut Terminal<B>, ctx: &mut Ctx) -> io::Result<()> {
    let mut picker = Picker::new(entries(ctx));
    let mut status = header_line(ctx);
    loop {
        let palette = ctx.palette();
        let strings = ctx.strings();
        let ctx_ref: &Ctx = ctx;
        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(9),
                    Constraint::Min(6),
                    Constraint::Length(3),
                ])
                .split(area);

            let mut banner: Vec<Line> = super::boot::block_lines(&brand::HFC, &palette);
            banner.push(Line::from(""));
            banner.extend(super::boot::follow_lines(ctx_ref));
            f.render_widget(
                Paragraph::new(banner).alignment(ratatui::layout::Alignment::Center),
                chunks[0],
            );

            picker.render(f, chunks[1], &status, &palette);
            screens::render_bars(f, chunks[2], &[], ctx_ref);
        })?;

        let Some(input) = event::read(event::POLL)? else {
            continue;
        };
        match input {
            Input::Quit => return Ok(()),
            Input::Help => {
                screens::help_overlay(terminal, ctx)?;
                continue;
            }
            Input::ThemeToggle => {
                ctx.flip_theme();
                picker.replace(entries(ctx));
                continue;
            }
            Input::Refresh => {
                ctx.client.forget_cache();
                screens::loading(terminal, &palette, strings.loading)?;
                status = header_line(ctx);
                continue;
            }
            _ => {}
        }
        match picker.handle(&input) {
            Outcome::Leave => return Ok(()),
            Outcome::Chosen(i) => {
                match i {
                    0 => screens::machines::run(terminal, ctx)?,
                    1 => screens::catalog::run(terminal, ctx)?,
                    2 => screens::vpn::run(terminal, ctx)?,
                    3 => screens::profile::run(terminal, ctx)?,
                    4 => screens::stats::run(terminal, ctx)?,
                    5 => screens::diagnostics::run(terminal, ctx)?,
                    6 => screens::settings::run(terminal, ctx)?,
                    7 => screens::credits::run(terminal, ctx)?,
                    _ => return Ok(()),
                }
                picker.replace(entries(ctx));
                status = header_line(ctx);
            }
            _ => {}
        }
    }
}

fn header_line(ctx: &Ctx) -> String {
    let s = ctx.strings();
    let info = ctx.client.v4_get("/user/info");
    if !info.ok {
        return s.main_prompt.to_string();
    }
    let nick = info.str_at("/info/name");
    let active = ctx.client.v5_get("/virtual_machine/active");
    let target = active.str_at("/info/name");
    if target == "-" {
        return nick;
    }
    let kind = active.str_at("/info/type");
    let ip = ctx.client.v4_get("/machine/active").str_at("/info/ip");
    let label = if kind == "-" || kind == "machine" {
        target
    } else {
        format!("{target} ({kind})")
    };
    if ip == "-" {
        format!("{nick}  ·  {label}")
    } else {
        format!("{nick}  ·  {label} {ip}")
    }
}
