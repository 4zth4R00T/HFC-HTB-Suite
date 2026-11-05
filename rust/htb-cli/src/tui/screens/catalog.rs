use crate::http::sanitize;
use crate::tui::event::{self, Input};
use crate::tui::screens;
use crate::tui::screens::labs;
use crate::tui::widgets::input::{ask, Prompt};
use crate::tui::widgets::picker::{Item, Outcome, Picker};
use crate::tui::Ctx;
use ratatui::backend::Backend;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Terminal;

const SHERLOCK_ZIP_PASSWORD: &str = "hacktheblue";
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io;

pub fn run<B: Backend<Error = std::io::Error>>(terminal: &mut Terminal<B>, ctx: &mut Ctx) -> io::Result<()> {
    let mut picker = Picker::new(menu_items(ctx));
    loop {
        let palette = ctx.palette();
        let strings = ctx.strings();
        let ctx_ref: &Ctx = ctx;
        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, strings.catalog_title, &palette);
            picker.render(f, body_a, "", &palette);
            screens::render_bars(f, footer_a, &[], ctx_ref);
        })?;

        let Some(input) = event::read(event::POLL)? else {
            continue;
        };
        if picker.capture(&input) {
            continue;
        }
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
                    0 => challenges(terminal, ctx)?,
                    1 => prolabs(terminal, ctx)?,
                    2 => fortresses(terminal, ctx)?,
                    3 => sherlocks(terminal, ctx)?,
                    4 => tracks(terminal, ctx)?,
                    5 => seasons(terminal, ctx)?,
                    _ => {}
                }
                picker.replace(menu_items(ctx));
            }
            _ => {}
        }
    }
}

fn menu_items(ctx: &Ctx) -> Vec<Item> {
    let s = ctx.strings();
    let p = ctx.palette();
    [
        (s.catalog_challenges, s.catalog_challenges_desc),
        (s.catalog_prolabs, s.catalog_prolabs_desc),
        (s.catalog_fortress, s.catalog_fortress_desc),
        (s.catalog_sherlocks, s.catalog_sherlocks_desc),
        (s.catalog_tracks, s.catalog_tracks_desc),
        (s.catalog_seasons, s.catalog_seasons_desc),
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

fn short_rating(raw: &str) -> String {
    match raw.parse::<f64>() {
        Ok(v) => format!("{v:.2}"),
        Err(_) => raw.to_string(),
    }
}

fn challenge_difficulty(rating: u8) -> u8 {
    let clamped = rating.clamp(1, 10);
    clamped * 10
}

fn id_of(v: &Value) -> String {
    v.get("id")
        .map(|x| {
            x.as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| x.to_string())
        })
        .unwrap_or_default()
}

fn text(v: &Value, key: &str) -> String {
    v.get(key)
        .map(|x| {
            x.as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| x.to_string())
        })
        .unwrap_or_else(|| "-".into())
}

fn challenge_categories(ctx: &Ctx) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for c in ctx.client.v4_get("/challenge/categories/list").array_at("/info") {
        map.insert(id_of(&c), text(&c, "name"));
    }
    map
}

fn challenges<B: Backend<Error = std::io::Error>>(terminal: &mut Terminal<B>, ctx: &mut Ctx) -> io::Result<()> {
    content_list(terminal, ctx, "challenges")
}

struct ContentQuery {
    state: String,
    keyword: String,
    difficulty: String,
    category: String,
    completed: String,
    sort: labs::Sort,
    page: u32,
}

impl ContentQuery {
    fn new() -> Self {
        ContentQuery {
            state: String::new(),
            keyword: String::new(),
            difficulty: String::new(),
            category: String::new(),
            completed: String::new(),
            sort: labs::Sort::none(),
            page: 1,
        }
    }

    fn to_string(&self, ctx: &Ctx) -> String {
        let mut qs = format!("per_page=25&page={}", self.page);
        if self.state == "favorites" {
            qs += "&todo=1";
        } else if !self.state.is_empty() {
            qs += &format!("&state={}", self.state);
        }
        if !self.keyword.is_empty() {
            qs += &format!("&keyword={}", labs::urlencode(&self.keyword));
        }
        if !self.difficulty.is_empty() {
            qs += &format!("&difficulty[]={}", labs::urlencode(&self.difficulty));
        }
        if !self.category.is_empty() {
            qs += &format!("&category[]={}", labs::urlencode(&self.category));
        }
        if !self.completed.is_empty() {
            qs += &format!("&showCompleted={}", self.completed);
        }
        qs += &self.sort.query(&labs::content_sort_fields(ctx));
        qs
    }

    fn clear(&mut self) {
        self.difficulty.clear();
        self.category.clear();
        self.completed.clear();
        self.keyword.clear();
        self.sort = labs::Sort::none();
        self.page = 1;
    }
}

fn content_categories(ctx: &Ctx, kind: &str) -> HashMap<String, String> {
    if kind == "challenges" {
        return challenge_categories(ctx);
    }
    let mut map = HashMap::new();
    for c in ctx.client.v4_get("/sherlocks/categories/list").first_array(&["/info", "/data"]) {
        map.insert(id_of(&c), text(&c, "name"));
    }
    map
}

fn content_item(c: &Value, kind: &str, cats: &HashMap<String, String>, ctx: &Ctx) -> Item {
    let p = ctx.palette();
    let s = ctx.strings();
    let solved = challenge_solved(c);
    let fav = labs::flag(c, "todo") || labs::flag(c, "is_todo") || labs::flag(c, "isTodo");
    let cat = challenge_category(c, cats);
    let diff = text(c, "difficulty");
    let progress = c.get("progress").and_then(|v| v.as_f64());
    let mut tail = format!("{} {}", s.field_solves, text(c, "solves"));
    if let Some(pr) = progress {
        if kind == "sherlocks" {
            tail = format!("{} {pr:.0}% · {tail}", s.field_progress);
        }
    }
    let labels = labs::labels_text(c, ctx);
    let state = text(c, "state");
    let mut extra = String::new();
    if state.starts_with("unreleased") {
        if let Some(delta) = countdown(&text(c, "release_date")) {
            extra = format!(" · {} {delta}", s.info_countdown);
        }
    }
    Item::with_detail(
        Line::from(vec![
            Span::styled(
                if solved { "✓" } else { " " }.to_string(),
                if solved { p.ok() } else { p.dim() },
            ),
            Span::styled(if fav { "★ " } else { "  " }.to_string(), p.warn()),
            Span::styled(format!("{:<30}", crate::commands::truncate(&text(c, "name"), 30)), p.value()),
            Span::styled(format!("{:<16}", crate::commands::truncate(&cat, 16)), p.special_style()),
            Span::styled(format!("{diff:<11}"), p.difficulty(&diff)),
            Span::styled(format!("{:<12}", labs::rating_text(c)), p.special_style()),
            Span::styled(labels, p.accent_style()),
        ]),
        Line::from(Span::styled(
            format!("      {tail} · {}{extra}", labs::date10(&text(c, "release_date"))),
            p.dim(),
        )),
    )
}

fn countdown(iso: &str) -> Option<String> {
    let secs = crate::dates::epoch_of_iso(iso)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    let left = secs - now;
    if left <= 0 {
        return None;
    }
    let d = left / 86400;
    let h = (left % 86400) / 3600;
    let mi = (left % 3600) / 60;
    Some(if d > 0 { format!("{d}d {h}h") } else { format!("{h}h {mi}m") })
}

fn content_list<B: Backend<Error = std::io::Error>>(terminal: &mut Terminal<B>, ctx: &mut Ctx, kind: &str) -> io::Result<()> {
    let cats = content_categories(ctx, kind);
    let mut query = ContentQuery::new();
    let mut rows: Vec<Value> = Vec::new();
    let mut total = String::from("0");
    let mut last = 1u32;
    let mut picker = Picker::new(Vec::new());
    let mut reload = true;

    loop {
        let palette = ctx.palette();
        let strings = ctx.strings();
        if reload {
            screens::loading(terminal, &palette, strings.loading)?;
            let r = ctx.client.v4_get(&format!("/{kind}?{}", query.to_string(ctx)));
            rows = r.first_array(&["/data", "/challenges", "/sherlocks"]);
            total = r.str_at("/meta/total");
            last = r.data.pointer("/meta/last_page").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
            let keep = picker.selected();
            picker.replace(rows.iter().map(|c| content_item(c, kind, &cats, ctx)).collect());
            picker.select(keep.min(rows.len().saturating_sub(1)));
            reload = false;
        }
        let base = if kind == "challenges" { strings.catalog_challenges } else { strings.catalog_sherlocks };
        let mut title = format!("{base} · {total}");
        if !query.state.is_empty() {
            let estado = if query.state == "favorites" { strings.dashboard_favorites.to_string() } else { labs::label_for(&labs::state_options(ctx), &query.state) };
            title += &format!(" · {estado}");
        }
        if !query.keyword.is_empty() {
            title += &format!(" · \"{}\"", query.keyword);
        }
        if !query.category.is_empty() {
            title += &format!(" · {}", cats.get(&query.category).cloned().unwrap_or_else(|| query.category.clone()));
        }
        if !query.difficulty.is_empty() {
            title += &format!(" · {}", labs::label_for(&challenge_difficulty_options(ctx), &query.difficulty));
        }
        if !query.completed.is_empty() {
            let hechos = if query.completed == "incomplete" { strings.challenge_only_unsolved.to_string() } else { labs::label_for(&labs::completed_options(ctx), &query.completed) };
            title += &format!(" · {hechos}");
        }
        let sort_label = query.sort.label(&labs::content_sort_fields(ctx), ctx);
        if !sort_label.is_empty() {
            title += &format!(" · {sort_label}");
        }
        title += &format!(" · {} {}/{last}", strings.action_page, query.page);
        let actions: Vec<(&str, &str)> = vec![
            ("t", strings.action_state),
            ("s", strings.action_search_server),
            ("c", strings.challenge_filter_category),
            ("d", strings.challenge_filter_difficulty),
            ("v", strings.action_filter_solved),
            ("a", strings.action_sort),
            ("A", strings.action_sort_flip),
            ("m", strings.action_favorite),
            ("n", strings.page_next),
            ("p", strings.page_previous),
            ("x", strings.action_clear_filters),
        ];
        let ctx_ref: &Ctx = ctx;
        let position = picker.position_label();
        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, &title, &palette);
            picker.render(f, body_a, &position, &palette);
            screens::render_list_bars(f, footer_a, &actions, &picker, ctx_ref);
        })?;

        let Some(input) = event::read(event::POLL)? else {
            continue;
        };
        if picker.capture(&input) {
            continue;
        }
        match &input {
            Input::Quit => return Ok(()),
            Input::Help => {
                screens::help_overlay(terminal, ctx)?;
                continue;
            }
            Input::ThemeToggle => {
                ctx.flip_theme();
                reload = true;
                continue;
            }
            Input::Refresh => {
                ctx.client.forget_cache();
                reload = true;
                continue;
            }
            Input::Key('t') | Input::Key('T') => {
                let options = labs::state_options(ctx);
                if let Some(v) = screens::pick_option(terminal, ctx, strings.action_state, &options)? {
                    query.state = v;
                    query.page = 1;
                    reload = true;
                }
                continue;
            }
            Input::Key('s') | Input::Key('S') => {
                let term = ask(
                    terminal,
                    &palette,
                    Prompt::new(strings.prompt_search, strings.prompt_search_label)
                        .hint(strings.prompt_search_hint)
                        .paste_hint(strings.paste_hint),
                )?;
                if let Some(term) = term {
                    query.keyword = term.trim().to_string();
                    query.page = 1;
                    reload = true;
                }
                continue;
            }
            Input::Key('c') | Input::Key('C') => {
                query.category = pick_category(terminal, ctx, &cats)?;
                query.page = 1;
                reload = true;
                continue;
            }
            Input::Key('d') | Input::Key('D') => {
                let options = challenge_difficulty_options(ctx);
                if let Some(v) = screens::pick_option(terminal, ctx, strings.challenge_filter_difficulty, &options)? {
                    query.difficulty = v;
                    query.page = 1;
                    reload = true;
                }
                continue;
            }
            Input::Key('v') | Input::Key('V') => {
                let options = labs::completed_options(ctx);
                if let Some(v) = screens::pick_option(terminal, ctx, strings.action_filter_solved, &options)? {
                    query.completed = v;
                    query.page = 1;
                    reload = true;
                }
                continue;
            }
            Input::Key('a') => {
                let fields = labs::content_sort_fields(ctx);
                query.sort.next(&fields);
                query.page = 1;
                reload = true;
                continue;
            }
            Input::Key('A') => {
                query.sort.desc = !query.sort.desc;
                if query.sort.field == 0 {
                    query.sort.field = 1;
                }
                query.page = 1;
                reload = true;
                continue;
            }
            Input::Key('m') | Input::Key('M') => {
                if let Some(c) = rows.get(picker.selected()) {
                    let fav_kind = if kind == "challenges" { "challenge" } else { "sherlock" };
                    labs::toggle_favorite(terminal, ctx, fav_kind, &id_of(c), &text(c, "name"))?;
                    ctx.client.forget_cache();
                    reload = true;
                }
                continue;
            }
            Input::Key('n') | Input::Key('N') => {
                if query.page < last {
                    query.page += 1;
                    reload = true;
                }
                continue;
            }
            Input::Key('p') | Input::Key('P') => {
                if query.page > 1 {
                    query.page -= 1;
                    reload = true;
                }
                continue;
            }
            Input::Key('x') | Input::Key('X') => {
                query.clear();
                reload = true;
                continue;
            }
            _ => {}
        }
        match picker.handle(&input) {
            Outcome::Leave => return Ok(()),
            Outcome::Chosen(i) => {
                if let Some(c) = rows.get(i) {
                    let id = id_of(c);
                    if kind == "challenges" {
                        challenge_detail(terminal, ctx, &id)?;
                    } else {
                        sherlock_detail(terminal, ctx, &id)?;
                    }
                    ctx.client.forget_cache();
                    reload = true;
                }
            }
            _ => {}
        }
    }
}

fn challenge_solved(c: &Value) -> bool {
    for key in ["is_owned", "authUserSolve", "isCompleted"] {
        if c.get(key).and_then(|v| v.as_bool()).unwrap_or(false) {
            return true;
        }
    }
    false
}

fn challenge_category(c: &Value, cats: &HashMap<String, String>) -> String {
    for key in ["category_id", "challenge_category_id"] {
        if let Some(name) = cats.get(&text(c, key)) {
            return name.clone();
        }
    }
    text(c, "category_name")
}

fn challenge_difficulty_options(ctx: &Ctx) -> Vec<(String, String)> {
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

fn pick_category<B: Backend<Error = std::io::Error>>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    cats: &HashMap<String, String>,
) -> io::Result<String> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    let mut ids: Vec<String> = cats.keys().cloned().collect();
    ids.sort_by_key(|k| k.parse::<i64>().unwrap_or(i64::MAX));
    let mut items = vec![Item::plain(strings.challenge_all_categories)];
    for id in &ids {
        items.push(Item::plain(cats[id].clone()));
    }
    let mut picker = Picker::new(items);
    loop {
        let ctx_ref: &Ctx = ctx;
        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, strings.challenge_filter_category, &palette);
            let inner = screens::centered(50, body_a.height, body_a);
            picker.render(f, inner, "", &palette);
            screens::render_bars(f, footer_a, &[], ctx_ref);
        })?;
        let Some(input) = event::read(event::POLL)? else {
            continue;
        };
        if picker.capture(&input) {
            continue;
        }
        match picker.handle(&input) {
            Outcome::Leave => return Ok(String::new()),
            Outcome::Chosen(0) => return Ok(String::new()),
            Outcome::Chosen(i) => return Ok(ids[i - 1].clone()),
            _ => {}
        }
    }
}

pub fn challenge_detail<B: Backend<Error = std::io::Error>>(terminal: &mut Terminal<B>, ctx: &mut Ctx, id: &str) -> io::Result<()> {
    loop {
        let palette = ctx.palette();
        let strings = ctx.strings();
        screens::loading(terminal, &palette, strings.loading)?;
        let raw = ctx.client.v4_get(&format!("/challenge/info/{id}"));
        let c = raw
            .data
            .pointer("/challenge")
            .or_else(|| raw.data.pointer("/data"))
            .cloned()
            .unwrap_or_else(|| raw.data.clone());
        let name = text(&c, "name");
        let downloadable = c.get("download").and_then(|v| v.as_bool()).unwrap_or(false);
        let docker = c.get("docker").and_then(|v| v.as_bool()).unwrap_or(false);
        let diff = text(&c, "difficulty");

        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled(format!("  {name}"), palette.title())),
            Line::from(""),
            screens::field(strings.field_category, &text(&c, "category_name"), &palette),
            screens::field_styled(
                strings.field_difficulty,
                &diff,
                &palette,
                palette.difficulty(&diff),
            ),
            screens::field_styled(
                strings.field_points,
                &text(&c, "points"),
                &palette,
                palette.warn(),
            ),
            screens::field(strings.field_solves, &text(&c, "solves"), &palette),
            screens::field(strings.field_author, &text(&c, "creator_name"), &palette),
            screens::field(
                strings.field_solved,
                if c.get("authUserSolve").and_then(|v| v.as_bool()).unwrap_or(false) {
                    strings.yes
                } else {
                    strings.no
                },
                &palette,
            ),
        ];
        if c.get("retired").and_then(|v| v.as_bool()).unwrap_or(false) {
            lines.push(screens::field(strings.field_state, strings.field_retired, &palette));
        }
        let description = text(&c, "description");
        if description != "-" && !description.is_empty() {
            lines.push(Line::from(""));
            lines.push(screens::section(strings.field_description, &palette));
            lines.extend(screens::wrap_paragraph(&description, &palette));
        }
        lines.push(Line::from(""));
        if downloadable {
            lines.push(screens::bullet(
                &format!("{} — {}", text(&c, "file_name"), strings.zip_password),
                &palette,
            ));
        } else {
            lines.push(screens::bullet(strings.no_files, &palette));
        }
        let container_ip = c
            .pointer("/play_info/ip")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let ports: Vec<String> = c
            .pointer("/play_info/ports")
            .and_then(|v| v.as_array())
            .map(|list| list.iter().map(|p| p.to_string()).collect())
            .unwrap_or_default();
        let container_up = !container_ip.is_empty();
        if docker {
            if container_up {
                lines.push(screens::bullet(
                    &format!(
                        "{}: {container_ip} {}   {}",
                        strings.field_container,
                        if ports.is_empty() {
                            String::new()
                        } else {
                            format!("{} {}", strings.vm_ports, ports.join(", "))
                        },
                        strings.vm_needs_vpn_labs
                    ),
                    &palette,
                ));
            } else {
                lines.push(screens::bullet(
                    &format!("{}: {}", strings.field_container, strings.vm_offline),
                    &palette,
                ));
            }
        }

        let mut actions: Vec<(&str, &str)> = vec![];
        if downloadable {
            actions.push(("d", strings.action_download));
        }
        if docker {
            if container_up {
                actions.push(("t", strings.action_stop_lab));
            } else {
                actions.push(("s", strings.action_start_lab));
            }
        }
        actions.push(("f", strings.action_flag));
        actions.push(("m", strings.action_favorite));
        actions.push(("o", strings.action_reviews));
        actions.push(("a", strings.action_activity));
        actions.push(("g", strings.action_changelog));
        if container_up {
            actions.push(("c", strings.action_copy));
        }
        let fav = labs::flag(&c, "isTodo") || labs::flag(&c, "todo") || labs::flag(&c, "is_todo");
        if fav {
            lines.insert(1, Line::from(Span::styled("  ★".to_string(), palette.warn())));
        }
        let rating = labs::rating_text(&c);
        if rating != "-" {
            lines.insert(if fav { 4 } else { 3 }, screens::field(strings.field_rating, &rating, &palette));
        }
        let ctx_ref: &Ctx = ctx;
        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, &name, &palette);
            let block = ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .border_style(palette.border_style());
            f.render_widget(
                ratatui::widgets::Paragraph::new(lines.clone())
                    .wrap(ratatui::widgets::Wrap { trim: false })
                    .block(block),
                body_a,
            );
            screens::render_bars(f, footer_a, &actions, ctx_ref);
        })?;

        match event::wait()? {
            Input::Back | Input::Quit => return Ok(()),
            Input::Help => screens::help_overlay(terminal, ctx)?,
            Input::ThemeToggle => ctx.flip_theme(),
            Input::Key('m') | Input::Key('M') => {
                labs::toggle_favorite(terminal, ctx, "challenge", id, &name)?;
                ctx.client.forget_cache();
            }
            Input::Key('o') | Input::Key('O') => labs::reviews_page(terminal, ctx, "challenge", id, &name)?,
            Input::Key('a') => labs::activity_page(terminal, ctx, "challenge", id, &name)?,
            Input::Key('g') | Input::Key('G') => labs::changelog_page(terminal, ctx, "challenge", id, &name)?,
            Input::Key('s') | Input::Key('S') if docker && !container_up => {
                screens::loading(terminal, &palette, strings.loading)?;
                let res = ctx.client.v4_post(
                    "/container/start",
                    json!({ "containerable_id": id.parse::<i64>().unwrap_or(0) }),
                );
                screens::action_result(
                    terminal,
                    ctx,
                    strings.action_start_lab,
                    &res.message(),
                    res.ok,
                )?;
            }
            Input::Copy | Input::Key('c') | Input::Key('C') if container_up => {
                let target = if ports.is_empty() {
                    container_ip.clone()
                } else {
                    format!("{container_ip}:{}", ports[0])
                };
                let value = if ports.is_empty() {
                    target
                } else {
                    format!("{target}\nhttp://{target}")
                };
                screens::copy_value(terminal, ctx, &value)?;
            }
            Input::Key('t') | Input::Key('T') if docker && container_up => {
                screens::loading(terminal, &palette, strings.loading)?;
                let res = ctx.client.v4_post(
                    "/container/stop",
                    json!({ "containerable_id": id.parse::<i64>().unwrap_or(0) }),
                );
                screens::action_result(
                    terminal,
                    ctx,
                    strings.action_stop_lab,
                    &res.message(),
                    res.ok,
                )?;
            }
            Input::Key('d') | Input::Key('D') => {
                if downloadable {
                    let out = ctx
                        .cfg
                        .download_dir
                        .join(format!("challenge_{id}_{}.zip", sanitize(&name)));
                    let url = format!("{}/challenge/download/{id}", crate::http::BASE_V4);
                    save(terminal, ctx, &url, &out)?;
                } else {
                    screens::notice(terminal, ctx, &name, strings.no_files, false)?;
                }
            }
            Input::Key('f') | Input::Key('F') => {
                let Some(flag) = ask(
                    terminal,
                    &palette,
                    Prompt::new(strings.prompt_flag, strings.prompt_flag_label)
                        .hint(strings.prompt_flag_hint).paste_hint(strings.paste_hint),
                )?
                else {
                    continue;
                };
                if flag.trim().is_empty() {
                    continue;
                }
                let Some(rating) = screens::ask_rating(terminal, ctx, strings.prompt_difficulty)?
                else {
                    continue;
                };
                let res = ctx.client.v4_post(
                    "/challenge/own",
                    json!({
                        "flag": flag.trim(),
                        "challenge_id": id.parse::<i64>().unwrap_or(0),
                        "difficulty": challenge_difficulty(rating)
                    }),
                );
                let msg = res.message();
                let good =
                    crate::http::read_flag_answer(&msg) == crate::http::Verdict::Accepted;
                screens::notice(
                    terminal,
                    ctx,
                    if good {
                        strings.flag_accepted
                    } else {
                        strings.flag_rejected
                    },
                    &msg,
                    good,
                )?;
            }
            _ => {}
        }
    }
}

fn save<B: Backend<Error = std::io::Error>>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    url: &str,
    out: &std::path::Path,
) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    screens::loading(terminal, &palette, strings.loading)?;
    match ctx.client.download(url, out) {
        Ok(size) if size > 100 => screens::notice(
            terminal,
            ctx,
            strings.result_title,
            &format!(
                "{} {} ({size} B) — {}",
                strings.saved_to,
                out.display(),
                strings.zip_password
            ),
            true,
        ),
        Ok(_) => {
            let _ = std::fs::remove_file(out);
            screens::notice(
                terminal,
                ctx,
                strings.result_title,
                strings.download_failed,
                false,
            )
        }
        Err(e) => {
            let _ = std::fs::remove_file(out);
            screens::notice(
                terminal,
                ctx,
                strings.result_title,
                &format!("{}: {e}", strings.download_failed),
                false,
            )
        }
    }
}

fn prolabs<B: Backend<Error = std::io::Error>>(terminal: &mut Terminal<B>, ctx: &mut Ctx) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    screens::loading(terminal, &palette, strings.loading)?;
    let response = ctx.client.v4_get("/prolabs");
    let labs = response.first_array(&["/data/labs", "/data"]);
    if labs.is_empty() {
        return screens::notice(terminal, ctx, strings.catalog_prolabs, strings.no_results, false);
    }
    let items: Vec<Item> = labs
        .iter()
        .map(|l| {
            let p = ctx.palette();
            let skill = text(l, "skill_level");
            Item::with_detail(
                Line::from(vec![
                    Span::styled(format!("  {:<24}", text(l, "name")), p.value()),
                    Span::styled(format!("{skill:<12}"), p.difficulty(&skill)),
                    Span::styled(format!("{:>3} ", text(l, "pro_machines_count")), p.info_style()),
                    Span::styled(strings.field_machines.to_lowercase(), p.dim()),
                ]),
                Line::from(Span::styled(
                    format!(
                        "      {} {}/{} · {} {}%",
                        strings.field_flags,
                        text(l, "pro_flags_owns_count"),
                        text(l, "pro_flags_count"),
                        strings.field_progress,
                        text(l, "ownership")
                    ),
                    p.dim(),
                )),
            )
        })
        .collect();
    let mut picker = Picker::new(items);
    loop {
        let palette = ctx.palette();
        let position = picker.position_label();
        let ctx_ref: &Ctx = ctx;
        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, strings.catalog_prolabs, &palette);
            picker.render(f, body_a, &position, &palette);
            screens::render_bars(f, footer_a, &[], ctx_ref);
        })?;
        let Some(input) = event::read(event::POLL)? else {
            continue;
        };
        if picker.capture(&input) {
            continue;
        }
        if input == Input::Quit {
            return Ok(());
        }
        if input == Input::Help {
            screens::help_overlay(terminal, ctx)?;
            continue;
        }
        match picker.handle(&input) {
            Outcome::Leave => return Ok(()),
            Outcome::Chosen(i) => {
                let lab = &labs[i];
                prolab_detail(terminal, ctx, &id_of(lab), &text(lab, "name"))?;
            }
            _ => {}
        }
    }
}

fn prolab_detail<B: Backend<Error = std::io::Error>>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    id: &str,
    name: &str,
) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    screens::loading(terminal, &palette, strings.loading)?;
    let info = ctx.client.v4_get(&format!("/prolab/{id}/info"));
    let progress = ctx.client.v4_get(&format!("/prolab/{id}/progress"));
    let entry: Vec<String> = info
        .array_at("/data/entry_points")
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    let ownership = {
        let o = progress.str_at("/data/ownership");
        if o == "-" {
            progress.str_at("/data/progress")
        } else {
            o
        }
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(format!("  {name}"), palette.title())),
        Line::from(""),
        screens::field(strings.field_version, &info.str_at("/data/version"), &palette),
        screens::field(strings.field_progress, &format!("{ownership}%"), &palette),
        screens::field(strings.field_entry_points, &entry.join(", "), &palette),
        Line::from(""),
        screens::section(strings.field_machines, &palette),
    ];
    for m in ctx.client.v4_get(&format!("/prolab/{id}/machines")).array_at("/data") {
        lines.push(screens::bullet(
            &format!("{} ({})", text(&m, "name"), text(&m, "os")),
            &palette,
        ));
    }
    lines.push(Line::from(""));
    lines.push(screens::section(strings.field_flags, &palette));
    for f in ctx.client.v4_get(&format!("/prolab/{id}/flags")).array_at("/data") {
        let owned = f.get("owned").and_then(|v| v.as_bool()).unwrap_or(false);
        lines.push(screens::bullet(
            &format!(
                "{} — {} pts {}",
                text(&f, "title"),
                text(&f, "points"),
                if owned { "✓" } else { "" }
            ),
            &palette,
        ));
    }
    screens::page(terminal, ctx, name, lines, &screens::nav_text(ctx))
}

fn fortresses<B: Backend<Error = std::io::Error>>(terminal: &mut Terminal<B>, ctx: &mut Ctx) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    screens::loading(terminal, &palette, strings.loading)?;
    let forts = ctx.client.v4_get("/fortresses").first_array(&["/data"]);
    if forts.is_empty() {
        return screens::notice(terminal, ctx, strings.catalog_fortress, strings.no_results, false);
    }
    let mut items = Vec::new();
    let mut ids = Vec::new();
    for f in &forts {
        let id = id_of(f);
        let detail = ctx.client.v4_get(&format!("/fortress/{id}"));
        let owned = detail
            .array_at("/data/flags")
            .iter()
            .filter(|fl| fl.get("owned").and_then(|v| v.as_bool()).unwrap_or(false))
            .count();
        let p = ctx.palette();
        items.push(Item::with_detail(
            Line::from(vec![
                Span::styled(format!("  {:<26}", text(f, "name")), p.value()),
                Span::styled(
                    format!("{owned}/{} {}", text(f, "number_of_flags"), strings.field_flags),
                    p.ok(),
                ),
            ]),
            Line::from(Span::styled(
                format!("      {} {}", strings.field_points, detail.str_at("/data/points")),
                p.dim(),
            )),
        ));
        ids.push(id);
    }
    let mut picker = Picker::new(items);
    loop {
        let palette = ctx.palette();
        let position = picker.position_label();
        let ctx_ref: &Ctx = ctx;
        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, strings.catalog_fortress, &palette);
            picker.render(f, body_a, &position, &palette);
            screens::render_bars(f, footer_a, &[], ctx_ref);
        })?;
        let Some(input) = event::read(event::POLL)? else {
            continue;
        };
        if picker.capture(&input) {
            continue;
        }
        if input == Input::Quit {
            return Ok(());
        }
        match picker.handle(&input) {
            Outcome::Leave => return Ok(()),
            Outcome::Chosen(i) => fortress_detail(terminal, ctx, &ids[i])?,
            _ => {}
        }
    }
}

fn fortress_detail<B: Backend<Error = std::io::Error>>(terminal: &mut Terminal<B>, ctx: &mut Ctx, id: &str) -> io::Result<()> {
    loop {
        let palette = ctx.palette();
        let strings = ctx.strings();
        screens::loading(terminal, &palette, strings.loading)?;
        let d = ctx.client.v4_get(&format!("/fortress/{id}"));
        let name = d.str_at("/data/name");
        let flags = d.array_at("/data/flags");
        let owned = flags
            .iter()
            .filter(|f| f.get("owned").and_then(|v| v.as_bool()).unwrap_or(false))
            .count();

        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled(format!("  {name}"), palette.title())),
            Line::from(""),
            screens::field_styled(
                strings.field_ip,
                &d.str_at("/data/ip"),
                &palette,
                palette.info_style(),
            ),
            screens::field_styled(
                strings.field_points,
                &d.str_at("/data/points"),
                &palette,
                palette.warn(),
            ),
            screens::field(
                strings.field_flags,
                &format!("{owned}/{}", flags.len()),
                &palette,
            ),
            screens::field(
                strings.field_progress,
                &format!("{}%", d.str_at("/data/progress_percent")),
                &palette,
            ),
        ];
        let description = d.str_at("/data/description");
        if description != "-" {
            lines.push(Line::from(""));
            lines.extend(screens::wrap_paragraph(&description, &palette));
        }
        lines.push(Line::from(""));
        lines.push(screens::section(strings.field_flags, &palette));
        for f in &flags {
            let got = f.get("owned").and_then(|v| v.as_bool()).unwrap_or(false);
            lines.push(screens::bullet(
                &format!(
                    "{} — {} pts {}",
                    text(f, "title"),
                    text(f, "points"),
                    if got { "✓" } else { "" }
                ),
                &palette,
            ));
        }

        let actions: Vec<(&str, &str)> = vec![
            ("f", strings.action_flag),
            ("c", strings.action_copy),
        ];
        let ctx_ref: &Ctx = ctx;
        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, &name, &palette);
            let block = ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .border_style(palette.border_style());
            f.render_widget(
                ratatui::widgets::Paragraph::new(lines.clone())
                    .wrap(ratatui::widgets::Wrap { trim: false })
                    .block(block),
                body_a,
            );
            screens::render_bars(f, footer_a, &actions, ctx_ref);
        })?;

        match event::wait()? {
            Input::Back | Input::Quit => return Ok(()),
            Input::Help => screens::help_overlay(terminal, ctx)?,
            Input::ThemeToggle => ctx.flip_theme(),
            Input::Key('f') | Input::Key('F') => {
                let Some(flag) = ask(
                    terminal,
                    &palette,
                    Prompt::new(strings.prompt_flag, strings.prompt_flag_label)
                        .hint(strings.prompt_flag_hint).paste_hint(strings.paste_hint),
                )?
                else {
                    continue;
                };
                if flag.trim().is_empty() {
                    continue;
                }
                let res = ctx
                    .client
                    .v4_post(&format!("/fortress/{id}/flag"), json!({ "flag": flag.trim() }));
                let msg = res.message();
                let good =
                    crate::http::read_flag_answer(&msg) == crate::http::Verdict::Accepted;
                screens::notice(
                    terminal,
                    ctx,
                    if good {
                        strings.flag_accepted
                    } else {
                        strings.flag_rejected
                    },
                    &msg,
                    good,
                )?;
            }
            _ => {}
        }
    }
}

fn sherlocks<B: Backend<Error = std::io::Error>>(terminal: &mut Terminal<B>, ctx: &mut Ctx) -> io::Result<()> {
    content_list(terminal, ctx, "sherlocks")
}

struct SherlockView {
    name: String,
    vm_ip: String,
    header: Vec<Line<'static>>,
    downloadable: bool,
    has_vm: bool,
    vm_up: bool,
    password: String,
    tasks: Vec<Value>,
}

fn load_sherlock(ctx: &Ctx, id: &str) -> SherlockView {
    let palette = ctx.palette();
    let strings = ctx.strings();
    let meta = ctx.client.v4_get(&format!("/sherlocks/{id}"));
    let info = ctx.client.v4_get(&format!("/sherlocks/{id}/info"));
    let progress = ctx.client.v4_get(&format!("/sherlocks/{id}/progress"));
    let tasks = ctx
        .client
        .v4_get(&format!("/sherlocks/{id}/tasks"))
        .array_at("/data");

    let play = ctx.client.v4_get(&format!("/sherlocks/{id}/play"));
    let name = meta.str_at("/data/name");
    let diff = meta.str_at("/data/difficulty");
    let methods = meta.array_at("/data/play_methods");
    let downloadable = methods.iter().any(|m| m.as_str() == Some("download"));
    let has_vm = methods.iter().any(|m| m.as_str() == Some("vm"));
    let password = {
        let raw = meta.str_at("/data/file_password");
        if raw == "-" || raw.trim().is_empty() {
            SHERLOCK_ZIP_PASSWORD.to_string()
        } else {
            raw
        }
    };
    let vm_status = play.str_at("/data/play_info/status");
    let vm_ip = play.str_at("/data/play_info/ip");
    let vm_up = vm_status != "-";

    let answered = progress.str_at("/data/tasks_answered");
    let total = progress.str_at("/data/total_tasks");
    let done = if answered == "-" {
        tasks
            .iter()
            .filter(|t| t.get("completed").and_then(|v| v.as_bool()).unwrap_or(false))
            .count()
            .to_string()
    } else {
        answered
    };
    let count = if total == "-" {
        tasks.len().to_string()
    } else {
        total
    };

    let mut header = vec![
        screens::field(
            strings.field_category,
            &meta.str_at("/data/category_name"),
            &palette,
        ),
        screens::field_styled(
            strings.field_difficulty,
            &diff,
            &palette,
            palette.difficulty(&diff),
        ),
        screens::field(
            strings.field_rating,
            &short_rating(&meta.str_at("/data/rating")),
            &palette,
        ),
        screens::field(strings.field_state, &meta.str_at("/data/state"), &palette),
        screens::field(
            strings.field_released,
            &meta
                .str_at("/data/release_at")
                .chars()
                .take(10)
                .collect::<String>(),
            &palette,
        ),
        screens::field(
            strings.field_progress,
            &format!("{done}/{count} {}", strings.sherlock_answered),
            &palette,
        ),
    ];
    if downloadable {
        header.push(screens::field(
            strings.sherlock_material,
            &if password == "-" || password.is_empty() {
                strings.zip_password.to_string()
            } else {
                format!("{} {password}", strings.zip_password_is)
            },
            &palette,
        ));
    }
    if has_vm {
        header.push(screens::field_styled(
            strings.vm_status,
            &if vm_up {
                if vm_ip == "-" {
                    strings.vm_deploying.to_string()
                } else {
                    format!("{vm_ip}   {}", strings.vm_needs_vpn)
                }
            } else {
                strings.vm_offline.to_string()
            },
            &palette,
            if vm_ip != "-" {
                palette.info_style()
            } else if vm_up {
                palette.warn()
            } else {
                palette.dim()
            },
        ));
        let user = play.str_at("/data/username");
        let pass = play.str_at("/data/password");
        if user != "-" || pass != "-" {
            header.push(screens::field(
                strings.vm_credentials,
                &format!("{} {user}   {} {pass}", strings.vm_user, strings.vm_password),
                &palette,
            ));
        }
    }
    if !downloadable && !has_vm {
        header.push(screens::field(
            strings.sherlock_material,
            strings.sherlock_no_material,
            &palette,
        ));
    }
    let scenario = info.str_at("/data/description");
    if scenario != "-" {
        header.push(Line::from(""));
        header.extend(screens::wrap_paragraph(&scenario, &palette));
    }
    if meta.data.pointer("/data/isTodo").and_then(|v| v.as_bool()).unwrap_or(false)
        || meta.data.pointer("/data/todo").and_then(|v| v.as_bool()).unwrap_or(false)
    {
        header.insert(0, Line::from(Span::styled("  ★".to_string(), palette.warn())));
    }

    SherlockView {
        name,
        vm_ip: vm_ip.clone(),
        header,
        downloadable,
        has_vm,
        vm_up,
        password,
        tasks,
    }
}

pub fn sherlock_detail<B: Backend<Error = std::io::Error>>(terminal: &mut Terminal<B>, ctx: &mut Ctx, id: &str) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    screens::loading(terminal, &palette, strings.loading)?;
    let mut view = load_sherlock(ctx, id);
    let mut picker = Picker::new(view.tasks.iter().map(|t| task_item(t, ctx)).collect());

    loop {
        let palette = ctx.palette();
        let strings = ctx.strings();
        let mut actions: Vec<(&str, &str)> = vec![("↵", strings.action_answer)];
        if view.downloadable {
            actions.push(("d", strings.action_download));
        }
        if view.has_vm {
            if view.vm_up {
                actions.push(("t", strings.action_stop_lab));
                actions.push(("r", strings.action_reset));
                if view.vm_ip != "-" {
                    actions.push(("c", strings.action_copy));
                }
            } else {
                actions.push(("s", strings.action_start_vm));
            }
        }
        actions.push(("m", strings.action_favorite));
        actions.push(("o", strings.action_reviews));
        actions.push(("w", strings.action_walkthroughs));
        let ctx_ref: &Ctx = ctx;
        let position = picker.position_label();
        let header_height = (view.header.len() as u16 + 2).min(18);
        let header = view.header.clone();
        let name = view.name.clone();

        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, &name, &palette);
            let split = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    ratatui::layout::Constraint::Length(header_height),
                    ratatui::layout::Constraint::Min(4),
                ])
                .split(body_a);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(palette.border_style());
            f.render_widget(
                Paragraph::new(header.clone())
                    .wrap(Wrap { trim: false })
                    .block(block),
                split[0],
            );
            picker.render(
                f,
                split[1],
                &format!("{} {position}", strings.field_tasks),
                &palette,
            );
            screens::render_bars(f, footer_a, &actions, ctx_ref);
        })?;

        let Some(input) = event::read(event::POLL)? else {
            continue;
        };
        if picker.capture(&input) {
            continue;
        }
        let mut reload = false;
        match &input {
            Input::Quit => return Ok(()),
            Input::Help => {
                screens::help_overlay(terminal, ctx)?;
                continue;
            }
            Input::ThemeToggle => {
                ctx.flip_theme();
                picker.replace(view.tasks.iter().map(|t| task_item(t, ctx)).collect());
                continue;
            }
            Input::Refresh => {
                ctx.client.forget_cache();
                reload = true;
            }
            Input::Key('m') | Input::Key('M') => {
                labs::toggle_favorite(terminal, ctx, "sherlock", id, &view.name)?;
                ctx.client.forget_cache();
                reload = true;
            }
            Input::Key('o') | Input::Key('O') => {
                labs::reviews_page(terminal, ctx, "sherlock", id, &view.name)?;
                continue;
            }
            Input::Key('w') | Input::Key('W') => {
                screens::loading(terminal, &palette, strings.loading)?;
                let w = ctx.client.v4_get_fresh(&format!("/sherlocks/{id}/writeup"));
                let url = w.str_at("/url");
                if url == "-" {
                    screens::action_result(terminal, ctx, strings.walkthrough_official, &w.message(), false)?;
                } else {
                    crate::clipboard::copy(&url);
                    if !labs::open_url(&url) {
                        screens::copy_value(terminal, ctx, &url)?;
                    }
                }
                continue;
            }
            Input::Copy | Input::Key('c') | Input::Key('C') if view.vm_ip != "-" => {
                let value = format!(
                    "{}\nxfreerdp /v:{} /u:LetsDefend /p:'' /cert:ignore /dynamic-resolution",
                    view.vm_ip, view.vm_ip
                );
                screens::copy_value(terminal, ctx, &value)?;
            }
            Input::Key('s') | Input::Key('S') if view.has_vm && !view.vm_up => {
                screens::loading(terminal, &palette, strings.loading)?;
                let res = ctx
                    .client
                    .v4_post(&format!("/challenge/start/vm/{id}"), json!({}));
                let msg = res.message();
                let good = res.ok;
                let user = res.str_at("/username");
                let pass = res.str_at("/password");
                let body = if user == "-" {
                    msg
                } else {
                    format!(
                        "{msg}\n{} {user}   {} {pass}",
                        strings.vm_user, strings.vm_password
                    )
                };
                screens::action_result(terminal, ctx, strings.action_start_vm, &body, good)?;
                reload = true;
            }
            Input::Key('t') | Input::Key('T') if view.has_vm && view.vm_up => {
                screens::loading(terminal, &palette, strings.loading)?;
                let res = ctx.client.v4_post(
                    "/vm/terminate",
                    json!({ "machine_id": id.parse::<i64>().unwrap_or(0) }),
                );
                let msg = res.message();
                screens::action_result(terminal, ctx, strings.action_stop_lab, &msg, res.ok)?;
                reload = true;
            }
            Input::Key('r') | Input::Key('R') if view.has_vm && view.vm_up => {
                screens::loading(terminal, &palette, strings.loading)?;
                let res = ctx
                    .client
                    .v4_post(&format!("/challenge/reset/vm/{id}"), json!({}));
                let msg = res.message();
                screens::action_result(terminal, ctx, strings.action_reset, &msg, res.ok)?;
                reload = true;
            }
            Input::Key('d') | Input::Key('D') => {
                if !view.downloadable {
                    screens::notice(terminal, ctx, &view.name, strings.sherlock_no_material, false)?;
                    continue;
                }
                screens::loading(terminal, &palette, strings.loading)?;
                let link = ctx
                    .client
                    .v4_get(&format!("/sherlocks/{id}/download_link"))
                    .str_at("/url");
                if link == "-" || link.is_empty() {
                    screens::notice(terminal, ctx, &view.name, strings.no_download_link, false)?;
                    continue;
                }
                let out = ctx
                    .cfg
                    .download_dir
                    .join(format!("sherlock_{id}_{}.zip", sanitize(&view.name)));
                let password = view.password.clone();
                save_with_password(terminal, ctx, &link, &out, &password)?;
                continue;
            }
            _ => {}
        }
        if !reload {
            match picker.handle(&input) {
                Outcome::Leave => return Ok(()),
                Outcome::Chosen(i) => {
                    if let Some(task) = view.tasks.get(i).cloned() {
                        answer_task(terminal, ctx, id, &task)?;
                        reload = true;
                    }
                }
                _ => {}
            }
        }
        if reload {
            let keep = picker.selected();
            screens::loading(terminal, &ctx.palette(), ctx.strings().loading)?;
            view = load_sherlock(ctx, id);
            picker.replace(view.tasks.iter().map(|t| task_item(t, ctx)).collect());
            picker.select(keep);
        }
    }
}

fn task_item(task: &Value, ctx: &Ctx) -> Item {
    let p = ctx.palette();
    let s = ctx.strings();
    let done = task
        .get("completed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let (mark, mark_style) = if done {
        ("✓ ", p.ok())
    } else {
        ("  ", p.dim())
    };
    let question = text(task, "description");
    let expected = text(task, "masked_flag");
    let detail = if expected == "-" {
        String::new()
    } else {
        format!("      {}: {expected}", s.sherlock_expected)
    };
    Item::with_detail(
        Line::from(vec![
            Span::styled(mark.to_string(), mark_style),
            Span::styled(format!("{:<10}", text(task, "title")), p.label()),
            Span::styled(question, p.value()),
        ]),
        Line::from(Span::styled(detail, p.dim())),
    )
}

fn answer_task<B: Backend<Error = std::io::Error>>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    sherlock: &str,
    task: &Value,
) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    let title = text(task, "title");
    let question = text(task, "description");
    let expected = text(task, "masked_flag");
    let hint = if expected == "-" {
        String::new()
    } else {
        format!("{}: {expected}", strings.sherlock_expected)
    };

    let Some(answer) = ask(
        terminal,
        &palette,
        Prompt::new(&format!("{title} — {question}"), strings.ask_answer)
            .hint(&hint)
            .paste_hint(strings.paste_hint),
    )?
    else {
        return Ok(());
    };
    if answer.trim().is_empty() {
        return Ok(());
    }

    screens::loading(terminal, &palette, strings.loading)?;
    let task_id = id_of(task);
    let res = ctx.client.v4_post(
        &format!("/sherlocks/{sherlock}/tasks/{task_id}/flag"),
        json!({ "flag": answer.trim() }),
    );
    let msg = res.message();
    let good = crate::http::read_flag_answer(&msg) == crate::http::Verdict::Accepted;
    screens::notice(
        terminal,
        ctx,
        if good {
            strings.task_accepted
        } else {
            strings.task_rejected
        },
        &msg,
        good,
    )
}

fn save_with_password<B: Backend<Error = std::io::Error>>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    url: &str,
    out: &std::path::Path,
    password: &str,
) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    screens::loading(terminal, &palette, strings.loading)?;
    let note = if password == "-" || password.is_empty() {
        strings.zip_password.to_string()
    } else {
        format!("{} {password}", strings.zip_password_is)
    };
    match ctx.client.download(url, out) {
        Ok(size) if size > 100 => screens::notice(
            terminal,
            ctx,
            strings.result_title,
            &format!("{} {} ({size} B)\n{note}", strings.saved_to, out.display()),
            true,
        ),
        Ok(_) => {
            let _ = std::fs::remove_file(out);
            screens::notice(terminal, ctx, strings.result_title, strings.download_failed, false)
        }
        Err(e) => {
            let _ = std::fs::remove_file(out);
            screens::notice(
                terminal,
                ctx,
                strings.result_title,
                &format!("{}: {e}", strings.download_failed),
                false,
            )
        }
    }
}

fn tracks<B: Backend<Error = std::io::Error>>(terminal: &mut Terminal<B>, ctx: &mut Ctx) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    screens::loading(terminal, &palette, strings.loading)?;
    let response = ctx.client.v4_get("/tracks");
    let all = response.first_array(&["/data"]);
    if all.is_empty() {
        return screens::notice(terminal, ctx, strings.catalog_tracks, strings.no_results, false);
    }
    let enrolled: HashMap<String, bool> = ctx
        .client
        .v4_get("/user/tracks")
        .first_array(&["/data"])
        .iter()
        .map(|t| (id_of(t), t.get("complete").and_then(|v| v.as_i64()).unwrap_or(0) > 0))
        .collect();
    let mut difficulty = String::new();
    let mut list: Vec<Value> = all.clone();
    let mut picker = Picker::new(Vec::new());
    let mut rebuild = true;
    loop {
        let palette = ctx.palette();
        let strings = ctx.strings();
        if rebuild {
            list = all
                .iter()
                .filter(|t| difficulty.is_empty() || text(t, "difficulty").eq_ignore_ascii_case(&difficulty))
                .cloned()
                .collect();
            let p = ctx.palette();
            picker.replace(
                list.iter()
                    .map(|t| {
                        let diff = text(t, "difficulty");
                        let id = id_of(t);
                        let mark = match enrolled.get(&id) {
                            Some(true) => ("✓✓", p.ok()),
                            Some(false) => ("✓ ", p.warn()),
                            None => ("  ", p.dim()),
                        };
                        let creator = t.get("creator").map(|c| text(c, "name")).unwrap_or_else(|| "-".into());
                        Item::with_detail(
                            Line::from(vec![
                                Span::styled(format!("{} ", mark.0), mark.1),
                                Span::styled(format!("{:<34}", text(t, "name")), p.value()),
                                Span::styled(format!("{diff:<10}"), p.difficulty(&diff)),
                                Span::styled(labs::labels_text(t, ctx), p.accent_style()),
                            ]),
                            Line::from(Span::styled(
                                format!(
                                    "      {} {} · {creator}{}",
                                    strings.field_likes,
                                    text(t, "likes"),
                                    if enrolled.contains_key(&id) { format!(" · {}", strings.tracks_enrolled_mark) } else { String::new() }
                                ),
                                p.dim(),
                            )),
                        )
                    })
                    .collect(),
            );
            picker.select(0);
            rebuild = false;
        }
        let position = picker.position_label();
        let title = if difficulty.is_empty() {
            strings.catalog_tracks.to_string()
        } else {
            format!("{} · {difficulty}", strings.catalog_tracks)
        };
        let actions = [("d", strings.challenge_filter_difficulty), ("x", strings.action_clear_filters)];
        let ctx_ref: &Ctx = ctx;
        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, &title, &palette);
            picker.render(f, body_a, &position, &palette);
            screens::render_list_bars(f, footer_a, &actions, &picker, ctx_ref);
        })?;
        let Some(input) = event::read(event::POLL)? else {
            continue;
        };
        if picker.capture(&input) {
            continue;
        }
        match &input {
            Input::Quit => return Ok(()),
            Input::Help => {
                screens::help_overlay(terminal, ctx)?;
                continue;
            }
            Input::ThemeToggle => {
                ctx.flip_theme();
                rebuild = true;
                continue;
            }
            Input::Key('d') | Input::Key('D') => {
                let options: Vec<(String, String)> = vec![
                    (strings.filter_any.to_string(), String::new()),
                    ("Very Easy".to_string(), "Very Easy".to_string()),
                    ("Easy".to_string(), "Easy".to_string()),
                    ("Medium".to_string(), "Medium".to_string()),
                    ("Hard".to_string(), "Hard".to_string()),
                ];
                if let Some(v) = screens::pick_option(terminal, ctx, strings.challenge_filter_difficulty, &options)? {
                    difficulty = v;
                    rebuild = true;
                }
                continue;
            }
            Input::Key('x') | Input::Key('X') => {
                difficulty.clear();
                rebuild = true;
                continue;
            }
            _ => {}
        }
        match picker.handle(&input) {
            Outcome::Leave => return Ok(()),
            Outcome::Chosen(i) => {
                if let Some(t) = list.get(i) {
                    track_detail(terminal, ctx, &id_of(t), &text(t, "name"))?;
                }
            }
            _ => {}
        }
    }
}

fn track_detail<B: Backend<Error = std::io::Error>>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    id: &str,
    name: &str,
) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    screens::loading(terminal, &palette, strings.loading)?;
    let raw = ctx.client.v4_get(&format!("/tracks/{id}"));
    let t = raw
        .data
        .pointer("/data")
        .cloned()
        .unwrap_or_else(|| raw.data.clone());
    let diff = text(&t, "difficulty");
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(format!("  {name}"), palette.title())),
        Line::from(""),
        screens::field_styled(
            strings.field_difficulty,
            &diff,
            &palette,
            palette.difficulty(&diff),
        ),
        screens::field(strings.field_likes, &text(&t, "likes"), &palette),
        screens::field(
            strings.field_enrolled,
            if t.get("enrolled").and_then(|v| v.as_bool()).unwrap_or(false) {
                strings.yes
            } else {
                strings.no
            },
            &palette,
        ),
    ];
    let description = text(&t, "description");
    if description != "-" {
        lines.push(Line::from(""));
        lines.extend(screens::wrap_paragraph(&description, &palette));
    }
    lines.push(Line::from(""));
    lines.push(screens::section(strings.field_content, &palette));
    if let Some(items) = t.get("items").and_then(|v| v.as_array()) {
        for it in items {
            lines.push(screens::bullet(
                &format!(
                    "[{}] {} ({})",
                    text(it, "type"),
                    text(it, "name"),
                    text(it, "difficulty")
                ),
                &palette,
            ));
        }
    }
    screens::page(terminal, ctx, name, lines, &screens::nav_text(ctx))
}

fn seasons<B: Backend<Error = std::io::Error>>(terminal: &mut Terminal<B>, ctx: &mut Ctx) -> io::Result<()> {
    let strings = ctx.strings();
    let list = {
        let palette = ctx.palette();
        screens::loading(terminal, &palette, strings.loading)?;
        ctx.client.v4_get("/season/list")
    };
    let active = list
        .array_at("/data")
        .into_iter()
        .find(|s| s.get("active").and_then(|v| v.as_bool()) == Some(true));
    let Some(a) = active else {
        return season_overview(terminal, ctx);
    };
    let sid = id_of(&a);
    let name = text(&a, "name");
    let p = ctx.palette();
    let mut picker = Picker::new(vec![
        Item::plain_line(Line::from(Span::styled(strings.season_overview.to_string(), p.value()))),
        Item::plain_line(Line::from(Span::styled(strings.season_leaderboard.to_string(), p.value()))),
        Item::plain_line(Line::from(Span::styled(strings.season_rewards.to_string(), p.value()))),
    ]);
    loop {
        let palette = ctx.palette();
        let ctx_ref: &Ctx = ctx;
        let title = format!("{} · {name}", strings.catalog_seasons);
        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, &title, &palette);
            let inner = screens::centered(46, 5, body_a);
            picker.render(f, inner, "", &palette);
            screens::render_bars(f, footer_a, &[], ctx_ref);
        })?;
        let Some(input) = event::read(event::POLL)? else { continue };
        if picker.capture(&input) {
            continue;
        }
        match &input {
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
            Outcome::Chosen(0) => season_overview(terminal, ctx)?,
            Outcome::Chosen(1) => labs::season_leaderboard(terminal, ctx, &sid, &name)?,
            Outcome::Chosen(2) => labs::season_rewards(terminal, ctx, &sid, &name)?,
            _ => {}
        }
    }
}

fn season_overview<B: Backend<Error = std::io::Error>>(terminal: &mut Terminal<B>, ctx: &Ctx) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    screens::loading(terminal, &palette, strings.loading)?;
    let list = ctx.client.v4_get("/season/list");
    let all = list.array_at("/data");
    let active = all
        .iter()
        .find(|s| s.get("active").and_then(|v| v.as_bool()) == Some(true));

    let mut lines = vec![Line::from("")];
    match active {
        None => lines.push(Line::from(Span::styled(
            format!("  {}", strings.season_none),
            palette.warn(),
        ))),
        Some(a) => {
            let sid = id_of(a);
            let rank = ctx.client.v4_get(&format!("/season/user/rank/{sid}"));
            lines.push(Line::from(Span::styled(
                format!("  {}", text(a, "name")),
                palette.title(),
            )));
            let end = text(a, "end_date");
            if let Some(left) = countdown(&end) {
                lines.push(Line::from(Span::styled(
                    format!("  {} {left}", strings.time_left),
                    palette.dim(),
                )));
            }
            lines.push(Line::from(""));
            lines.push(screens::field_styled(
                strings.season_tier,
                &format!(
                    "{} ({}/{})",
                    rank.str_at("/data/league"),
                    rank.str_at("/data/rank"),
                    rank.str_at("/data/total_ranks")
                ),
                &palette,
                palette.special_style(),
            ));
            lines.push(screens::field(
                strings.season_points,
                &rank.str_at("/data/total_season_points"),
                &palette,
            ));
            lines.push(screens::field(
                strings.season_flags,
                &format!(
                    "{}/{}",
                    rank.str_at("/data/total_season_flags/obtained"),
                    rank.str_at("/data/total_season_flags/total")
                ),
                &palette,
            ));
            lines.push(screens::field(
                strings.season_bloods,
                &rank.str_at("/data/total_season_bloods"),
                &palette,
            ));
            lines.push(Line::from(""));
            let current = ctx.client.v4_get("/season/machine/active");
            if current.has("/data/name") {
                lines.push(screens::field_styled(
                    strings.season_next_drop,
                    &format!(
                        "{} · {} {}/{}",
                        current.str_at("/data/name"),
                        strings.season_points_user_root,
                        current.str_at("/data/user_own_points"),
                        current.str_at("/data/root_own_points")
                    ),
                    &palette,
                    palette.accent_style(),
                ));
            }
            lines.push(Line::from(""));
            lines.push(screens::section(strings.season_machines, &palette));
            for (week, m) in ctx.client.v4_get("/season/machines").first_array(&["/data"]).iter().enumerate() {
                let root = m.get("is_owned_root").and_then(|v| v.as_bool()).unwrap_or(false);
                let user = m.get("is_owned_user").and_then(|v| v.as_bool()).unwrap_or(false);
                let released = m.get("is_released").and_then(|v| v.as_bool()).unwrap_or(true);
                let unknown = m.get("unknown").and_then(|v| v.as_bool()).unwrap_or(false);
                let mark = if root {
                    "✓✓"
                } else if user {
                    "✓ "
                } else {
                    "  "
                };
                let name = if unknown { "?".to_string() } else { text(m, "name") };
                let when = if released {
                    String::new()
                } else {
                    match countdown(&text(m, "release_time")) {
                        Some(d) => format!(" · {} {d}", strings.info_countdown),
                        None => format!(" · {}", labs::date10(&text(m, "release_time"))),
                    }
                };
                lines.push(screens::bullet(
                    &format!(
                        "{mark} {} {:<2} {name} ({} · {}){when}",
                        strings.season_week,
                        week + 1,
                        text(m, "os"),
                        text(m, "difficulty_text")
                    ),
                    &palette,
                ));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  {}", strings.season_spawn_note),
                palette.dim(),
            )));
        }
    }
    screens::page(terminal, ctx, strings.catalog_seasons, lines, &screens::nav_text(ctx))
}
