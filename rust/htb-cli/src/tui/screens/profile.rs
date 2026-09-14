use crate::tui::screens;
use crate::tui::Ctx;
use ratatui::backend::Backend;
use ratatui::text::{Line, Span};
use ratatui::Terminal;
use std::io;

fn round_points(raw: &str) -> String {
    match raw.parse::<f64>() {
        Ok(v) if v.fract().abs() < f64::EPSILON => format!("{v:.0}"),
        Ok(v) => format!("{v:.2}"),
        Err(_) => raw.to_string(),
    }
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
            strings.profile_title,
            screens::api_problem(info.status, ctx),
            false,
        );
    }
    let uid = info.str_at("/info/id");
    let basic = ctx.client.v4_get(&format!("/user/profile/basic/{uid}"));

    let nick = basic.str_at("/profile/name");
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(format!("  {nick}"), palette.title())),
        Line::from(""),
        screens::section(strings.profile_section_account, &palette),
        screens::field_styled(
            strings.profile_rank,
            &format!(
                "{} (#{})",
                basic.str_at("/profile/rank"),
                basic.str_at("/profile/ranking")
            ),
            &palette,
            palette.warn(),
        ),
        screens::field(
            strings.profile_next_rank,
            &format!(
                "{} — {}% ({} pts)",
                basic.str_at("/profile/next_rank"),
                basic.str_at("/profile/current_rank_progress"),
                round_points(&basic.str_at("/profile/next_rank_points"))
            ),
            &palette,
        ),
        screens::field_styled(
            strings.profile_points,
            &basic.str_at("/profile/points"),
            &palette,
            palette.ok(),
        ),
        screens::field(
            strings.profile_owns,
            &format!(
                "user {} · system {}",
                basic.str_at("/profile/user_owns"),
                basic.str_at("/profile/system_owns")
            ),
            &palette,
        ),
        screens::field(
            strings.profile_bloods,
            &format!(
                "user {} · system {} · challenge {}",
                basic.str_at("/profile/user_bloods"),
                basic.str_at("/profile/system_bloods"),
                basic.str_at("/profile/challenge_bloods")
            ),
            &palette,
        ),
        screens::field(
            strings.profile_respects,
            &basic.str_at("/profile/respects"),
            &palette,
        ),
        screens::field(
            strings.profile_country,
            &basic.str_at("/profile/country_name"),
            &palette,
        ),
        screens::field_styled(
            strings.profile_server,
            &basic.str_at("/profile/server"),
            &palette,
            palette.info_style(),
        ),
        screens::field(
            strings.profile_member_since,
            &basic
                .str_at("/profile/joined_date")
                .chars()
                .take(10)
                .collect::<String>(),
            &palette,
        ),
        screens::field(strings.profile_team, &basic.str_at("/profile/team"), &palette),
        Line::from(""),
        screens::section(strings.profile_section_subscription, &palette),
        screens::field_styled(
            strings.profile_plan,
            &info.str_at("/info/subscriptionType"),
            &palette,
            palette.warn(),
        ),
        screens::field(
            strings.profile_vip,
            if info.str_at("/info/canAccessVIP") == "true" {
                strings.yes
            } else {
                strings.no
            },
            &palette,
        ),
    ];

    let subs = ctx
        .client
        .v4_get("/user/subscriptions")
        .array_at("/subscriptions");
    if subs.is_empty() {
        lines.push(screens::bullet(strings.subscription_none, &palette));
        lines.push(screens::bullet(crate::http::STORE_URL, &palette));
    } else {
        for s in subs {
            lines.push(screens::bullet(
                &format!(
                    "{} ({})",
                    s.get("name").and_then(|v| v.as_str()).unwrap_or("?"),
                    s.get("status").and_then(|v| v.as_str()).unwrap_or("?")
                ),
                &palette,
            ));
        }
    }

    lines.push(Line::from(""));
    lines.push(screens::section(strings.profile_section_season, &palette));
    let list = ctx.client.v4_get("/season/list");
    let active = list
        .array_at("/data")
        .into_iter()
        .find(|s| s.get("active").and_then(|v| v.as_bool()) == Some(true));
    match active {
        None => lines.push(screens::bullet(strings.season_none, &palette)),
        Some(a) => {
            let sid = a
                .get("id")
                .map(|v| v.to_string())
                .unwrap_or_default();
            let rank = ctx.client.v4_get(&format!("/season/user/rank/{sid}"));
            let state = a.get("state").and_then(|v| v.as_str()).unwrap_or("");
            lines.push(screens::field(
                strings.season_name,
                &format!(
                    "{}{}",
                    a.get("name").and_then(|v| v.as_str()).unwrap_or("-"),
                    if state.is_empty() {
                        String::new()
                    } else {
                        format!("  ({state})")
                    }
                ),
                &palette,
            ));
            let league = rank.str_at("/data/league");
            let tier = if league == "-" {
                let next = rank.str_at("/data/next_rank/title");
                if next == "-" {
                    league
                } else {
                    format!("→ {next}")
                }
            } else {
                league
            };
            lines.push(screens::field_styled(
                strings.season_tier,
                &tier,
                &palette,
                palette.special_style(),
            ));
            let position = rank.str_at("/data/rank");
            if position != "-" {
                lines.push(screens::field(
                    strings.season_rank,
                    &format!("{position}/{}", rank.str_at("/data/total_ranks")),
                    &palette,
                ));
            }
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
            lines.push(screens::field(
                strings.profile_owns,
                &format!(
                    "user {} · root {}",
                    rank.str_at("/data/user_owns"),
                    rank.str_at("/data/root_owns")
                ),
                &palette,
            ));
        }
    }

    screens::page(terminal, ctx, strings.profile_title, lines, &screens::nav_text(ctx))
}
