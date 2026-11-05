use crate::http::ApiResponse;
use crate::tui::event::{self, Input};
use crate::tui::screens;
use crate::tui::widgets::input::{ask, Prompt};
use crate::tui::widgets::picker::{Item, Outcome, Picker};
use crate::tui::Ctx;
use ratatui::backend::Backend;
use ratatui::text::{Line, Span};
use ratatui::Terminal;
use serde_json::{json, Value};
use std::io;

pub const WEB: &str = "https://app.hackthebox.com";

pub fn text(v: &Value, key: &str) -> String {
    v.get(key)
        .map(|x| match x {
            Value::String(s) => s.clone(),
            Value::Null => "-".to_string(),
            other => other.to_string(),
        })
        .unwrap_or_else(|| "-".into())
}

pub fn id_of(v: &Value) -> String {
    text(v, "id")
}

pub fn flag(v: &Value, key: &str) -> bool {
    v.get(key).and_then(|x| x.as_bool()).unwrap_or(false)
}

pub fn date10(s: &str) -> String {
    s.chars().take(10).collect()
}

pub fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            b' ' => "+".to_string(),
            other => format!("%{other:02X}"),
        })
        .collect()
}

pub fn open_url(url: &str) -> bool {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return false;
    }
    crate::commands::open_externally(url)
}

pub fn rating_text(v: &Value) -> String {
    let rating = v
        .get("rating")
        .and_then(|r| r.as_f64().or_else(|| r.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0.0);
    let count = v
        .get("ratingCount")
        .or_else(|| v.get("rating_count"))
        .and_then(|c| c.as_u64())
        .unwrap_or(0);
    if count == 0 {
        "-".to_string()
    } else {
        format!("★{rating:.1} ({count})")
    }
}

pub fn labels_text(v: &Value, ctx: &Ctx) -> String {
    let s = ctx.strings();
    let mut out = Vec::new();
    if let Some(labels) = v.get("labels").and_then(|l| l.as_array()) {
        for l in labels {
            let name = text(l, "name");
            if name != "-" {
                out.push(name.to_uppercase());
            }
        }
    }
    if v.get("labels").and_then(|l| l.as_object()).is_some() {
        let name = text(&v["labels"], "name");
        if name != "-" {
            out.push(name.to_uppercase());
        }
    }
    if flag(v, "staff_pick") || v.get("staff_pick").and_then(|x| x.as_i64()) == Some(1) {
        out.push(s.staff_pick.to_uppercase());
    }
    if flag(v, "recommended") && out.is_empty() {
        out.push("HTB".to_string());
    }
    out.join(" · ")
}

#[derive(Clone)]
pub struct Sort {
    pub field: usize,
    pub desc: bool,
}

impl Sort {
    pub fn none() -> Self {
        Sort { field: 0, desc: true }
    }

    pub fn query(&self, fields: &[(&str, String)]) -> String {
        if self.field == 0 {
            return String::new();
        }
        match fields.get(self.field) {
            Some((api, _)) => format!(
                "&sort_by={api}&sort_type={}",
                if self.desc { "desc" } else { "asc" }
            ),
            None => String::new(),
        }
    }

    pub fn label(&self, fields: &[(&str, String)], ctx: &Ctx) -> String {
        let s = ctx.strings();
        if self.field == 0 {
            return String::new();
        }
        let name = fields.get(self.field).map(|(_, l)| l.clone()).unwrap_or_default();
        let (flecha, sentido) = if self.desc { ("▼", s.sort_desc) } else { ("▲", s.sort_asc) };
        format!("{} {} {flecha} {sentido}", s.action_sort, name)
    }

    pub fn next(&mut self, fields: &[(&str, String)]) {
        self.field = (self.field + 1) % fields.len();
    }
}

pub fn machine_sort_fields(ctx: &Ctx) -> Vec<(&'static str, String)> {
    let s = ctx.strings();
    vec![
        ("", String::new()),
        ("rating", s.sort_rating.to_string()),
        ("release_date", s.sort_release.to_string()),
        ("user_difficulty", s.sort_user_difficulty.to_string()),
        ("user_owns", s.sort_user_owns.to_string()),
        ("system_owns", s.sort_system_owns.to_string()),
        ("name", s.sort_name.to_string()),
    ]
}

pub fn content_sort_fields(ctx: &Ctx) -> Vec<(&'static str, String)> {
    let s = ctx.strings();
    vec![
        ("", String::new()),
        ("rating", s.sort_rating.to_string()),
        ("release_date", s.sort_release.to_string()),
        ("solves", s.sort_solves.to_string()),
        ("name", s.sort_name.to_string()),
    ]
}

pub fn completed_options(ctx: &Ctx) -> Vec<(String, String)> {
    let s = ctx.strings();
    vec![
        (s.filter_any.to_string(), String::new()),
        (s.filter_solved.to_string(), "complete".to_string()),
        (s.filter_unsolved.to_string(), "incomplete".to_string()),
    ]
}

pub fn state_options(ctx: &Ctx) -> Vec<(String, String)> {
    let s = ctx.strings();
    vec![
        (s.state_all.to_string(), String::new()),
        (s.state_active.to_string(), "active".to_string()),
        (s.state_retired.to_string(), "retired".to_string()),
        (s.state_unreleased.to_string(), "unreleased".to_string()),
        (s.state_favorites.to_string(), "favorites".to_string()),
    ]
}

pub fn label_for(options: &[(String, String)], value: &str) -> String {
    options
        .iter()
        .find(|(_, v)| v == value)
        .map(|(label, _)| label.clone())
        .unwrap_or_else(|| value.to_string())
}

pub fn toggle_favorite<B: Backend<Error = std::io::Error>>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    kind: &str,
    id: &str,
    title: &str,
) -> io::Result<bool> {
    let palette = ctx.palette();
    let s = ctx.strings();
    screens::loading(terminal, &palette, s.loading)?;
    let paths: Vec<String> = match kind {
        "machine" => vec![format!("/machine/todo/update/{id}")],
        "challenge" => vec![format!("/challenge/todo/update/{id}")],
        _ => vec![
            format!("/sherlock/todo/update/{id}"),
            format!("/sherlocks/todo/update/{id}"),
            format!("/sherlocks/{id}/todo"),
        ],
    };
    let mut res: Option<ApiResponse> = None;
    for p in &paths {
        let r = ctx.client.v4_post(p, json!({}));
        if r.status != 404 && r.status != 405 {
            res = Some(r);
            break;
        }
    }
    let Some(res) = res else {
        screens::notice(terminal, ctx, title, s.not_in_api, false)?;
        return Ok(false);
    };
    if !res.ok {
        screens::action_result(terminal, ctx, title, &res.message(), false)?;
        return Ok(false);
    }
    let ids = res.first_array(&["/info", "/data"]);
    let now_fav = ids.iter().any(|v| id_of(v) == id || v == id);
    let body = if now_fav { s.favorite_added } else { s.favorite_removed };
    screens::notice(terminal, ctx, title, body, true)?;
    Ok(now_fav)
}

fn stars(n: u64) -> String {
    let n = n.min(5) as usize;
    format!("{}{}", "★".repeat(n), "☆".repeat(5 - n))
}

pub fn reviews_page<B: Backend<Error = std::io::Error>>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    kind: &str,
    id: &str,
    title: &str,
) -> io::Result<()> {
    let palette = ctx.palette();
    let s = ctx.strings();
    screens::loading(terminal, &palette, s.loading)?;
    let r = ctx.client.v4_get(&format!(
        "/review/{kind}/{id}/paginated?sort_by[]=created_at&sort_type=desc&per_page=40"
    ));
    let rows = r.array_at("/data");
    let mut lines = vec![Line::from("")];
    if rows.is_empty() {
        lines.push(Line::from(Span::styled(format!("  {}", s.reviews_none), palette.dim())));
    } else {
        let total = r.str_at("/meta/total");
        lines.push(Line::from(Span::styled(
            format!("  {} · {total}", s.reviews_title),
            palette.title(),
        )));
        lines.push(Line::from(""));
        for rv in &rows {
            let st = rv.get("stars").and_then(|v| v.as_u64()).unwrap_or(0);
            let user = rv.get("user").map(|u| text(u, "name")).unwrap_or_else(|| "-".into());
            let when = date10(&text(rv, "created_at"));
            let head = text(rv, "title");
            let helpful = rv.get("helpful_reviews_count").and_then(|v| v.as_u64()).unwrap_or(0);
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", stars(st)), palette.warn()),
                Span::styled(head, palette.value()),
                Span::styled(format!("   {user} · {when}"), palette.dim()),
            ]));
            let msg = text(rv, "message");
            if msg != "-" {
                lines.extend(screens::wrap_paragraph(&msg, &palette));
            }
            if helpful > 0 {
                lines.push(Line::from(Span::styled(
                    format!("    {} {helpful}", s.reviews_helpful),
                    palette.dim(),
                )));
            }
            lines.push(Line::from(""));
        }
    }
    screens::page(terminal, ctx, &format!("{} · {title}", s.reviews_title), lines, &screens::nav_text(ctx))
}

pub fn activity_page<B: Backend<Error = std::io::Error>>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    kind: &str,
    id: &str,
    title: &str,
) -> io::Result<()> {
    let palette = ctx.palette();
    let s = ctx.strings();
    screens::loading(terminal, &palette, s.loading)?;
    let r = ctx.client.v4_get(&format!("/{kind}/activity/{id}"));
    let rows = r.first_array(&["/info/activity", "/data"]);
    let mut lines = vec![Line::from("")];
    if rows.is_empty() {
        lines.push(Line::from(Span::styled(format!("  {}", s.activity_none), palette.dim())));
    } else {
        for a in rows.iter().take(80) {
            let kind = text(a, "type");
            let blood = text(a, "blood_type");
            let style = if blood != "-" {
                palette.err()
            } else if kind == "root" || kind == "system" {
                palette.ok()
            } else {
                palette.warn()
            };
            let what = if blood != "-" {
                format!("{blood} blood")
            } else {
                match kind.as_str() {
                    "blood" => "blood".to_string(),
                    "own" => "solved".to_string(),
                    other => format!("{other} flag"),
                }
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<24}", text(a, "user_name")), palette.value()),
                Span::styled(format!("{what:<14}"), style),
                Span::styled(text(a, "date_diff"), palette.dim()),
            ]));
        }
    }
    screens::page(terminal, ctx, &format!("{} · {title}", s.activity_title), lines, &screens::nav_text(ctx))
}

pub fn changelog_page<B: Backend<Error = std::io::Error>>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    kind: &str,
    id: &str,
    title: &str,
) -> io::Result<()> {
    let palette = ctx.palette();
    let s = ctx.strings();
    screens::loading(terminal, &palette, s.loading)?;
    let r = ctx.client.v4_get(&format!("/{kind}/changelog/{id}"));
    let rows = r.first_array(&["/info", "/data"]);
    let mut lines = vec![Line::from("")];
    if rows.is_empty() {
        lines.push(Line::from(Span::styled(format!("  {}", s.changelog_none), palette.dim())));
    } else {
        for c in &rows {
            let kind = match text(c, "type").as_str() {
                "1" | "enhancement" => "ENHANCEMENT",
                "2" | "info" => "INFO",
                "3" | "bug" => "BUG",
                other => other,
            }
            .to_string();
            lines.push(Line::from(vec![
                Span::styled(format!("  {}  ", date10(&text(c, "created_at"))), palette.dim()),
                Span::styled(format!("[{kind}] "), palette.warn()),
                Span::styled(text(c, "title"), palette.value()),
            ]));
            let desc = text(c, "description");
            if desc != "-" {
                lines.extend(screens::wrap_paragraph(&desc, &palette));
            }
            lines.push(Line::from(""));
        }
    }
    screens::page(terminal, ctx, &format!("{} · {title}", s.changelog_title), lines, &screens::nav_text(ctx))
}

pub fn walkthroughs<B: Backend<Error = std::io::Error>>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    id: &str,
    title: &str,
) -> io::Result<()> {
    let palette = ctx.palette();
    let s = ctx.strings();
    screens::loading(terminal, &palette, s.loading)?;
    let r = ctx.client.v4_get(&format!("/machine/walkthroughs/{id}"));
    let msg = r.data.pointer("/message").cloned().unwrap_or(Value::Null);
    let mut options: Vec<(String, String)> = Vec::new();
    let official = msg.get("official").cloned().unwrap_or(Value::Null);
    if text(&official, "filename") != "-" {
        options.push((format!("{}  (PDF)", s.walkthrough_official), "official".to_string()));
    }
    let video = msg.get("video").cloned().unwrap_or(Value::Null);
    let yt = text(&video, "youtube_id");
    if yt != "-" {
        options.push((
            format!("{}  {}  youtube", s.walkthrough_video, text(&video, "creator_name")),
            format!("https://www.youtube.com/watch?v={yt}"),
        ));
    }
    let community = msg.get("writeups").and_then(|w| w.as_array()).cloned().unwrap_or_default();
    let picker_title = if community.is_empty() {
        format!("{} · {title} · {}", s.walkthroughs_title, s.walkthrough_open)
    } else {
        format!("{} · {title} · {} {} · {}", s.walkthroughs_title, community.len().min(60), s.walkthrough_community.to_lowercase(), s.walkthrough_open)
    };
    for w in community.iter().take(60) {
        let lang = text(w, "language_name");
        let rating = w.get("rating").and_then(|v| v.as_u64()).unwrap_or(0);
        let votes = w.get("total_ratings").and_then(|v| v.as_u64()).unwrap_or(0);
        let label = format!(
            "{:<18} {}%  ({votes})  {}  {}",
            crate::commands::truncate(&text(w, "user_name"), 18),
            rating,
            date10(&text(w, "created_at")),
            if lang == "-" { String::new() } else { lang },
        );
        options.push((label, text(w, "url")));
    }
    if options.is_empty() {
        return screens::notice(terminal, ctx, s.walkthroughs_title, s.walkthrough_none, false);
    }
    loop {
        let Some(choice) = screens::pick_option(terminal, ctx, &picker_title, &options)? else {
            return Ok(());
        };
        let url = if choice == "official" {
            screens::loading(terminal, &palette, s.writeup_open)?;
            let w = ctx.client.v4_get_fresh(&format!("/machine/writeup/{id}"));
            let u = w.str_at("/url");
            if u == "-" {
                screens::action_result(terminal, ctx, s.walkthrough_official, &w.message(), false)?;
                continue;
            }
            u
        } else {
            choice
        };
        crate::clipboard::copy(&url);
        if !open_url(&url) {
            screens::copy_value(terminal, ctx, &url)?;
        }
    }
}

pub fn machine_info_page<B: Backend<Error = std::io::Error>>(
    terminal: &mut Terminal<B>,
    ctx: &Ctx,
    raw: &ApiResponse,
    id: &str,
) -> io::Result<()> {
    let palette = ctx.palette();
    let s = ctx.strings();
    screens::loading(terminal, &palette, s.loading)?;
    let name = raw.str_at("/info/name");
    let tags = ctx.client.v4_get(&format!("/machine/tags/{id}")).array_at("/info");
    let matrix = ctx.client.v4_get(&format!("/machine/graph/matrix/{id}"));
    let mut lines = vec![Line::from(""), Line::from(Span::styled(format!("  {name}"), palette.title())), Line::from("")];
    let synopsis = raw.str_at("/info/synopsis");
    if synopsis != "-" {
        lines.extend(screens::wrap_paragraph(&synopsis, &palette));
        lines.push(Line::from(""));
    }
    let blood_line = |who: &str, b: &Value| -> Option<Line<'static>> {
        let user = b.get("user").map(|u| text(u, "name")).unwrap_or_else(|| "-".into());
        if user == "-" {
            return None;
        }
        let when = text(b, "created_at");
        let diff = text(b, "blood_difference");
        Some(Line::from(vec![
            Span::styled(format!("  {who:<6}"), palette.label()),
            Span::styled(format!("{user:<24}"), palette.err()),
            Span::styled(
                format!("{}{}", date10(&when), if diff == "-" { String::new() } else { format!(" · {diff}") }),
                palette.dim(),
            ),
        ]))
    };
    let mut bloods = Vec::new();
    if let Some(b) = raw.data.pointer("/info/userBlood") {
        bloods.extend(blood_line(s.info_blood_user, b));
    }
    if let Some(b) = raw.data.pointer("/info/rootBlood") {
        bloods.extend(blood_line(s.info_blood_root, b));
    }
    if !bloods.is_empty() {
        lines.push(screens::section(s.info_bloods, &palette));
        lines.extend(bloods);
        lines.push(Line::from(""));
    }
    if !tags.is_empty() {
        lines.push(screens::section(s.info_tags, &palette));
        let mut groups: Vec<(String, Vec<String>)> = Vec::new();
        for t in &tags {
            let cat = text(t, "category");
            let name = text(t, "name");
            match groups.iter_mut().find(|(c, _)| *c == cat) {
                Some((_, list)) => list.push(name),
                None => groups.push((cat, vec![name])),
            }
        }
        for (cat, list) in groups {
            lines.push(Line::from(vec![
                Span::styled(format!("  {cat:<18}"), palette.label()),
                Span::styled(list.join(", "), palette.special_style()),
            ]));
        }
        lines.push(Line::from(""));
    }
    if let Some(agg) = matrix.data.pointer("/info/aggregate") {
        lines.push(screens::section(s.info_matrix, &palette));
        for key in ["enum", "real", "cve", "custom", "ctf"] {
            let v = agg.get(key).and_then(|x| x.as_f64()).unwrap_or(0.0);
            let width = (v.clamp(0.0, 10.0) * 3.0).round() as usize;
            lines.push(Line::from(vec![
                Span::styled(format!("  {key:<8}"), palette.label()),
                Span::styled("█".repeat(width), palette.accent_style()),
                Span::styled(format!(" {v:.1}"), palette.dim()),
            ]));
        }
        lines.push(Line::from(""));
    }
    let info_status = raw.str_at("/info/info_status");
    if info_status != "-" {
        lines.extend(screens::wrap_paragraph(&info_status, &palette));
    }
    screens::page(terminal, ctx, &format!("{} · {name}", s.info_title), lines, &screens::nav_text(ctx))
}

pub struct Live {
    pub ip: String,
    pub expires: String,
    pub spawning: bool,
}

pub fn live_status(ctx: &Ctx, id: &str) -> Option<Live> {
    let act = ctx.client.v4_get_fresh("/machine/active");
    if act.str_at("/info/id") != id {
        return None;
    }
    let ip = act.str_at("/info/ip");
    let spawning = ip == "-" || ip.is_empty();
    Some(Live { ip, expires: act.str_at("/info/expires_at"), spawning })
}

pub enum SearchPick {
    Machine(String),
    Challenge(String),
    User(String),
    Team(String),
}

type SearchGroup<'a> = (&'a str, &'a str, fn(String) -> SearchPick);

pub fn global_search<B: Backend<Error = std::io::Error>>(terminal: &mut Terminal<B>, ctx: &Ctx) -> io::Result<Option<SearchPick>> {
    let palette = ctx.palette();
    let s = ctx.strings();
    let Some(term) = ask(
        terminal,
        &palette,
        Prompt::new(s.search_global_title, s.prompt_search_label).hint(s.prompt_search_hint).paste_hint(s.paste_hint),
    )?
    else {
        return Ok(None);
    };
    let term = term.trim().to_string();
    if term.is_empty() {
        return Ok(None);
    }
    screens::loading(terminal, &palette, s.loading)?;
    let r = ctx.client.v4_get(&format!("/search/fetch?query={}", urlencode(&term)));
    let mut items: Vec<Item> = Vec::new();
    let mut picks: Vec<SearchPick> = Vec::new();
    let groups: [SearchGroup; 4] = [
        ("machines", s.search_machines, SearchPick::Machine),
        ("challenges", s.search_challenges, SearchPick::Challenge),
        ("users", s.search_users, SearchPick::User),
        ("teams", s.search_teams, SearchPick::Team),
    ];
    for (key, label, mk) in groups {
        for v in r.array_at(&format!("/{key}")) {
            let name = text(&v, "value");
            let extra = if key == "machines" && flag(&v, "isSp") { "  Starting Point" } else { "" };
            items.push(Item::plain_line(Line::from(vec![
                Span::styled(format!("  {label:<10} "), palette.label()),
                Span::styled(format!("{name}{extra}"), palette.value()),
            ])));
            picks.push(mk(id_of(&v)));
        }
    }
    if items.is_empty() {
        screens::notice(terminal, ctx, s.search_global_title, s.no_results, false)?;
        return Ok(None);
    }
    let mut picker = Picker::new(items);
    loop {
        let ctx_ref: &Ctx = ctx;
        let position = picker.position_label();
        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, &format!("{}: {term}", s.search_global_title), &palette);
            picker.render(f, body_a, &position, &palette);
            screens::render_bars(f, footer_a, &[], ctx_ref);
        })?;
        let Some(input) = event::read(event::POLL)? else { continue };
        if picker.capture(&input) {
            continue;
        }
        if input == Input::Quit {
            return Ok(None);
        }
        match picker.handle(&input) {
            Outcome::Leave => return Ok(None),
            Outcome::Chosen(i) => {
                return Ok(picks.get(i).map(|p| match p {
                    SearchPick::Machine(x) => SearchPick::Machine(x.clone()),
                    SearchPick::Challenge(x) => SearchPick::Challenge(x.clone()),
                    SearchPick::User(x) => SearchPick::User(x.clone()),
                    SearchPick::Team(x) => SearchPick::Team(x.clone()),
                }))
            }
            _ => {}
        }
    }
}

pub fn user_page<B: Backend<Error = std::io::Error>>(terminal: &mut Terminal<B>, ctx: &Ctx, uid: &str) -> io::Result<()> {
    let palette = ctx.palette();
    let s = ctx.strings();
    screens::loading(terminal, &palette, s.loading)?;
    let b = ctx.client.v4_get(&format!("/user/profile/basic/{uid}"));
    let name = b.str_at("/profile/name");
    if name == "-" {
        return screens::notice(terminal, ctx, s.user_view_title, screens::api_problem(b.status, ctx), false);
    }
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(format!("  {name}"), palette.title())),
        Line::from(""),
        screens::field_styled(s.profile_rank, &b.str_at("/profile/rank"), &palette, palette.warn()),
        screens::field(s.stats_global_rank, &format!("#{}", b.str_at("/profile/ranking")), &palette),
        screens::field(s.profile_points, &b.str_at("/profile/points"), &palette),
        screens::field(
            s.profile_owns,
            &format!("user {} · system {}", b.str_at("/profile/user_owns"), b.str_at("/profile/system_owns")),
            &palette,
        ),
        screens::field(
            s.profile_bloods,
            &format!(
                "user {} · system {} · retos {}",
                b.str_at("/profile/user_bloods"),
                b.str_at("/profile/system_bloods"),
                b.str_at("/profile/challenge_bloods")
            ),
            &palette,
        ),
        screens::field(s.profile_respects, &b.str_at("/profile/respects"), &palette),
        screens::field(s.profile_country, &b.str_at("/profile/country_name"), &palette),
        screens::field(s.profile_team, &b.str_at("/profile/team/name"), &palette),
        screens::field(s.profile_member_since, &date10(&b.str_at("/profile/joined_date")), &palette),
        Line::from(""),
        Line::from(Span::styled(format!("  {WEB}/users/{uid}"), palette.dim())),
    ];
    let desc = b.str_at("/profile/description");
    if desc != "-" {
        lines.push(Line::from(""));
        lines.extend(screens::wrap_paragraph(&desc, &palette));
    }
    screens::page(terminal, ctx, &format!("{} · {name}", s.user_view_title), lines, &screens::nav_text(ctx))
}

pub enum DashPick {
    Machine(String),
    Sherlock(String),
    Challenge(String),
}

pub fn dashboard<B: Backend<Error = std::io::Error>>(terminal: &mut Terminal<B>, ctx: &Ctx, which: &str, title: &str) -> io::Result<Option<DashPick>> {
    let palette = ctx.palette();
    let s = ctx.strings();
    screens::loading(terminal, &palette, s.loading)?;
    let r = ctx.client.v5_get_fresh(&format!("/user/dashboard/{which}"));
    let mut items: Vec<Item> = Vec::new();
    let mut picks: Vec<DashPick> = Vec::new();
    let groups: [(&str, &str); 4] = [
        ("machines", "machine"),
        ("startingPoints", "machine"),
        ("sherlocks", "sherlock"),
        ("challenges", "challenge"),
    ];
    for (key, kind) in groups {
        for v in r.array_at(&format!("/{key}")) {
            let diff = text(&v, "difficulty");
            let os = text(&v, "os");
            let progress = v.get("progress").and_then(|p| p.as_f64()).unwrap_or(0.0);
            let tasks = format!("{}/{}", text(&v, "tasksCompleted"), text(&v, "tasksTotal"));
            let user = flag(&v, "user_flag");
            let root = flag(&v, "root_flag");
            let mark = match (user, root) {
                (true, true) => "✓✓",
                (true, false) | (false, true) => "✓ ",
                _ => "  ",
            };
            items.push(Item::with_detail(
                Line::from(vec![
                    Span::styled(format!("{mark} "), if root { palette.ok() } else { palette.warn() }),
                    Span::styled(format!("{:<9}", if key == "startingPoints" { "sp" } else { kind }), palette.label()),
                    Span::styled(format!("{:<28}", text(&v, "name")), palette.value()),
                    Span::styled(format!("{:<9}", if os == "-" { String::new() } else { os }), palette.info_style()),
                    Span::styled(format!("{diff:<10}"), palette.difficulty(&diff)),
                ]),
                Line::from(Span::styled(
                    format!(
                        "      {} {progress:.0}% · {} {tasks} · {}",
                        s.field_progress,
                        s.field_tasks,
                        rating_text(&v)
                    ),
                    palette.dim(),
                )),
            ));
            picks.push(match kind {
                "sherlock" => DashPick::Sherlock(id_of(&v)),
                "challenge" => DashPick::Challenge(id_of(&v)),
                _ => DashPick::Machine(id_of(&v)),
            });
        }
    }
    if items.is_empty() {
        screens::notice(terminal, ctx, title, s.dashboard_empty, false)?;
        return Ok(None);
    }
    let mut picker = Picker::new(items);
    loop {
        let ctx_ref: &Ctx = ctx;
        let position = picker.position_label();
        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, title, &palette);
            picker.render(f, body_a, &position, &palette);
            screens::render_bars(f, footer_a, &[], ctx_ref);
        })?;
        let Some(input) = event::read(event::POLL)? else { continue };
        if picker.capture(&input) {
            continue;
        }
        if input == Input::Quit {
            return Ok(None);
        }
        match picker.handle(&input) {
            Outcome::Leave => return Ok(None),
            Outcome::Chosen(i) => {
                return Ok(picks.get(i).map(|p| match p {
                    DashPick::Machine(x) => DashPick::Machine(x.clone()),
                    DashPick::Sherlock(x) => DashPick::Sherlock(x.clone()),
                    DashPick::Challenge(x) => DashPick::Challenge(x.clone()),
                }))
            }
            _ => {}
        }
    }
}

pub fn season_leaderboard<B: Backend<Error = std::io::Error>>(terminal: &mut Terminal<B>, ctx: &Ctx, sid: &str, season_name: &str) -> io::Result<()> {
    let s = ctx.strings();
    let me = ctx.client.v4_get(&format!("/season/user/rank/{sid}"));
    let my_rank = me.str_at("/data/rank");
    let mut page: u32 = 1;
    loop {
        let palette = ctx.palette();
        screens::loading(terminal, &palette, s.loading)?;
        let r = ctx.client.v4_get(&format!("/season/players/leaderboard?per_page=15&season={sid}&page={page}"));
        let rows = r.array_at("/data");
        let last = r.data.pointer("/meta/last_page").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
        let mut lines = vec![Line::from("")];
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<6}{:<24}{:<10}{:>7}{:>7}{:>7}{:>7}", "#", "", "tier", "pts", "user", "root", "bloods"), palette.label()),
        ]));
        for p in &rows {
            let rank = text(p, "rank");
            let mine = rank == my_rank;
            let style = if mine { palette.accent_style() } else { palette.value() };
            let bloods = p.get("user_bloods").and_then(|v| v.as_u64()).unwrap_or(0)
                + p.get("root_bloods").and_then(|v| v.as_u64()).unwrap_or(0);
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<6}", rank), if mine { palette.accent_style() } else { palette.warn() }),
                Span::styled(format!("{:<24}", crate::commands::truncate(&text(p, "name"), 22)), style),
                Span::styled(format!("{:<10}", text(p, "league_rank")), palette.special_style()),
                Span::styled(format!("{:>7}", text(p, "points")), palette.value()),
                Span::styled(format!("{:>7}", text(p, "user_owns")), palette.dim()),
                Span::styled(format!("{:>7}", text(p, "root_owns")), palette.dim()),
                Span::styled(format!("{:>7}", bloods), palette.err()),
            ]));
        }
        if my_rank != "-" {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(format!("  #{my_rank}  "), palette.accent_style()),
                Span::styled(
                    format!(
                        "{} · {} · {} {} · {} {}",
                        s.season_you,
                        me.str_at("/data/league"),
                        me.str_at("/data/total_season_points"),
                        s.season_points.to_lowercase(),
                        me.str_at("/data/total_season_flags/obtained"),
                        s.season_flags.to_lowercase()
                    ),
                    palette.value(),
                ),
            ]));
        }
        let title = format!("{} · {season_name} · {} {page}/{last}", s.season_leaderboard, s.action_page);
        let ctx_ref: &Ctx = ctx;
        let actions = [("n", s.page_next), ("p", s.page_previous)];
        let mut scroll: u16 = 0;
        let outcome = loop {
            let height = terminal.size().map(|x| x.height).unwrap_or(24);
            let viewport = height.saturating_sub(7);
            let max_scroll = (lines.len() as u16).saturating_sub(viewport);
            let body = lines.clone();
            let t = title.clone();
            terminal.draw(|f| {
                let area = f.area();
                screens::paint_background(f, &palette);
                let (title_a, body_a, footer_a) = screens::frame_layout(area);
                screens::render_title(f, title_a, &t, &palette);
                let block = ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_style(palette.border_style());
                f.render_widget(ratatui::widgets::Paragraph::new(body).block(block).scroll((scroll, 0)), body_a);
                screens::render_bars(f, footer_a, &actions, ctx_ref);
            })?;
            let Some(input) = event::read(event::POLL)? else { continue };
            match input {
                Input::Up => scroll = scroll.saturating_sub(1),
                Input::Down => scroll = (scroll + 1).min(max_scroll),
                Input::PageUp => scroll = scroll.saturating_sub(viewport.max(1)),
                Input::PageDown => scroll = (scroll + viewport.max(1)).min(max_scroll),
                Input::Home => scroll = 0,
                Input::End => scroll = max_scroll,
                Input::Back | Input::Quit => break 0,
                Input::Key('n') | Input::Key('N') => break 1,
                Input::Key('p') | Input::Key('P') => break 2,
                _ => {}
            }
        };
        match outcome {
            0 => return Ok(()),
            1 if page < last => page += 1,
            2 if page > 1 => page -= 1,
            _ => {}
        }
    }
}

pub fn season_rewards<B: Backend<Error = std::io::Error>>(terminal: &mut Terminal<B>, ctx: &Ctx, sid: &str, season_name: &str) -> io::Result<()> {
    let palette = ctx.palette();
    let s = ctx.strings();
    screens::loading(terminal, &palette, s.loading)?;
    let r = ctx.client.v4_get(&format!("/season/rewards/{sid}"));
    let mut lines = vec![Line::from("")];
    for block in r.array_at("/data") {
        let Some(types) = block.get("reward_types") else { continue };
        let list: Vec<Value> = match types {
            Value::Array(a) => a.clone(),
            Value::Object(_) => vec![types.clone()],
            _ => Vec::new(),
        };
        for t in list {
            lines.push(screens::section(&text(&t, "name"), &palette));
            let desc = text(&t, "description");
            if desc != "-" {
                lines.extend(screens::wrap_paragraph(&desc, &palette));
            }
            for g in t.get("groups").and_then(|v| v.as_array()).cloned().unwrap_or_default() {
                let req = text(&g, "rank_up_requirement");
                let flags = text(&g, "flags_needed");
                lines.push(Line::from(vec![
                    Span::styled(format!("  {:<12}", text(&g, "name")), palette.special_style()),
                    Span::styled(format!("{:<16}", text(&g, "subtitle")), palette.label()),
                    Span::styled(
                        format!(
                            "{}{}",
                            if req == "-" { String::new() } else { format!("top {req}  ") },
                            if flags == "-" { String::new() } else { format!("{flags} {}", s.season_flags.to_lowercase()) }
                        ),
                        palette.dim(),
                    ),
                ]));
                for rw in g.get("rewards").and_then(|v| v.as_array()).cloned().unwrap_or_default() {
                    let n = text(&rw, "name");
                    if n != "-" {
                        lines.push(screens::bullet(&n, &palette));
                    }
                }
            }
            lines.push(Line::from(""));
        }
    }
    if lines.len() == 1 {
        lines.push(Line::from(Span::styled(format!("  {}", s.no_results), palette.dim())));
    }
    screens::page(terminal, ctx, &format!("{} · {season_name}", s.season_rewards), lines, &screens::nav_text(ctx))
}

pub fn rankings<B: Backend<Error = std::io::Error>>(terminal: &mut Terminal<B>, ctx: &Ctx, kind: &str, title: &str) -> io::Result<()> {
    let palette = ctx.palette();
    let s = ctx.strings();
    screens::loading(terminal, &palette, s.loading)?;
    let r = ctx.client.v4_get(&format!("/rankings/{kind}"));
    let rows = r.array_at("/data");
    let mut lines = vec![Line::from("")];
    let head = match kind {
        "countries" => format!("  {:<6}{:<28}{:>9}{:>10}{:>8}{:>8}", "#", "", "members", "pts", "user", "root"),
        "universities" => format!("  {:<6}{:<28}{:>9}{:>10}{:>8}{:>8}", "#", "", "students", "pts", "user", "root"),
        _ => format!("  {:<6}{:<28}{:>9}{:>10}{:>8}{:>8}", "#", "", "", "pts", "user", "root"),
    };
    lines.push(Line::from(Span::styled(head, palette.label())));
    for e in rows.iter().take(150) {
        let name = if kind == "countries" {
            let cn = text(e, "country_name");
            if cn == "-" { text(e, "country") } else { format!("{}  {cn}", text(e, "country")) }
        } else {
            text(e, "name")
        };
        let third = match kind {
            "countries" => text(e, "members"),
            "universities" => text(e, "students"),
            _ => String::new(),
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<6}", text(e, "rank")), palette.warn()),
            Span::styled(format!("{:<28}", crate::commands::truncate(&name, 26)), palette.value()),
            Span::styled(format!("{third:>9}"), palette.dim()),
            Span::styled(format!("{:>10}", text(e, "points")), palette.accent_style()),
            Span::styled(format!("{:>8}", text(e, "user_owns")), palette.dim()),
            Span::styled(format!("{:>8}", text(e, "root_owns")), palette.dim()),
        ]));
    }
    if rows.is_empty() {
        lines.push(Line::from(Span::styled(format!("  {}", screens::api_problem(r.status, ctx)), palette.err())));
    }
    screens::page(terminal, ctx, title, lines, &screens::nav_text(ctx))
}

pub fn pwnbox_page<B: Backend<Error = std::io::Error>>(terminal: &mut Terminal<B>, ctx: &Ctx) -> io::Result<()> {
    let palette = ctx.palette();
    let s = ctx.strings();
    screens::loading(terminal, &palette, s.loading)?;
    let status = ctx.client.v4_get_fresh("/pwnbox/status");
    let usage = ctx.client.v4_get_fresh("/pwnbox/usage");
    let mut lines = vec![Line::from("")];
    let st = status.str_at("/status");
    if st == "-" && status.str_at("/message") != "-" {
        lines.push(Line::from(Span::styled(format!("  {}", s.pwnbox_none), palette.dim())));
        lines.push(Line::from(Span::styled(format!("  {}", status.str_at("/message")), palette.dim())));
    } else {
        lines.push(screens::field_styled(s.vm_status, &st, &palette, palette.ok()));
        for key in ["hostname", "vnc_view_only_password", "vnc_password", "expires_at", "location"] {
            let v = status.str_at(&format!("/{key}"));
            if v != "-" {
                lines.push(screens::field(key, &v, &palette));
            }
        }
    }
    lines.push(Line::from(""));
    lines.push(screens::field(
        s.pwnbox_usage,
        &format!("{} / {}", usage.str_at("/used"), usage.str_at("/allowed")),
        &palette,
    ));
    lines.push(screens::field("sessions", &usage.str_at("/sessions"), &palette));
    lines.push(Line::from(""));
    screens::page(terminal, ctx, s.pwnbox_labs, lines, &screens::nav_text(ctx))
}
