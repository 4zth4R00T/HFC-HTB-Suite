use crate::tui::screens;
use crate::tui::theme::Palette;
use crate::tui::Ctx;
use ratatui::backend::Backend;
use ratatui::text::{Line, Span};
use ratatui::Terminal;
use serde_json::Value;
use std::io;

const BAR_WIDTH: usize = 26;
const LABEL_WIDTH: usize = 22;

fn number(value: &Value, key: &str) -> i64 {
    value
        .get(key)
        .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0)
}

fn bar(done: i64, total: i64, palette: &Palette, tint: ratatui::style::Style) -> Vec<Span<'static>> {
    let ratio = if total > 0 {
        (done as f64 / total as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let filled = (ratio * BAR_WIDTH as f64).round() as usize;
    vec![
        Span::styled("█".repeat(filled), tint),
        Span::styled("░".repeat(BAR_WIDTH - filled), palette.dim()),
        Span::styled(format!("  {done}/{total}"), palette.value()),
        Span::styled(format!("  {:.0}%", ratio * 100.0), palette.label()),
    ]
}

fn stat_field(label: &str, value: &str, ctx: &Ctx, tint: ratatui::style::Style) -> Line<'static> {
    let palette = ctx.palette();
    Line::from(vec![
        Span::styled(
            format!("  {:<width$}", trim_label(label), width = LABEL_WIDTH),
            palette.label(),
        ),
        Span::styled(value.to_string(), tint),
    ])
}

fn trim_label(label: &str) -> String {
    if label.chars().count() > LABEL_WIDTH - 1 {
        label.chars().take(LABEL_WIDTH - 2).collect::<String>() + "…"
    } else {
        label.to_string()
    }
}

fn bar_row(label: &str, done: i64, total: i64, ctx: &Ctx, tint: ratatui::style::Style) -> Line<'static> {
    let palette = ctx.palette();
    let mut spans = vec![Span::styled(
        format!("  {:<width$}", trim_label(label), width = LABEL_WIDTH),
        palette.label(),
    )];
    spans.extend(bar(done, total, &palette, tint));
    Line::from(spans)
}

pub fn run<B: Backend>(terminal: &mut Terminal<B>, ctx: &Ctx) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    screens::loading(terminal, &palette, strings.loading)?;

    let info = ctx.client.v4_get("/user/info");
    if !info.ok {
        return screens::notice(
            terminal,
            ctx,
            strings.stats_title,
            screens::api_problem(info.status, ctx),
            false,
        );
    }
    let uid = info.str_at("/info/id");
    let basic = ctx.client.v4_get(&format!("/user/profile/basic/{uid}"));
    let machines = ctx.client.v4_get(&format!("/user/profile/progress/machines/{uid}"));
    let challenges = ctx.client.v4_get(&format!("/user/profile/progress/challenges/{uid}"));
    let sherlocks = ctx.client.v4_get(&format!("/user/profile/progress/sherlocks/{uid}"));
    let fortresses = ctx.client.v4_get(&format!("/user/profile/progress/fortress/{uid}"));
    let prolabs = ctx.client.v4_get(&format!("/user/profile/progress/prolab/{uid}"));
    let dashboard = ctx.client.v4_get("/user/dashboard");
    let leaderboard = ctx.client.v4_get("/rankings/users");

    let mut lines: Vec<Line<'static>> = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", basic.str_at("/profile/name")),
            palette.title(),
        )),
        Line::from(""),
    ];

    let rank = basic.str_at("/profile/rank");
    let position = basic.str_at("/profile/ranking");
    let points: i64 = basic.str_at("/profile/points").parse().unwrap_or(0);
    lines.push(stat_field(strings.profile_rank, &rank, ctx, palette.warn()));
    lines.push(screens::field(strings.stats_global_rank, &format!("#{position}"), &palette));
    lines.push(screens::field(strings.profile_points, &points.to_string(), &palette));

    let progress: i64 = basic.str_at("/profile/current_rank_progress").parse().unwrap_or(0);
    let gap: f64 = basic.str_at("/profile/next_rank_points").parse().unwrap_or(0.0);
    lines.push(bar_row(
        &basic.str_at("/profile/next_rank"),
        progress,
        100,
        ctx,
        palette.accent_style(),
    ));
    if gap > 0.0 {
        lines.push(Line::from(Span::styled(
            format!(
                "  {} {:.0} {}",
                strings.stats_next_rank_gap, gap, strings.stats_points_away
            ),
            palette.dim(),
        )));
    }

    lines.push(Line::from(""));
    lines.push(screens::section(strings.stats_machines, &palette));
    let solved = machines
        .data
        .pointer("/profile/machine_owns")
        .cloned()
        .unwrap_or(Value::Null);
    lines.push(bar_row(
        strings.stats_solved,
        number(&solved, "solved"),
        number(&solved, "total"),
        ctx,
        palette.accent_style(),
    ));
    for entry in machines.array_at("/profile/machine_difficulties") {
        let name = entry.get("name").and_then(|v| v.as_str()).unwrap_or("?");
        lines.push(bar_row(
            name,
            number(&entry, "owned_machines"),
            number(&entry, "total_machines"),
            ctx,
            palette.difficulty(name),
        ));
    }

    lines.push(Line::from(""));
    lines.push(screens::section(strings.stats_challenges, &palette));
    let solved = challenges
        .data
        .pointer("/profile/challenge_owns")
        .cloned()
        .unwrap_or(Value::Null);
    lines.push(bar_row(
        strings.stats_solved,
        number(&solved, "solved"),
        number(&solved, "total"),
        ctx,
        palette.special_style(),
    ));
    let mut categories: Vec<Value> = challenges
        .array_at("/profile/challenge_categories")
        .into_iter()
        .filter(|c| number(c, "owned_flags") > 0 || number(c, "owns") > 0)
        .collect();
    categories.sort_by_key(|c| -(number(c, "owned_flags") + number(c, "owns")));
    for entry in categories.iter().take(8) {
        let name = entry.get("name").and_then(|v| v.as_str()).unwrap_or("?");
        let done = number(entry, "owned_flags").max(number(entry, "owns"));
        let total = number(entry, "total_flags").max(number(entry, "total"));
        lines.push(bar_row(name, done, total, ctx, palette.special_style()));
    }

    let solved = sherlocks
        .data
        .pointer("/profile/challenge_owns")
        .cloned()
        .unwrap_or(Value::Null);
    if number(&solved, "total") > 0 {
    lines.push(Line::from(""));
    lines.push(screens::section(strings.stats_sherlocks, &palette));
    lines.push(bar_row(
        strings.stats_solved,
        number(&solved, "solved"),
        number(&solved, "total"),
        ctx,
        palette.info_style(),
    ));
    for entry in sherlocks.array_at("/profile/challenge_categories") {
        if number(&entry, "owned_flags") == 0 {
            continue;
        }
        let name = entry.get("name").and_then(|v| v.as_str()).unwrap_or("?");
        lines.push(bar_row(
            name,
            number(&entry, "owned_flags"),
            number(&entry, "total_flags"),
            ctx,
            palette.info_style(),
        ));
    }
    }

    let started: Vec<Value> = fortresses
        .array_at("/profile/fortresses")
        .into_iter()
        .filter(|f| number(f, "completion_percentage") > 0)
        .collect();
    if !started.is_empty() {
        lines.push(Line::from(""));
        lines.push(screens::section(strings.stats_fortress, &palette));
        for entry in started {
            let name = entry.get("name").and_then(|v| v.as_str()).unwrap_or("?");
            lines.push(bar_row(
                name,
                number(&entry, "owned_flags"),
                number(&entry, "total_flags"),
                ctx,
                palette.err(),
            ));
        }
    }

    let started: Vec<Value> = prolabs
        .array_at("/profile/prolabs")
        .into_iter()
        .filter(|l| number(l, "owned_flags") > 0)
        .collect();
    if !started.is_empty() {
        lines.push(Line::from(""));
        lines.push(screens::section(strings.stats_prolabs, &palette));
        for entry in started {
            let name = entry.get("name").and_then(|v| v.as_str()).unwrap_or("?");
            lines.push(bar_row(
                name,
                number(&entry, "owned_flags"),
                number(&entry, "total_flags"),
                ctx,
                palette.warm_style(),
            ));
        }
    }

    let seasons = ctx.client.v4_get("/season/list");
    let active = seasons
        .array_at("/data")
        .into_iter()
        .find(|s| s.get("active").and_then(|v| v.as_bool()) == Some(true));
    if let Some(season) = active {
        let sid = season
            .get("id")
            .map(|v| v.to_string())
            .unwrap_or_default();
        let rank = ctx.client.v4_get(&format!("/season/user/rank/{sid}"));
        lines.push(Line::from(""));
        lines.push(screens::section(strings.profile_section_season, &palette));
        lines.push(stat_field(strings.season_name, &season.get("name").and_then(|v| v.as_str()).unwrap_or("-"), ctx, palette.value()));
        let league = rank.str_at("/data/league");
        let tier = if league == "-" {
            format!("→ {}", rank.str_at("/data/next_rank/title"))
        } else {
            league
        };
        lines.push(stat_field(strings.season_tier, &tier, ctx, palette.special_style()));
        let obtained: i64 = rank
            .str_at("/data/total_season_flags/obtained")
            .parse()
            .unwrap_or(0);
        let total: i64 = rank
            .str_at("/data/total_season_flags/total")
            .parse()
            .unwrap_or(0);
        lines.push(bar_row(
            strings.season_flags,
            obtained,
            total,
            ctx,
            palette.ok(),
        ));
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<width$}", strings.stats_streak, width = LABEL_WIDTH),
                palette.label(),
            ),
            Span::styled(
                "▮".repeat(obtained.clamp(0, 40) as usize),
                palette.accent_style(),
            ),
            Span::styled(format!("  {obtained}"), palette.value()),
        ]));
        lines.push(Line::from(Span::styled(
            format!("  {:<width$}{}", "", strings.stats_streak_note, width = LABEL_WIDTH),
            palette.dim(),
        )));
        let to_next = rank.str_at("/data/flags_to_next_rank/total");
        if to_next != "-" {
            lines.push(screens::field(
                strings.stats_next_rank_gap,
                &format!(
                    "{}/{} {}",
                    rank.str_at("/data/flags_to_next_rank/obtained"),
                    to_next,
                    strings.season_flags.to_lowercase()
                ),
                &palette,
            ));
        }
    }

    lines.push(Line::from(""));
    lines.push(screens::section(strings.stats_leaderboard, &palette));
    let online = dashboard.str_at("/dashboard_players/online_players");
    if online != "-" {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<width$}", strings.stats_online, width = LABEL_WIDTH),
                palette.label(),
            ),
            Span::styled(online, palette.value()),
        ]));
    }
    for entry in leaderboard.array_at("/data").iter().take(5) {
        let name = entry
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string();
        lines.push(Line::from(vec![
            Span::styled(format!("    #{:<4}", number(entry, "rank")), palette.warn()),
            Span::styled(format!("{name:<24}"), palette.value()),
            Span::styled(format!("{} pts", number(entry, "points")), palette.label()),
        ]));
    }
    lines.push(Line::from(vec![
        Span::styled(format!("    #{position:<4}"), palette.accent_style()),
        Span::styled(
            format!("{:<24}", basic.str_at("/profile/name")),
            palette.accent_style(),
        ),
        Span::styled(format!("{points} pts"), palette.label()),
    ]));

    screens::page(terminal, ctx, strings.stats_title, lines, &screens::nav_text(ctx))
}
