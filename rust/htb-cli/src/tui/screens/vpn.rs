use crate::tui::event::{self, Input};
use crate::tui::screens;
use crate::tui::widgets::input::confirm;
use crate::tui::widgets::picker::{Item, Outcome, Picker};
use crate::tui::Ctx;
use ratatui::backend::Backend;
use ratatui::text::{Line, Span};
use ratatui::Terminal;
use serde_json::json;
use std::io;

pub const PRODUCTS: [&str; 6] = [
    "labs",
    "starting_point",
    "sherlocks",
    "fortresses",
    "release_arena",
    "competitive",
];

pub fn run<B: Backend>(terminal: &mut Terminal<B>, ctx: &mut Ctx) -> io::Result<()> {
    let mut picker = Picker::new(PRODUCTS.iter().map(|p| Item::plain(*p)).collect());
    loop {
        let palette = ctx.palette();
        let strings = ctx.strings();
        let ctx_ref: &Ctx = ctx;
        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, strings.vpn_title, &palette);
            let inner = screens::centered(52, body_a.height, body_a);
            picker.render(f, inner, strings.vpn_product, &palette);
            screens::render_bars(f, footer_a, &[], ctx_ref);
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
                continue;
            }
            _ => {}
        }
        match picker.handle(&input) {
            Outcome::Leave => return Ok(()),
            Outcome::Chosen(i) => servers(terminal, ctx, PRODUCTS[i])?,
            _ => {}
        }
    }
}

struct Server {
    id: String,
    name: String,
    location: String,
    clients: String,
    full: bool,
}

fn servers<B: Backend>(terminal: &mut Terminal<B>, ctx: &mut Ctx, product: &str) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    screens::loading(terminal, &palette, strings.loading)?;
    let r = ctx
        .client
        .v4_get(&format!("/connections/servers?product={product}"));
    if r.str_at("/status") != "true" {
        let message = r.message();
        let body = if message == "-" {
            strings.vpn_unavailable.to_string()
        } else {
            format!("{} — {message}", strings.vpn_unavailable)
        };
        return screens::action_result(terminal, ctx, product, &body, false);
    }

    let mut assigned_id = r.str_at("/data/assigned/id");
    let assigned_name = r.str_at("/data/assigned/friendly_name");
    let assigned_location = r.str_at("/data/assigned/location");

    let mut list: Vec<Server> = Vec::new();
    if let Some(regions) = r.data.pointer("/data/options").and_then(|v| v.as_object()) {
        for (_, kinds) in regions {
            let Some(kinds) = kinds.as_object() else {
                continue;
            };
            for (_, kind) in kinds {
                let Some(servers) = kind.get("servers").and_then(|v| v.as_object()) else {
                    continue;
                };
                for (_, srv) in servers {
                    list.push(Server {
                        id: srv
                            .get("id")
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        name: srv
                            .get("friendly_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?")
                            .to_string(),
                        location: srv
                            .get("location")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?")
                            .to_string(),
                        clients: srv
                            .get("current_clients")
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        full: srv.get("full").and_then(|v| v.as_bool()).unwrap_or(false),
                    });
                }
            }
        }
    }
    list.sort_by(|a, b| a.location.cmp(&b.location).then(a.name.cmp(&b.name)));
    if list.is_empty() {
        return screens::notice(terminal, ctx, product, strings.no_results, false);
    }

    let mut picker = Picker::new(items_for(&list, &assigned_id, ctx));
    loop {
        let palette = ctx.palette();
        let strings = ctx.strings();
        let title = format!("{} · {product}", strings.vpn_title);
        let assigned_line = format!(
            "{}: {assigned_name} ({assigned_id}, {assigned_location})",
            strings.vpn_assigned
        );
        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, &title, &palette);
            picker.render(f, body_a, &assigned_line, &palette);
            screens::render_footer(f, footer_a, strings.vpn_hint, &palette);
        })?;

        let Some(input) = event::read(event::POLL)? else {
            continue;
        };
        match &input {
            Input::Quit => return Ok(()),
            Input::Help => {
                screens::help_overlay(terminal, ctx)?;
                continue;
            }
            Input::ThemeToggle => {
                ctx.flip_theme();
                picker.replace(items_for(&list, &assigned_id, ctx));
                continue;
            }
            Input::Key('t') | Input::Key('T') => {
                let target = &list[picker.selected()];
                download(terminal, ctx, product, &target.id, 1, &mut assigned_id)?;
                picker.replace(items_for(&list, &assigned_id, ctx));
                continue;
            }
            Input::Key('s') | Input::Key('S') => {
                let target = list[picker.selected()].id.clone();
                switch(terminal, ctx, &target, &mut assigned_id)?;
                picker.replace(items_for(&list, &assigned_id, ctx));
                continue;
            }
            _ => {}
        }
        match picker.handle(&input) {
            Outcome::Leave => return Ok(()),
            Outcome::Chosen(i) => {
                let target = list[i].id.clone();
                download(terminal, ctx, product, &target, 0, &mut assigned_id)?;
                picker.replace(items_for(&list, &assigned_id, ctx));
            }
            _ => {}
        }
    }
}

fn items_for(list: &[Server], assigned: &str, ctx: &Ctx) -> Vec<Item> {
    let p = ctx.palette();
    let s = ctx.strings();
    list.iter()
        .map(|srv| {
            let here = srv.id == assigned;
            Item::with_detail(
                Line::from(vec![
                    Span::styled(
                        if here { "▶ " } else { "  " }.to_string(),
                        if here { p.ok() } else { p.dim() },
                    ),
                    Span::styled(format!("{:<28}", srv.name), p.value()),
                    Span::styled(format!("{:<10}", srv.location), p.info_style()),
                    Span::styled(
                        format!("{} {}", srv.clients, s.vpn_clients),
                        if srv.full { p.err() } else { p.dim() },
                    ),
                ]),
                Line::from(Span::styled(
                    format!(
                        "      ID {}{}",
                        srv.id,
                        if srv.full {
                            format!(" · {}", s.vpn_full)
                        } else {
                            String::new()
                        }
                    ),
                    p.dim(),
                )),
            )
        })
        .collect()
}

fn switch<B: Backend>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    server_id: &str,
    assigned: &mut String,
) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    screens::loading(terminal, &palette, strings.vpn_switching)?;
    let res = ctx
        .client
        .v4_post(&format!("/connections/servers/switch/{server_id}"), json!({}));
    if res.str_at("/status") == "true" || res.ok {
        *assigned = server_id.to_string();
        screens::notice(
            terminal,
            ctx,
            strings.action_switch_server,
            &format!("{} → {}", res.message(), res.str_at("/data/friendly_name")),
            true,
        )
    } else {
        screens::notice(
            terminal,
            ctx,
            strings.action_switch_server,
            &res.message(),
            false,
        )
    }
}

fn download<B: Backend>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    product: &str,
    server_id: &str,
    tcp: u8,
    assigned: &mut String,
) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();

    if server_id != assigned {
        if !confirm(
            terminal,
            &palette,
            strings.confirm_title,
            strings.confirm_switch_server,
            strings.confirm_yes_hint,
        )? {
            return Ok(());
        }
        screens::loading(terminal, &palette, strings.vpn_switching)?;
        let res = ctx
            .client
            .v4_post(&format!("/connections/servers/switch/{server_id}"), json!({}));
        if res.str_at("/status") == "true" || res.ok {
            *assigned = server_id.to_string();
        } else {
            return screens::notice(
                terminal,
                ctx,
                strings.download_failed,
                &res.message(),
                false,
            );
        }
    }

    let proto = if tcp == 1 { "tcp" } else { "udp" };
    let out = ctx
        .cfg
        .download_dir
        .join(format!("htb_{product}_{server_id}_{proto}.ovpn"));
    let url = format!(
        "{}/access/ovpnfile/{server_id}/{tcp}",
        crate::http::BASE_V4
    );
    screens::loading(terminal, &palette, strings.loading)?;
    match ctx.client.download(&url, &out) {
        Ok(size) if size > 500 => {
            let current = ctx.cfg.vpn_dir.join("current.ovpn");
            let _ = std::fs::copy(&out, &current);
            let body = format!(
                "{} {}\n{} {}\n{} {}",
                strings.saved_to,
                out.display(),
                strings.vpn_copy_note,
                ctx.cfg.vpn_dir.display(),
                strings.vpn_connect_note,
                out.display()
            );
            crate::clipboard::copy(&out.display().to_string());
            let lines: Vec<Line<'static>> = body
                .split('\n')
                .map(|l| Line::from(Span::styled(format!("  {l}"), palette.ok())))
                .chain(std::iter::once(Line::from(Span::styled(
                    format!("  {}", strings.copied),
                    palette.dim(),
                ))))
                .collect();
            screens::page(terminal, ctx, strings.vpn_saved, lines, &screens::nav_text(ctx))
        }
        Ok(_) => {
            let _ = std::fs::remove_file(&out);
            screens::notice(terminal, ctx, strings.download_failed, "", false)
        }
        Err(e) => {
            let _ = std::fs::remove_file(&out);
            screens::notice(terminal, ctx, strings.download_failed, &e, false)
        }
    }
}
