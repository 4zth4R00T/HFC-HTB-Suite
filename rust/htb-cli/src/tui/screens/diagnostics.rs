use crate::http::{BASE_V4, BASE_V5};
use crate::tui::screens;
use crate::tui::theme::Palette;
use crate::tui::Ctx;
use ratatui::backend::Backend;
use ratatui::text::{Line, Span};
use ratatui::Terminal;
use std::io;

struct Probe {
    label: &'static str,
    base: &'static str,
    path: String,
}

pub fn run<B: Backend>(terminal: &mut Terminal<B>, ctx: &Ctx) -> io::Result<()> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    screens::loading(terminal, &palette, strings.diag_running)?;

    let uid = ctx.client.v4_get("/user/info").str_at("/info/id");
    let season = ctx
        .client
        .v4_get("/season/list")
        .array_at("/data")
        .into_iter()
        .find(|s| s.get("active").and_then(|v| v.as_bool()) == Some(true))
        .and_then(|s| s.get("id").map(|v| v.to_string()))
        .unwrap_or_else(|| "1".to_string());

    let probes = vec![
        Probe { label: "v4 /user/info", base: BASE_V4, path: "/user/info".into() },
        Probe { label: "v4 /user/profile/basic", base: BASE_V4, path: format!("/user/profile/basic/{uid}") },
        Probe { label: "v4 /user/subscriptions", base: BASE_V4, path: "/user/subscriptions".into() },
        Probe { label: "v5 /machines", base: BASE_V5, path: "/machines?per_page=1".into() },
        Probe { label: "v4 /machine/active", base: BASE_V4, path: "/machine/active".into() },
        Probe { label: "v4 /connections/servers", base: BASE_V4, path: "/connections/servers?product=labs".into() },
        Probe { label: "v4 /challenge/list", base: BASE_V4, path: "/challenge/list".into() },
        Probe { label: "v4 /challenge/categories", base: BASE_V4, path: "/challenge/categories/list".into() },
        Probe { label: "v4 /prolabs", base: BASE_V4, path: "/prolabs".into() },
        Probe { label: "v4 /fortresses", base: BASE_V4, path: "/fortresses".into() },
        Probe { label: "v4 /sherlocks", base: BASE_V4, path: "/sherlocks?page=1".into() },
        Probe { label: "v4 /tracks", base: BASE_V4, path: "/tracks".into() },
        Probe { label: "v4 /season/list", base: BASE_V4, path: "/season/list".into() },
        Probe { label: "v4 /season/user/rank", base: BASE_V4, path: format!("/season/user/rank/{season}") },
    ];

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!(
                "  {:<8}{:<30}{:<10}{}",
                strings.diag_status, strings.diag_endpoint, "HTTP", strings.diag_latency
            ),
            palette.title(),
        )),
        Line::from(""),
    ];

    let mut healthy = 0usize;
    for probe in &probes {
        let (code, ms) = ctx.client.status_of(probe.base, &probe.path);
        if (200..300).contains(&code) {
            healthy += 1;
        }
        lines.push(probe_line(probe.label, code, ms, &palette));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("  {healthy}/{} {}", probes.len(), strings.diag_summary),
        if healthy == probes.len() {
            palette.ok()
        } else {
            palette.warn()
        },
    )));
    lines.push(Line::from(""));
    let vault = ctx.cfg.vault_kdf();
    lines.push(screens::field_styled(
        strings.diag_token,
        if vault.is_some() {
            strings.diag_configured
        } else {
            strings.diag_missing
        },
        &palette,
        if vault.is_some() {
            palette.ok()
        } else {
            palette.err()
        },
    ));
    lines.push(screens::field(
        strings.diag_vault,
        &vault.map(|k| k.label()).unwrap_or_else(|| "-".to_string()),
        &palette,
    ));
    lines.push(screens::field(strings.diag_theme, ctx.theme.label(), &palette));
    lines.push(screens::field(strings.diag_language, ctx.lang.label(), &palette));
    lines.push(screens::field(
        strings.diag_config_dir,
        &ctx.cfg.dir.display().to_string(),
        &palette,
    ));
    lines.push(screens::field(
        strings.diag_downloads,
        &ctx.cfg.download_dir.display().to_string(),
        &palette,
    ));

    screens::page(terminal, ctx, strings.diag_title, lines, &screens::nav_text(ctx))
}

fn probe_line(label: &str, code: u16, ms: u128, palette: &Palette) -> Line<'static> {
    let (mark, style) = match code {
        200..=299 => ("OK", palette.ok()),
        401 | 403 => ("AUTH", palette.warn()),
        _ => ("FAIL", palette.err()),
    };
    Line::from(vec![
        Span::styled(format!("  {mark:<8}"), style),
        Span::styled(format!("{label:<30}"), palette.value()),
        Span::styled(format!("{code:<10}"), palette.label()),
        Span::styled(format!("{ms} ms"), palette.dim()),
    ])
}
