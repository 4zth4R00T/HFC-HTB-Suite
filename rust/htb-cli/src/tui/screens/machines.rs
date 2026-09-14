use crate::tui::event::{self, Input};
use crate::tui::screens;
use crate::tui::widgets::input::{ask, confirm, Prompt};
use crate::tui::widgets::picker::{Item, Outcome, Picker};
use crate::tui::Ctx;
use ratatui::backend::Backend;
use ratatui::text::{Line, Span};
use ratatui::Terminal;
use serde_json::{json, Value};
use std::io;

pub fn run<B: Backend>(terminal: &mut Terminal<B>, ctx: &mut Ctx) -> io::Result<()> {
    let mut picker = Picker::new(menu_items(ctx));
    loop {
        let palette = ctx.palette();
        let strings = ctx.strings();
        let ctx_ref: &Ctx = ctx;
        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, strings.machines_title, &palette);
            picker.render(f, body_a, "", &palette);
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
                picker.replace(menu_items(ctx));
                continue;
            }
            _ => {}
        }
        match picker.handle(&input) {
            Outcome::Leave => return Ok(()),
            Outcome::Chosen(i) => {
                match i {
                    0 => browse(terminal, ctx, "active", String::new())?,
                    1 => browse(terminal, ctx, "retired", String::new())?,
                    2 => {
                        let term = ask(
                            terminal,
                            &palette,
                            Prompt::new(strings.prompt_search, strings.prompt_search_label)
                                .hint(strings.prompt_search_hint).paste_hint(strings.paste_hint),
                        )?;
                        if let Some(term) = term {
                            if !term.trim().is_empty() {
                                browse(terminal, ctx, "", term.trim().to_string())?;
                            }
                        }
                    }
                    3 => target(terminal, ctx)?,
                    4 => starting_point(terminal, ctx)?,
                    _ => {}
                }
                picker.replace(menu_items(ctx));
            }
            _ => {}
        }
    }
}

fn starting_point<B: Backend>(terminal: &mut Terminal<B>, ctx: &mut Ctx) -> io::Result<()> {
    let tiers = {
        let palette = ctx.palette();
        let strings = ctx.strings();
        screens::loading(terminal, &palette, strings.loading)?;
        ctx.client.v4_get("/sp/tiers/progress").array_at("/data")
    };
    if tiers.is_empty() {
        let strings = ctx.strings();
        return screens::notice(
            terminal,
            ctx,
            strings.machines_starting_point,
            strings.no_results,
            false,
        );
    }

    let mut picker = Picker::new(tier_items(&tiers, ctx));
    loop {
        let palette = ctx.palette();
        let strings = ctx.strings();
        let ctx_ref: &Ctx = ctx;
        let position = picker.position_label();
        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, strings.machines_starting_point, &palette);
            picker.render(f, body_a, &position, &palette);
            screens::render_bars(f, footer_a, &[], ctx_ref);
        })?;
        let Some(input) = event::read(event::POLL)? else {
            continue;
        };
        if matches!(input, Input::Help) {
            screens::help_overlay(terminal, ctx)?;
            continue;
        }
        match picker.handle(&input) {
            Outcome::Leave => return Ok(()),
            Outcome::Chosen(i) => {
                if let Some(t) = tiers.get(i) {
                    let id = t.get("id").and_then(|v| v.as_u64()).unwrap_or(1);
                    let name = t
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Starting Point")
                        .to_string();
                    sp_tier_machines(terminal, ctx, id, &name)?;
                }
            }
            _ => {}
        }
    }
}

fn tier_items(tiers: &[Value], ctx: &Ctx) -> Vec<Item> {
    let p = ctx.palette();
    tiers
        .iter()
        .map(|t| {
            let pct = t
                .get("completion_percentage")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let filled = ((pct as f64 / 100.0) * 20.0).round() as usize;
            Item {
                title: Line::from(vec![
                    Span::styled(
                        format!("{:<10}", t.get("name").and_then(|v| v.as_str()).unwrap_or("-")),
                        p.value(),
                    ),
                    Span::styled("█".repeat(filled), p.accent_style()),
                    Span::styled("░".repeat(20 - filled), p.dim()),
                    Span::styled(format!("  {pct}%"), p.label()),
                ]),
                detail: Some(Line::from(Span::styled(
                    format!(
                        "      {}",
                        t.get("description").and_then(|v| v.as_str()).unwrap_or("")
                    ),
                    p.dim(),
                ))),
            }
        })
        .collect()
}

fn sp_tier_machines<B: Backend>(
    terminal: &mut Terminal<B>,
    ctx: &mut Ctx,
    tier: u64,
    tier_name: &str,
) -> io::Result<()> {
    let mut rows = {
        let palette = ctx.palette();
        let strings = ctx.strings();
        screens::loading(terminal, &palette, strings.loading)?;
        ctx.client
            .v5_get(&format!("/machines?spTier={tier}"))
            .array_at("/data")
    };
    if rows.is_empty() {
        let strings = ctx.strings();
        return screens::notice(terminal, ctx, tier_name, strings.no_results, false);
    }

    let mut picker = Picker::new(rows.iter().map(|m| sp_item(m, ctx)).collect());
    loop {
        let palette = ctx.palette();
        let strings = ctx.strings();
        let title = format!("{} · {tier_name} · {}", strings.machines_starting_point, rows.len());
        let ctx_ref: &Ctx = ctx;
        let actions: Vec<(&str, &str)> = vec![
            ("s", strings.action_spawn),
            ("f", strings.action_flag),
            ("t", strings.action_stop),
            ("c", strings.action_copy),
        ];
        let position = picker.position_label();
        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, &title, &palette);
            picker.render(f, body_a, &position, &palette);
            screens::render_bars(f, footer_a, &actions, ctx_ref);
        })?;

        let Some(input) = event::read(event::POLL)? else {
            continue;
        };
        let selected = rows.get(picker.selected()).cloned();
        let mut reload = false;
        match &input {
            Input::Quit => return Ok(()),
            Input::Help => {
                screens::help_overlay(terminal, ctx)?;
                continue;
            }
            Input::Key('s') | Input::Key('S') => {
                if let Some(m) = selected {
                    spawn(terminal, ctx, &id_of(&m))?;
                }
                continue;
            }
            Input::Key('f') | Input::Key('F') => {
                if let Some(m) = selected {
                    submit_flag(terminal, ctx, &id_of(&m))?;
                    reload = true;
                }
            }
            Input::Key('t') | Input::Key('T') => {
                stop(terminal, ctx)?;
                continue;
            }
            Input::Copy | Input::Key('c') | Input::Key('C') => {
                let ip = ctx.client.v4_get("/machine/active").str_at("/info/ip");
                screens::copy_value(terminal, ctx, &ip)?;
                continue;
            }
            _ => {}
        }
        if reload {
            ctx.client.forget_cache();
            let keep = picker.selected();
            rows = ctx
                .client
                .v5_get(&format!("/machines?spTier={tier}"))
                .array_at("/data");
            picker.replace(rows.iter().map(|m| sp_item(m, ctx)).collect());
            picker.select(keep);
            continue;
        }
        match picker.handle(&input) {
            Outcome::Leave => return Ok(()),
            Outcome::Chosen(i) => {
                if let Some(m) = rows.get(i) {
                    details(terminal, ctx, &id_of(m))?;
                }
            }
            _ => {}
        }
    }
}

fn load_tasks<B: Backend>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    id: &str,
) -> io::Result<Vec<Value>> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    screens::loading(terminal, &palette, strings.loading)?;
    let guided = ctx.client.v4_get(&format!("/machines/{id}/tasks"));
    let rows = guided.array_at("/data");
    if !rows.is_empty() {
        return Ok(rows);
    }
    Ok(ctx
        .client
        .v4_get(&format!("/machines/{id}/adventure"))
        .array_at("/data"))
}

fn task_item(t: &Value, ctx: &Ctx) -> Item {
    let p = ctx.palette();
    let s = ctx.strings();
    let done = t.get("completed").and_then(|v| v.as_bool()).unwrap_or(false);
    let kind = t
        .pointer("/type/text")
        .and_then(|v| v.as_str())
        .unwrap_or("task");
    let title = match kind {
        "user" | "root" => format!("{} {}", kind.to_uppercase(), text_of(t, "title")),
        _ => text_of(t, "title"),
    };
    let raw_desc = t
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let mut question = screens::strip_markup(&raw_desc);
    if question.chars().count() > 96 {
        question = question.chars().take(96).collect::<String>() + "…";
    }
    let answer = t.get("flag").and_then(|v| v.as_str()).unwrap_or("");
    let detail = if done && !answer.is_empty() {
        format!("      {} {answer}", s.task_answer)
    } else {
        format!("      {} {}", s.task_expected, text_of(t, "masked_flag"))
    };
    Item {
        title: Line::from(vec![
            Span::styled(
                if done { "✓ " } else { "  " }.to_string(),
                if done { p.ok() } else { p.dim() },
            ),
            Span::styled(format!("{title:<22}"), p.label()),
            Span::styled(question, p.value()),
        ]),
        detail: Some(Line::from(Span::styled(detail, p.dim()))),
    }
}

fn answer_task<B: Backend>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    id: &str,
    task: &Value,
) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    let hint = screens::strip_markup(&text_of(task, "hint"));
    let label = screens::strip_markup(&text_of(task, "description"));
    let Some(answer) = ask(
        terminal,
        &palette,
        Prompt::new(strings.tasks_title, &label)
            .hint(&hint)
            .paste_hint(strings.paste_hint),
    )?
    else {
        return Ok(());
    };
    if answer.trim().is_empty() {
        return Ok(());
    }
    let task_id = task
        .get("id")
        .and_then(|v| v.as_i64())
        .map(|v| v.to_string())
        .unwrap_or_default();
    screens::loading(terminal, &palette, strings.loading)?;
    let res = if task_id.is_empty() {
        ctx.client.v5_post(
            "/machine/own",
            json!({ "flag": answer.trim(), "id": id.parse::<i64>().unwrap_or(0), "difficulty": 5 }),
        )
    } else {
        ctx.client.v4_post(
            &format!("/machines/{id}/tasks/{task_id}/flag"),
            json!({ "flag": answer.trim() }),
        )
    };
    let msg = res.message();
    let (title, good) = match crate::http::read_flag_answer(&msg) {
        crate::http::Verdict::Accepted => (strings.task_accepted_m, true),
        crate::http::Verdict::Rejected => (strings.task_rejected_m, false),
        crate::http::Verdict::Unclear => (strings.result_title, res.ok),
    };
    screens::notice(terminal, ctx, title, &msg, good)
}

fn text_of(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("-")
        .to_string()
}

fn task_progress(m: &Value) -> Option<u64> {
    let raw = m.get("taskCompletionPercentage")?;
    let pct = match raw {
        Value::Number(n) => n.as_u64()?,
        Value::String(t) => t.trim_end_matches('%').parse::<u64>().ok()?,
        _ => return None,
    };
    Some(pct.min(100))
}

fn progress_span(m: &Value, ctx: &Ctx) -> Span<'static> {
    let p = ctx.palette();
    let s = ctx.strings();
    match task_progress(m) {
        Some(pct) => {
            let filled = (pct as usize * 10) / 100;
            let bar: String = "█".repeat(filled) + &"░".repeat(10 - filled);
            let style = if pct == 100 { p.ok() } else { p.label() };
            Span::styled(format!("{} {bar} {pct:>3}%", s.tasks_short), style)
        }
        None => Span::styled(String::new(), p.dim()),
    }
}

fn sp_item(m: &Value, ctx: &Ctx) -> Item {
    let p = ctx.palette();
    let user = m.get("authUserInUserOwns").and_then(|v| v.as_bool()).unwrap_or(false);
    let root = m.get("authUserInRootOwns").and_then(|v| v.as_bool()).unwrap_or(false);
    let mark = match (user, root) {
        (true, true) => "✓✓",
        (true, false) | (false, true) => "✓ ",
        _ => "  ",
    };
    let diff = text_of(m, "difficultyText");
    Item {
        title: Line::from(vec![
            Span::styled(format!("{mark} "), if root { p.ok() } else { p.dim() }),
            Span::styled(format!("{:<22}", text_of(m, "name")), p.value()),
            Span::styled(format!("{:<9}", text_of(m, "os")), p.label()),
            Span::styled(format!("{diff:<11}"), p.difficulty(&diff)),
            progress_span(m, ctx),
        ]),
        detail: None,
    }
}

fn menu_items(ctx: &Ctx) -> Vec<Item> {
    let s = ctx.strings();
    let p = ctx.palette();
    [
        (s.machines_active, s.machines_active_desc),
        (s.machines_retired, s.machines_retired_desc),
        (s.machines_search, s.machines_search_desc),
        (s.machines_target, s.machines_target_desc),
        (s.machines_starting_point, s.machines_starting_point_desc),
    ]
    .iter()
    .map(|(t, d)| {
        Item::with_detail(
            Line::from(Span::styled(t.to_string(), p.value())),
            Line::from(Span::styled(format!("    {d}"), p.dim())),
        )
    })
    .collect()
}

struct Query {
    state: String,
    keyword: String,
    difficulty: String,
    os: String,
    solved: String,
    page: u32,
}

fn difficulty_options(ctx: &Ctx) -> Vec<(String, String)> {
    let s = ctx.strings();
    vec![
        (s.filter_any.to_string(), String::new()),
        ("Very Easy".to_string(), "very-easy".to_string()),
        ("Easy".to_string(), "easy".to_string()),
        ("Medium".to_string(), "medium".to_string()),
        ("Hard".to_string(), "hard".to_string()),
        ("Insane".to_string(), "insane".to_string()),
    ]
}

fn os_options(ctx: &Ctx) -> Vec<(String, String)> {
    let s = ctx.strings();
    vec![
        (s.filter_any.to_string(), String::new()),
        ("Linux".to_string(), "linux".to_string()),
        ("Windows".to_string(), "windows".to_string()),
        ("FreeBSD".to_string(), "freebsd".to_string()),
        ("OpenBSD".to_string(), "openbsd".to_string()),
        ("Android".to_string(), "android".to_string()),
        ("Other".to_string(), "other".to_string()),
    ]
}

fn solved_options(ctx: &Ctx) -> Vec<(String, String)> {
    let s = ctx.strings();
    vec![
        (s.filter_any.to_string(), String::new()),
        (s.filter_solved.to_string(), "solved".to_string()),
        (s.filter_unsolved.to_string(), "unsolved".to_string()),
    ]
}

fn is_owned(m: &Value) -> bool {
    m.get("authUserInUserOwns").and_then(|v| v.as_bool()).unwrap_or(false)
        || m.get("authUserInRootOwns").and_then(|v| v.as_bool()).unwrap_or(false)
}

fn label_for(options: &[(String, String)], value: &str) -> String {
    options
        .iter()
        .find(|(_, v)| v == value)
        .map(|(label, _)| label.clone())
        .unwrap_or_else(|| value.to_string())
}

impl Query {
    fn to_string(&self) -> String {
        let mut qs = format!("per_page=25&page={}", self.page);
        if !self.state.is_empty() {
            qs += &format!("&state={}", self.state);
        }
        if !self.keyword.is_empty() {
            qs += &format!("&keyword={}", urlencode(&self.keyword));
        }
        if !self.difficulty.is_empty() {
            qs += &format!("&difficulty[]={}", urlencode(&self.difficulty));
        }
        if !self.os.is_empty() {
            qs += &format!("&os[]={}", urlencode(&self.os));
        }
        qs
    }
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b' ' => "+".to_string(),
            other => format!("%{other:02X}"),
        })
        .collect()
}

fn machine_item(m: &Value, ctx: &Ctx) -> Item {
    let p = ctx.palette();
    let s = ctx.strings();
    let name = m.get("name").and_then(|v| v.as_str()).unwrap_or("?");
    let os = m.get("os").and_then(|v| v.as_str()).unwrap_or("?");
    let diff = m
        .get("difficultyText")
        .and_then(|v| v.as_str())
        .or_else(|| m.get("difficulty").and_then(|v| v.as_str()))
        .unwrap_or("?");
    let pts = number(m, "points");
    let owns = number(m, "userOwnsCount");
    let free = m.get("free").and_then(|v| v.as_bool()).unwrap_or(false);
    let user_own = m
        .get("authUserInUserOwns")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let root_own = m
        .get("authUserInRootOwns")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let (mark, mark_style) = match (user_own, root_own) {
        (true, true) => ("✓✓", p.ok()),
        (true, false) | (false, true) => ("✓ ", p.warn()),
        _ => ("  ", p.dim()),
    };
    let released: String = m
        .get("release")
        .and_then(|v| v.as_str())
        .or_else(|| m.get("releaseDate").and_then(|v| v.as_str()))
        .unwrap_or("")
        .chars()
        .take(10)
        .collect();

    let title = Line::from(vec![
        Span::styled(mark.to_string(), mark_style),
        Span::styled(format!(" {name:<26}"), p.value()),
        Span::styled(format!("{os:<8}"), p.info_style()),
        Span::styled(format!("{diff:<9}"), p.difficulty(diff)),
        Span::styled(format!("{pts:>4} pts"), p.warn()),
    ]);
    let mut detail = vec![Span::styled(
        format!(
            "      {} {owns} · {} {} · {released}",
            s.field_owns,
            s.field_access,
            if free { "Free" } else { "VIP" }
        ),
        p.dim(),
    )];
    if task_progress(m).is_some() {
        detail.push(Span::styled(" · ".to_string(), p.dim()));
        detail.push(progress_span(m, ctx));
    }
    Item::with_detail(title, Line::from(detail))
}

fn number(v: &Value, key: &str) -> String {
    v.get(key)
        .map(|x| {
            x.as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| x.to_string())
        })
        .unwrap_or_else(|| "0".into())
}

fn browse<B: Backend>(
    terminal: &mut Terminal<B>,
    ctx: &mut Ctx,
    state: &str,
    keyword: String,
) -> io::Result<()> {
    let mut query = Query {
        state: state.to_string(),
        keyword,
        difficulty: String::new(),
        os: String::new(),
        solved: String::new(),
        page: 1,
    };
    let mut data = fetch(terminal, ctx, &query)?;
    let mut picker = Picker::new(data.rows.iter().map(|m| machine_item(m, ctx)).collect());

    loop {
        let palette = ctx.palette();
        let strings = ctx.strings();
        let title = screen_title(ctx, &query, &data);
        let ctx_ref: &Ctx = ctx;
        let actions: Vec<(&str, &str)> = vec![
            ("s", strings.action_spawn),
            ("f", strings.action_flag),
            ("t", strings.action_stop),
            ("r", strings.action_reset),
            ("e", strings.action_extend),
            ("n", strings.page_next),
            ("p", strings.page_previous),
            ("d", strings.action_filter_difficulty),
            ("o", strings.action_filter_os),
            ("v", strings.action_filter_solved),
            ("x", strings.action_clear_filters),
            ("c", strings.action_copy),
        ];
        let position = picker.position_label();
        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, &title, &palette);
            picker.render(f, body_a, &position, &palette);
            screens::render_bars(f, footer_a, &actions, ctx_ref);
        })?;

        let Some(input) = event::read(event::POLL)? else {
            continue;
        };

        let mut reload = false;
        match &input {
            Input::Quit => return Ok(()),
            Input::Help => {
                screens::help_overlay(terminal, ctx)?;
                continue;
            }
            Input::ThemeToggle => {
                ctx.flip_theme();
                picker.replace(data.rows.iter().map(|m| machine_item(m, ctx)).collect());
                continue;
            }
            Input::Refresh => {
                ctx.client.forget_cache();
                reload = true;
            }
            Input::Copy | Input::Key('c') | Input::Key('C') => {
                let ip = ctx.client.v4_get("/machine/active").str_at("/info/ip");
                screens::copy_value(terminal, ctx, &ip)?;
            }
            Input::Key(c) => {
                let selected = data.rows.get(picker.selected()).cloned();
                match c {
                    'n' | 'N' => {
                        if query.page < data.last_page {
                            query.page += 1;
                            reload = true;
                        }
                    }
                    'p' | 'P' => {
                        if query.page > 1 {
                            query.page -= 1;
                            reload = true;
                        }
                    }
                    'd' | 'D' => {
                        let options = difficulty_options(ctx);
                        if let Some(v) = screens::pick_option(
                            terminal,
                            ctx,
                            strings.action_filter_difficulty,
                            &options,
                        )? {
                            query.difficulty = v;
                            query.page = 1;
                            reload = true;
                        }
                    }
                    'o' | 'O' => {
                        let options = os_options(ctx);
                        if let Some(v) =
                            screens::pick_option(terminal, ctx, strings.action_filter_os, &options)?
                        {
                            query.os = v;
                            query.page = 1;
                            reload = true;
                        }
                    }
                    'v' | 'V' => {
                        let options = solved_options(ctx);
                        if let Some(v) = screens::pick_option(
                            terminal,
                            ctx,
                            strings.action_filter_solved,
                            &options,
                        )? {
                            query.solved = v;
                            reload = true;
                        }
                    }
                    'x' | 'X' => {
                        query.difficulty.clear();
                        query.os.clear();
                        query.solved.clear();
                        query.page = 1;
                        reload = true;
                    }
                    's' | 'S' => {
                        if let Some(m) = selected {
                            spawn(terminal, ctx, &id_of(&m))?;
                            reload = true;
                        }
                    }
                    'f' | 'F' => {
                        if let Some(m) = selected {
                            submit_flag(terminal, ctx, &id_of(&m))?;
                            reload = true;
                        }
                    }
                    't' | 'T' => {
                        stop(terminal, ctx)?;
                        reload = true;
                    }
                    'r' | 'R' => {
                        reset(terminal, ctx)?;
                    }
                    'e' | 'E' => {
                        extend(terminal, ctx)?;
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        if !reload {
            match picker.handle(&input) {
                Outcome::Leave => return Ok(()),
                Outcome::Chosen(i) => {
                    if let Some(m) = data.rows.get(i) {
                        details(terminal, ctx, &id_of(m))?;
                    }
                }
                _ => {}
            }
        }

        if reload {
            let keep = picker.selected();
            data = fetch(terminal, ctx, &query)?;
            picker.replace(data.rows.iter().map(|m| machine_item(m, ctx)).collect());
            picker.select(keep);
        }
    }
}

struct Page {
    rows: Vec<Value>,
    total: String,
    last_page: u32,
}

fn fetch<B: Backend>(terminal: &mut Terminal<B>, ctx: &Ctx, query: &Query) -> io::Result<Page> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    screens::loading(terminal, &palette, strings.loading)?;
    let r = ctx.client.v5_get(&format!("/machines?{}", query.to_string()));
    let rows: Vec<Value> = r
        .array_at("/data")
        .into_iter()
        .filter(|m| match query.solved.as_str() {
            "solved" => is_owned(m),
            "unsolved" => !is_owned(m),
            _ => true,
        })
        .collect();
    Ok(Page {
        rows,
        total: r.str_at("/meta/total"),
        last_page: r
            .data
            .pointer("/meta/last_page")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as u32,
    })
}

fn screen_title(ctx: &Ctx, query: &Query, data: &Page) -> String {
    let s = ctx.strings();
    let base = if !query.keyword.is_empty() {
        format!("{}: {}", s.machines_search, query.keyword)
    } else if query.state == "retired" {
        s.machines_retired.to_string()
    } else {
        s.machines_active.to_string()
    };
    let mut extra = String::new();
    if !query.difficulty.is_empty() {
        extra += &format!(" · {}", label_for(&difficulty_options(ctx), &query.difficulty));
    }
    if !query.os.is_empty() {
        extra += &format!(" · {}", label_for(&os_options(ctx), &query.os));
    }
    if !query.solved.is_empty() {
        extra += &format!(" · {}", label_for(&solved_options(ctx), &query.solved));
    }
    let count = if query.solved.is_empty() {
        data.total.clone()
    } else {
        data.rows.len().to_string()
    };
    format!(
        "{base}{extra} · {count} · {} {}/{}",
        s.action_page, query.page, data.last_page
    )
}

fn id_of(m: &Value) -> String {
    m.get("id")
        .map(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| v.to_string())
        })
        .unwrap_or_default()
}

fn target<B: Backend>(terminal: &mut Terminal<B>, ctx: &mut Ctx) -> io::Result<()> {
    let mut act = {
        let palette = ctx.palette();
        let strings = ctx.strings();
        screens::loading(terminal, &palette, strings.loading)?;
        ctx.client.v4_get("/machine/active")
    };
    if act.str_at("/info/name") == "-" {
        let strings = ctx.strings();
        return screens::notice(
            terminal,
            ctx,
            strings.machines_target,
            strings.no_active_machine,
            false,
        );
    }
    let mut id = act.str_at("/info/id");
    let mut rows = load_tasks(terminal, ctx, &id)?;
    let mut picker = Picker::new(rows.iter().map(|t| task_item(t, ctx)).collect());

    loop {
        let palette = ctx.palette();
        let strings = ctx.strings();
        let name = act.str_at("/info/name");
        let header = vec![
            Line::from(Span::styled(format!("  {name}"), palette.title())),
            Line::from(""),
            screens::field("ID", &id, &palette),
            screens::field_styled(
                strings.field_ip,
                &act.str_at("/info/ip"),
                &palette,
                palette.info_style(),
            ),
            screens::field_styled(
                strings.field_expires,
                &act.str_at("/info/expires_at"),
                &palette,
                palette.warn(),
            ),
        ];
        let mut header = header;
        push_progress(&mut header, &rows, ctx);
        let list_title = if rows.is_empty() {
            strings.no_tasks.to_string()
        } else {
            strings.tasks_title.to_string()
        };
        let mut actions: Vec<(&str, &str)> = Vec::new();
        if !rows.is_empty() {
            actions.push(("↵", strings.task_answer));
        }
        actions.extend([
            ("f", strings.action_flag),
            ("t", strings.action_stop),
            ("r", strings.action_reset),
            ("e", strings.action_extend),
            ("c", strings.action_copy),
        ]);
        let ctx_ref: &Ctx = ctx;
        let position = picker.position_label();
        let header_height = header.len() as u16 + 2;

        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, strings.machines_target, &palette);
            let split = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    ratatui::layout::Constraint::Length(header_height),
                    ratatui::layout::Constraint::Min(3),
                ])
                .split(body_a);
            let block = ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .border_style(palette.border_style());
            f.render_widget(
                ratatui::widgets::Paragraph::new(header.clone()).block(block),
                split[0],
            );
            picker.render(f, split[1], &format!("{list_title} {position}"), &palette);
            screens::render_bars(f, footer_a, &actions, ctx_ref);
        })?;

        let Some(input) = event::read(event::POLL)? else {
            continue;
        };
        let mut reload = false;
        match &input {
            Input::Quit => return Ok(()),
            Input::Help => {
                screens::help_overlay(terminal, ctx)?;
                continue;
            }
            Input::ThemeToggle => {
                ctx.flip_theme();
                picker.replace(rows.iter().map(|t| task_item(t, ctx)).collect());
                continue;
            }
            Input::Refresh => reload = true,
            Input::Key('f') | Input::Key('F') => {
                submit_flag(terminal, ctx, &id)?;
                reload = true;
            }
            Input::Key('t') | Input::Key('T') => {
                stop(terminal, ctx)?;
                reload = true;
            }
            Input::Key('r') | Input::Key('R') => {
                reset(terminal, ctx)?;
                reload = true;
            }
            Input::Key('e') | Input::Key('E') => {
                extend(terminal, ctx)?;
                reload = true;
            }
            Input::Copy | Input::Key('c') | Input::Key('C') => {
                let value = act.str_at("/info/ip");
                screens::copy_value(terminal, ctx, &value)?;
                continue;
            }
            _ => {}
        }
        if !reload {
            match picker.handle(&input) {
                Outcome::Leave => return Ok(()),
                Outcome::Chosen(i) => {
                    if let Some(task) = rows.get(i).cloned() {
                        answer_task(terminal, ctx, &id, &task)?;
                        reload = true;
                    }
                }
                _ => {}
            }
        }
        if reload {
            ctx.client.forget_cache();
            let keep = picker.selected();
            let palette = ctx.palette();
            let strings = ctx.strings();
            screens::loading(terminal, &palette, strings.loading)?;
            act = ctx.client.v4_get("/machine/active");
            if act.str_at("/info/name") == "-" {
                return screens::notice(
                    terminal,
                    ctx,
                    strings.machines_target,
                    strings.no_active_machine,
                    false,
                );
            }
            id = act.str_at("/info/id");
            rows = load_tasks(terminal, ctx, &id)?;
            picker.replace(rows.iter().map(|t| task_item(t, ctx)).collect());
            picker.select(keep);
        }
    }
}

fn push_progress(header: &mut Vec<Line<'static>>, rows: &[Value], ctx: &Ctx) {
    if rows.is_empty() {
        return;
    }
    let palette = ctx.palette();
    let strings = ctx.strings();
    let done = rows
        .iter()
        .filter(|t| t.get("completed").and_then(|v| v.as_bool()).unwrap_or(false))
        .count();
    let style = if done == rows.len() {
        palette.ok()
    } else {
        palette.warn()
    };
    header.push(screens::field_styled(
        strings.field_progress,
        &format!("{done}/{} {}", rows.len(), strings.tasks_answered),
        &palette,
        style,
    ));
}

fn machine_header(raw: &crate::http::ApiResponse, id: &str, ctx: &Ctx) -> Vec<Line<'static>> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    let name = raw.str_at("/info/name");
    let diff = raw.str_at("/info/difficultyText");
    let ip = raw.str_at("/info/playInfo/ip");
    let mut lines = vec![
        Line::from(Span::styled(format!("  {name}"), palette.title())),
        Line::from(""),
        screens::field("ID", id, &palette),
        screens::field(strings.field_os, &raw.str_at("/info/os"), &palette),
        screens::field_styled(
            strings.field_difficulty,
            &diff,
            &palette,
            palette.difficulty(&diff),
        ),
        screens::field_styled(
            strings.field_points,
            &raw.str_at("/info/points"),
            &palette,
            palette.warn(),
        ),
        screens::field_styled(strings.field_ip, &ip, &palette, palette.info_style()),
        screens::field(
            strings.field_owns,
            &if raw
                .data
                .pointer("/info/isSingleFlag")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                raw.str_at("/info/root_owns_count")
            } else {
                format!(
                    "user {} · root {}",
                    raw.str_at("/info/user_owns_count"),
                    raw.str_at("/info/root_owns_count")
                )
            },
            &palette,
        ),
        screens::field(strings.field_rating, &raw.str_at("/info/stars"), &palette),
        screens::field(
            strings.field_user_rating,
            &screens::rating_label(&raw.data, "/info/feedbackForChart", ctx),
            &palette,
        ),
        screens::field(
            strings.field_released,
            &raw.str_at("/info/release").chars().take(10).collect::<String>(),
            &palette,
        ),
    ];
    if raw.has("/info/maker/name") {
        lines.push(screens::field(
            strings.field_author,
            &raw.str_at("/info/maker/name"),
            &palette,
        ));
    }
    lines
}

fn details<B: Backend>(terminal: &mut Terminal<B>, ctx: &mut Ctx, id: &str) -> io::Result<()> {
    let mut raw = {
        let palette = ctx.palette();
        let strings = ctx.strings();
        screens::loading(terminal, &palette, strings.loading)?;
        ctx.client.v4_get(&format!("/machine/profile/{id}"))
    };
    let name = raw.str_at("/info/name");
    let mut rows = load_tasks(terminal, ctx, id)?;
    let mut header = machine_header(&raw, id, ctx);
    push_progress(&mut header, &rows, ctx);
    let mut picker = Picker::new(rows.iter().map(|t| task_item(t, ctx)).collect());

    loop {
        let palette = ctx.palette();
        let strings = ctx.strings();
        let list_title = if rows.is_empty() {
            strings.no_tasks.to_string()
        } else {
            strings.tasks_title.to_string()
        };
        let mut actions: Vec<(&str, &str)> = vec![("s", strings.action_spawn)];
        if !rows.is_empty() {
            actions.push(("↵", strings.task_answer));
        }
        actions.extend([
            ("f", strings.action_flag),
            ("t", strings.action_stop),
            ("r", strings.action_reset),
            ("e", strings.action_extend),
            ("v", strings.votes_title),
            ("c", strings.action_copy),
        ]);
        let ctx_ref: &Ctx = ctx;
        let position = picker.position_label();
        let header_height = (header.len() as u16 + 2).min(17);
        let header_now = header.clone();
        let name_now = name.clone();

        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, &name_now, &palette);
            let split = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    ratatui::layout::Constraint::Length(header_height),
                    ratatui::layout::Constraint::Min(3),
                ])
                .split(body_a);
            let block = ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .border_style(palette.border_style());
            f.render_widget(
                ratatui::widgets::Paragraph::new(header_now.clone()).block(block),
                split[0],
            );
            picker.render(f, split[1], &format!("{list_title} {position}"), &palette);
            screens::render_bars(f, footer_a, &actions, ctx_ref);
        })?;

        let Some(input) = event::read(event::POLL)? else {
            continue;
        };
        let mut reload = false;
        match &input {
            Input::Quit => return Ok(()),
            Input::Help => {
                screens::help_overlay(terminal, ctx)?;
                continue;
            }
            Input::ThemeToggle => {
                ctx.flip_theme();
                header = machine_header(&raw, id, ctx);
                push_progress(&mut header, &rows, ctx);
                picker.replace(rows.iter().map(|t| task_item(t, ctx)).collect());
                continue;
            }
            Input::Refresh => reload = true,
            Input::Key('s') | Input::Key('S') => {
                spawn(terminal, ctx, id)?;
                reload = true;
            }
            Input::Key('f') | Input::Key('F') => {
                submit_flag(terminal, ctx, id)?;
                reload = true;
            }
            Input::Key('t') | Input::Key('T') => {
                stop(terminal, ctx)?;
                reload = true;
            }
            Input::Key('r') | Input::Key('R') => {
                reset(terminal, ctx)?;
                reload = true;
            }
            Input::Key('e') | Input::Key('E') => {
                extend(terminal, ctx)?;
                reload = true;
            }
            Input::Key('v') | Input::Key('V') => {
                let bars = raw
                    .data
                    .pointer("/info/feedbackForChart")
                    .map(|c| screens::vote_bars(c, ctx))
                    .unwrap_or_default();
                let strings = ctx.strings();
                if bars.is_empty() {
                    screens::notice(terminal, ctx, strings.votes_title, strings.no_results, false)?;
                } else {
                    let mut body = vec![Line::from("")];
                    body.extend(bars);
                    screens::page(terminal, ctx, strings.votes_title, body, &screens::nav_text(ctx))?;
                }
                continue;
            }
            Input::Copy | Input::Key('c') | Input::Key('C') => {
                let value = raw.str_at("/info/playInfo/ip");
                screens::copy_value(terminal, ctx, &value)?;
                continue;
            }
            _ => {}
        }
        if !reload {
            match picker.handle(&input) {
                Outcome::Leave => return Ok(()),
                Outcome::Chosen(i) => {
                    if let Some(task) = rows.get(i).cloned() {
                        answer_task(terminal, ctx, id, &task)?;
                        reload = true;
                    }
                }
                _ => {}
            }
        }
        if reload {
            ctx.client.forget_cache();
            let keep = picker.selected();
            let palette = ctx.palette();
            let strings = ctx.strings();
            screens::loading(terminal, &palette, strings.loading)?;
            raw = ctx.client.v4_get(&format!("/machine/profile/{id}"));
            rows = load_tasks(terminal, ctx, id)?;
            header = machine_header(&raw, id, ctx);
            push_progress(&mut header, &rows, ctx);
            picker.replace(rows.iter().map(|t| task_item(t, ctx)).collect());
            picker.select(keep);
        }
    }
}

fn spawn<B: Backend>(terminal: &mut Terminal<B>, ctx: &Ctx, id: &str) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    let active = ctx.client.v4_get("/machine/active");
    if active.str_at("/info/name") != "-" {
        let current = active.str_at("/info/name");
        if !confirm(
            terminal,
            &palette,
            strings.confirm_title,
            &format!("{current} — {}", strings.confirm_replace_machine),
            strings.confirm_yes_hint,
        )? {
            return Ok(());
        }
        let aid = active.str_at("/info/id");
        ctx.client.v4_post(
            "/vm/terminate",
            json!({ "machine_id": aid.parse::<i64>().unwrap_or(0) }),
        );
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    screens::loading(terminal, &palette, strings.loading)?;
    let res = ctx.client.v4_post(
        "/vm/spawn",
        json!({ "machine_id": id.parse::<i64>().unwrap_or(0) }),
    );
    let msg = res.message();
    let good = crate::http::read_action_answer(&msg, res.ok) == crate::http::Verdict::Accepted;
    if !good {
        return screens::action_result(terminal, ctx, strings.action_spawn, &msg, false);
    }
    screens::loading(terminal, &palette, strings.spawn_waiting)?;
    let mut ip = String::from("-");
    for _ in 0..10 {
        std::thread::sleep(std::time::Duration::from_secs(3));
        ip = ctx.client.v4_get("/machine/active").str_at("/info/ip");
        if ip != "-" && !ip.is_empty() {
            break;
        }
    }
    let detail = if ip == "-" || ip.is_empty() {
        format!("{msg}\n{}", strings.spawn_no_ip_yet)
    } else {
        format!("{msg}\nIP: {ip}")
    };
    screens::notice(terminal, ctx, strings.action_spawn, &detail, true)
}

fn stop<B: Backend>(terminal: &mut Terminal<B>, ctx: &Ctx) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    let act = ctx.client.v4_get("/machine/active");
    if act.str_at("/info/name") == "-" {
        return screens::notice(
            terminal,
            ctx,
            strings.action_stop,
            strings.no_active_machine,
            false,
        );
    }
    if !confirm(
        terminal,
        &palette,
        strings.confirm_title,
        strings.confirm_stop,
        strings.confirm_yes_hint,
    )? {
        return Ok(());
    }
    let aid = act.str_at("/info/id");
    let res = ctx.client.v4_post(
        "/vm/terminate",
        json!({ "machine_id": aid.parse::<i64>().unwrap_or(0) }),
    );
    let msg = res.message();
    let good = crate::http::read_action_answer(&msg, res.ok) == crate::http::Verdict::Accepted;
    screens::action_result(terminal, ctx, strings.action_stop, &msg, good)
}

fn reset<B: Backend>(terminal: &mut Terminal<B>, ctx: &Ctx) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    let act = ctx.client.v4_get("/machine/active");
    if act.str_at("/info/name") == "-" {
        return screens::notice(
            terminal,
            ctx,
            strings.action_reset,
            strings.no_active_machine,
            false,
        );
    }
    if !confirm(
        terminal,
        &palette,
        strings.confirm_title,
        strings.confirm_reset,
        strings.confirm_yes_hint,
    )? {
        return Ok(());
    }
    let aid = act.str_at("/info/id");
    let res = ctx.client.v4_post(
        "/vm/reset",
        json!({ "machine_id": aid.parse::<i64>().unwrap_or(0) }),
    );
    let msg = res.message();
    let good = crate::http::read_action_answer(&msg, res.ok) == crate::http::Verdict::Accepted;
    screens::action_result(terminal, ctx, strings.action_reset, &msg, good)
}

fn extend<B: Backend>(terminal: &mut Terminal<B>, ctx: &Ctx) -> io::Result<()> {
    let strings = ctx.strings();
    let act = ctx.client.v4_get("/machine/active");
    if act.str_at("/info/name") == "-" {
        return screens::notice(
            terminal,
            ctx,
            strings.action_extend,
            strings.no_active_machine,
            false,
        );
    }
    let aid = act.str_at("/info/id");
    let res = ctx.client.v4_post(
        "/vm/extend",
        json!({ "machine_id": aid.parse::<i64>().unwrap_or(0) }),
    );
    let msg = res.message();
    let good = crate::http::read_action_answer(&msg, res.ok) == crate::http::Verdict::Accepted;
    screens::action_result(terminal, ctx, strings.action_extend, &msg, good)
}

fn submit_flag<B: Backend>(terminal: &mut Terminal<B>, ctx: &Ctx, id: &str) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    let Some(flag) = ask(
        terminal,
        &palette,
        Prompt::new(strings.prompt_flag, strings.prompt_flag_label).hint(strings.prompt_flag_hint).paste_hint(strings.paste_hint),
    )?
    else {
        return Ok(());
    };
    if flag.trim().is_empty() {
        return Ok(());
    }
    let Some(rating) = screens::ask_rating(terminal, ctx, strings.prompt_difficulty)? else {
        return Ok(());
    };

    screens::loading(terminal, &palette, strings.loading)?;
    let res = ctx.client.v5_post(
        "/machine/own",
        json!({
            "flag": flag.trim(),
            "id": id.parse::<i64>().unwrap_or(0),
            "difficulty": rating
        }),
    );
    let msg = res.message();
    let (title, good) = match crate::http::read_flag_answer(&msg) {
        crate::http::Verdict::Accepted => (strings.flag_accepted, true),
        crate::http::Verdict::Rejected => (strings.flag_rejected, false),
        crate::http::Verdict::Unclear => (strings.result_title, res.ok),
    };
    screens::notice(terminal, ctx, title, &msg, good)
}
