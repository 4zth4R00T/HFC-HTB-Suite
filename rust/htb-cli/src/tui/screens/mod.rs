pub mod boot;
pub mod catalog;
pub mod credits;
pub mod diagnostics;
pub mod language;
pub mod machines;
pub mod main_menu;
pub mod profile;
pub mod settings;
pub mod stats;
pub mod vpn;

use crate::tui::event::{self, Input};
use crate::tui::theme::Palette;
use crate::tui::Ctx;
use ratatui::backend::Backend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use std::io;

pub fn frame_layout(area: Rect) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(area);
    (chunks[0], chunks[1], chunks[2])
}

pub fn paint_background(f: &mut Frame, palette: &Palette) {
    f.render_widget(
        Block::default().style(Style::default().bg(palette.bg)),
        f.area(),
    );
}

pub fn render_title(f: &mut Frame, area: Rect, title: &str, palette: &Palette) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(palette.border_style())
        .style(Style::default().bg(palette.bg));
    let para = Paragraph::new(Line::from(Span::styled(title.to_string(), palette.title())))
        .alignment(Alignment::Center)
        .block(block);
    f.render_widget(para, area);
}

pub fn render_footer(f: &mut Frame, area: Rect, hint: &str, palette: &Palette) {
    let para = Paragraph::new(Line::from(Span::styled(hint.to_string(), palette.dim())))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(Style::default().bg(palette.bg));
    f.render_widget(para, area);
}

pub fn copy_value<B: Backend>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    value: &str,
) -> io::Result<()> {
    let s = ctx.strings();
    if value.trim().is_empty() || value == "-" {
        return notice(terminal, ctx, s.action_copy, s.nothing_to_copy, false);
    }
    crate::clipboard::copy(value);
    notice(
        terminal,
        ctx,
        s.action_copy,
        &format!("{}\n\n{value}", s.copied),
        true,
    )
}

pub fn api_problem(status: u16, ctx: &Ctx) -> &'static str {
    let s = ctx.strings();
    if status == 429 {
        s.api_rate_limited
    } else {
        s.token_invalid
    }
}

pub fn keys_line(pairs: &[(&str, &str)], palette: &Palette) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    for (index, (key, label)) in pairs.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled("    ", palette.dim()));
        }
        spans.push(Span::styled(key.to_string(), palette.key()));
        spans.push(Span::styled(" ", palette.dim()));
        spans.push(Span::styled(label.to_string(), palette.label()));
    }
    Line::from(spans)
}

pub fn nav_text(ctx: &Ctx) -> String {
    nav_pairs(ctx)
        .iter()
        .map(|(key, label)| format!("{key} {label}"))
        .collect::<Vec<_>>()
        .join("    ")
}

pub fn nav_pairs(ctx: &Ctx) -> Vec<(&'static str, &'static str)> {
    let s = ctx.strings();
    vec![
        ("↑↓", s.nav_move),
        ("↵", s.nav_open),
        ("←", s.nav_back),
        ("?", s.nav_help),
    ]
}

pub fn render_bars(f: &mut Frame, area: Rect, actions: &[(&str, &str)], ctx: &Ctx) {
    let palette = ctx.palette();
    let mut lines = Vec::new();
    if !actions.is_empty() {
        lines.push(keys_line(actions, &palette));
    }
    let nav: Vec<(&str, &str)> = nav_pairs(ctx)
        .into_iter()
        .filter(|(key, _)| !actions.iter().any(|(taken, _)| taken == key))
        .collect();
    lines.push(keys_line(&nav, &palette));
    let para = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(Style::default().bg(palette.bg));
    f.render_widget(para, area);
}

pub fn loading<B: Backend>(
    terminal: &mut Terminal<B>,
    palette: &Palette,
    label: &str,
) -> io::Result<()> {
    terminal.draw(|f| {
        let area = f.area();
        f.render_widget(Clear, area);
        paint_background(f, palette);
        let (_, body, _) = frame_layout(area);
        let para = Paragraph::new(Line::from(Span::styled(
            label.to_string(),
            palette.accent_style(),
        )))
        .alignment(Alignment::Center);
        f.render_widget(para, body);
    })?;
    Ok(())
}

pub fn page<B: Backend>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    title: &str,
    lines: Vec<Line<'static>>,
    hint: &str,
) -> io::Result<()> {
    let mut scroll: u16 = 0;
    loop {
        let palette = ctx.palette();
        let height = terminal.size().map(|s| s.height).unwrap_or(24);
        let viewport = height.saturating_sub(7);
        let max_scroll = (lines.len() as u16).saturating_sub(viewport);
        if scroll > max_scroll {
            scroll = max_scroll;
        }
        let body_lines = lines.clone();
        terminal.draw(|f| {
            let area = f.area();
            paint_background(f, &palette);
            let (title_a, body_a, footer_a) = frame_layout(area);
            render_title(f, title_a, title, &palette);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(palette.border_style())
                .style(Style::default().bg(palette.bg));
            f.render_widget(
                Paragraph::new(body_lines)
                    .wrap(Wrap { trim: false })
                    .block(block)
                    .scroll((scroll, 0)),
                body_a,
            );
            render_footer(f, footer_a, hint, &palette);
        })?;

        let Some(input) = event::read(event::POLL)? else {
            continue;
        };
        match input {
            Input::Up => scroll = scroll.saturating_sub(1),
            Input::Down => scroll = (scroll + 1).min(max_scroll),
            Input::PageUp => scroll = scroll.saturating_sub(viewport.max(1)),
            Input::PageDown => scroll = (scroll + viewport.max(1)).min(max_scroll),
            Input::Home => scroll = 0,
            Input::End => scroll = max_scroll,
            Input::Back | Input::Quit => return Ok(()),
            _ => {}
        }
    }
}

pub fn notice<B: Backend>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    title: &str,
    body: &str,
    good: bool,
) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    let style = if good { palette.ok() } else { palette.err() };
    let lines: Vec<Line<'static>> = body
        .split('\n')
        .map(|l| Line::from(Span::styled(format!("  {l}"), style)))
        .collect();

    terminal.draw(|f| {
        let area = f.area();
        paint_background(f, &palette);
        let (title_a, body_a, footer_a) = frame_layout(area);
        render_title(f, title_a, title, &palette);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(if good {
                palette.accent_style()
            } else {
                palette.err()
            })
            .style(Style::default().bg(palette.bg));
        let mut all = vec![Line::from("")];
        all.extend(lines);
        f.render_widget(
            Paragraph::new(all).wrap(Wrap { trim: false }).block(block),
            body_a,
        );
        render_footer(f, footer_a, strings.press_key, &palette);
    })?;

    event::drain();
    loop {
        if let Some(input) = event::read(event::POLL)? {
            if input != Input::Redraw {
                return Ok(());
            }
        }
    }
}

pub fn action_result<B: Backend>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    title: &str,
    message: &str,
    good: bool,
) -> io::Result<()> {
    if good || !crate::http::mentions_plan(message) {
        return notice(terminal, ctx, title, message, good);
    }
    let s = ctx.strings();
    let plan = ctx.client.v4_get("/user/info").str_at("/info/subscriptionType");
    let body = format!(
        "{message}\n\n{}\n{} {plan}\n{} {}",
        s.vip_required,
        s.your_plan,
        s.plan_upgrade,
        crate::http::STORE_URL
    );
    notice(terminal, ctx, title, &body, false)
}

pub fn help_overlay<B: Backend>(terminal: &mut Terminal<B>, ctx: &Ctx) -> io::Result<()> {
    let palette = ctx.palette();
    let s = ctx.strings();
    let rows: [(&str, &str); 13] = [
        ("↑ ↓        k j", s.help_move),
        ("↵          →  l", s.help_select),
        ("←          Esc  q", s.help_back),
        ("AvPág  RePág", s.help_page),
        ("Inicio  Fin", s.help_edges),
        (s.help_keys, s.help_actions),
        ("?", s.help_show),
        ("c     Ctrl+C", s.help_copy),
        ("Ctrl+V", s.help_paste),
        (s.help_mouse_key, s.help_mouse),
        ("F2", s.help_theme),
        ("F5     Ctrl+R", s.help_refresh),
        ("Ctrl+C", s.help_quit),
    ];
    loop {
        terminal.draw(|f| {
            let area = f.area();
            let popup = centered(84, (rows.len() as u16) + 5, area);
            f.render_widget(Clear, popup);
            let block = Block::default()
                .title(Line::from(Span::styled(
                    format!(" {} ", s.help_title),
                    palette.title(),
                )))
                .borders(Borders::ALL)
                .border_style(palette.accent_style())
                .style(Style::default().bg(palette.bg_panel));
            let mut lines = vec![Line::from("")];
            for (keys, text) in rows {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {keys:<22}"), palette.key()),
                    Span::styled(text.to_string(), palette.label()),
                ]));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  {}", s.press_key),
                palette.dim(),
            )));
            f.render_widget(
                Paragraph::new(lines).wrap(Wrap { trim: false }).block(block),
                popup,
            );
        })?;
        event::drain();
        if event::read(event::POLL)?.is_some() {
            return Ok(());
        }
    }
}

pub fn centered(percent_x: u16, height: u16, r: Rect) -> Rect {
    let height = height.min(r.height);
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

pub fn field(label: &str, value: &str, palette: &Palette) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {label:<18}"), palette.label()),
        Span::styled(value.to_string(), palette.value()),
    ])
}

pub fn field_styled(label: &str, value: &str, palette: &Palette, style: Style) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {label:<18}"), palette.label()),
        Span::styled(value.to_string(), style),
    ])
}

pub fn section(title: &str, palette: &Palette) -> Line<'static> {
    Line::from(Span::styled(format!("  ── {title} "), palette.accent_style()))
}

pub fn bullet(text: &str, palette: &Palette) -> Line<'static> {
    Line::from(vec![
        Span::styled("    · ", palette.dim()),
        Span::styled(text.to_string(), palette.value()),
    ])
}

pub fn strip_markup(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut inside = false;
    for c in text.chars() {
        match c {
            '<' => inside = true,
            '>' => {
                inside = false;
                out.push(' ');
            }
            _ if !inside => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#039;", "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn wrap_paragraph(text: &str, palette: &Palette) -> Vec<Line<'static>> {
    let clean = strip_markup(text);
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in clean.split(' ') {
        if current.chars().count() + word.chars().count() + 1 > 96 {
            lines.push(Line::from(Span::styled(
                format!("  {current}"),
                palette.dim(),
            )));
            current = word.to_string();
        } else if current.is_empty() {
            current = word.to_string();
        } else {
            current.push(' ');
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("  {current}"),
            palette.dim(),
        )));
    }
    lines
}



pub fn pick_option<B: Backend>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    title: &str,
    options: &[(String, String)],
) -> io::Result<Option<String>> {
    let palette = ctx.palette();
    let mut picker = crate::tui::widgets::picker::Picker::new(
        options
            .iter()
            .map(|(label, _)| crate::tui::widgets::picker::Item::plain(label.clone()))
            .collect(),
    );
    loop {
        let ctx_ref: &Ctx = ctx;
        terminal.draw(|f| {
            let area = f.area();
            paint_background(f, &palette);
            let (title_a, body_a, footer_a) = frame_layout(area);
            render_title(f, title_a, title, &palette);
            let height = (options.len() as u16 + 2).min(body_a.height);
            let inner = centered(46, height, body_a);
            picker.render(f, inner, "", &palette);
            render_bars(f, footer_a, &[], ctx_ref);
        })?;
        let Some(input) = event::read(event::POLL)? else {
            continue;
        };
        match picker.handle(&input) {
            crate::tui::widgets::picker::Outcome::Leave => return Ok(None),
            crate::tui::widgets::picker::Outcome::Chosen(i) => {
                return Ok(options.get(i).map(|(_, value)| value.clone()))
            }
            _ => {}
        }
    }
}

pub fn ask_rating<B: Backend>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    title: &str,
) -> io::Result<Option<u8>> {
    let strings = ctx.strings();
    let options: Vec<(String, String)> = strings
        .rate_scale
        .iter()
        .enumerate()
        .map(|(i, name)| (format!("{:>2}  {name}", i + 1), (i + 1).to_string()))
        .collect();
    Ok(pick_option(terminal, ctx, title, &options)?
        .and_then(|v| v.parse::<u8>().ok()))
}

pub fn vote_bars(chart: &serde_json::Value, ctx: &Ctx) -> Vec<Line<'static>> {
    const KEYS: [&str; 10] = [
        "counterCake",
        "counterVeryEasy",
        "counterEasy",
        "counterTooEasy",
        "counterMedium",
        "counterBitHard",
        "counterHard",
        "counterTooHard",
        "counterExHard",
        "counterBrainFuck",
    ];
    let palette = ctx.palette();
    let strings = ctx.strings();
    let counts: Vec<u64> = KEYS
        .iter()
        .map(|k| chart.get(k).and_then(|v| v.as_u64()).unwrap_or(0))
        .collect();
    let top = counts.iter().copied().max().unwrap_or(0);
    if top == 0 {
        return Vec::new();
    }
    let mut lines = vec![section(strings.votes_title, &palette), Line::from("")];
    for (i, count) in counts.iter().enumerate() {
        let width = ((*count as f64 / top as f64) * 24.0).round() as usize;
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<16}", strings.rate_scale[i]), palette.label()),
            Span::styled("█".repeat(width), palette.accent_style()),
            Span::styled(format!(" {count}"), palette.dim()),
        ]));
    }
    lines
}

pub fn rating_label(data: &serde_json::Value, pointer: &str, ctx: &Ctx) -> String {
    const KEYS: [&str; 10] = [
        "counterCake",
        "counterVeryEasy",
        "counterEasy",
        "counterTooEasy",
        "counterMedium",
        "counterBitHard",
        "counterHard",
        "counterTooHard",
        "counterExHard",
        "counterBrainFuck",
    ];
    let strings = ctx.strings();
    let Some(chart) = data.pointer(pointer) else {
        return "-".to_string();
    };
    let counts: Vec<u64> = KEYS
        .iter()
        .map(|k| chart.get(k).and_then(|v| v.as_u64()).unwrap_or(0))
        .collect();
    let total: u64 = counts.iter().sum();
    if total == 0 {
        return "-".to_string();
    }
    let weighted: u64 = counts
        .iter()
        .enumerate()
        .map(|(i, c)| (i as u64 + 1) * c)
        .sum();
    let average = weighted as f64 / total as f64;
    let index = (average.round() as usize).clamp(1, 10) - 1;
    format!("{} ({total})", strings.rate_scale[index])
}
