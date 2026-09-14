use crate::brand;
use crate::tui::event::Input;
use crate::tui::i18n::{strings, Lang};
use crate::tui::screens;
use crate::tui::theme::{palette, ThemeMode};
use crate::tui::widgets::picker::{Item, Outcome, Picker};
use ratatui::backend::TestBackend;
use ratatui::text::Line;
use ratatui::Terminal;

fn has_content(term: &Terminal<TestBackend>) -> bool {
    term.backend()
        .buffer()
        .content
        .iter()
        .any(|cell| cell.symbol() != " ")
}

pub fn run() -> bool {
    let mut all_ok = true;
    let mut check = |name: &str, ok: bool| {
        println!("  [{}] {name}", if ok { "OK  " } else { "FAIL" });
        all_ok = all_ok && ok;
    };

    for lang in Lang::all() {
        let s = strings(lang);
        check(
            &format!("i18n {} — frases de bienvenida", lang.code()),
            !s.welcome.is_empty() && s.welcome.iter().all(|w| !w.is_empty()),
        );
        check(
            &format!("i18n {} — claves principales", lang.code()),
            !s.app_title.is_empty()
                && !s.nav_hint.is_empty()
                && !s.machines_title.is_empty()
                && !s.catalog_title.is_empty()
                && !s.vpn_title.is_empty()
                && !s.profile_title.is_empty()
                && !s.credits_title.is_empty(),
        );
    }

    let dark = palette(ThemeMode::Dark);
    let light = palette(ThemeMode::Light);
    check(
        "tema — oscuro y claro son distintos",
        dark.bg != light.bg && dark.text != light.text,
    );

    check(
        "logo — HFC tiene ancho uniforme",
        brand::HFC.windows(2).all(|w| w[0].chars().count() == w[1].chars().count()),
    );
    check(
        "logo — el wordmark tiene ancho uniforme",
        brand::WORDMARK
            .windows(2)
            .all(|w| w[0].chars().count() == w[1].chars().count()),
    );

    let mut picker = Picker::new(vec![
        Item::plain("uno"),
        Item::plain("dos"),
        Item::plain("tres"),
    ]);
    picker.handle(&Input::Down);
    let moved = picker.selected() == 1;
    picker.handle(&Input::Up);
    picker.handle(&Input::Up);
    let wrapped = picker.selected() == 2;
    picker.handle(&Input::Home);
    let home = picker.selected() == 0;
    picker.handle(&Input::End);
    let end = picker.selected() == 2;
    let chosen = matches!(picker.handle(&Input::Select), Outcome::Chosen(2));
    let leaves = matches!(picker.handle(&Input::Back), Outcome::Leave);
    check("lista — baja una posicion", moved);
    check("lista — da la vuelta al llegar arriba", wrapped);
    check("lista — Inicio salta a la primera", home);
    check("lista — Fin salta a la ultima", end);
    check("lista — Enter devuelve la seleccion", chosen);
    check("lista — Esc sale de la pantalla", leaves);

    let mut empty = Picker::new(Vec::new());
    empty.handle(&Input::Down);
    check(
        "lista — una lista vacia no revienta",
        matches!(empty.handle(&Input::Select), Outcome::Ignored),
    );

    for lang in Lang::all() {
        let s = strings(lang);
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let pal = palette(mode);
            let Ok(mut term) = Terminal::new(TestBackend::new(110, 44)) else {
                check(&format!("render {}/{}", lang.code(), mode.label()), false);
                continue;
            };
            let mut menu = Picker::new(vec![
                Item::plain(s.menu_machines),
                Item::plain(s.menu_catalog),
                Item::plain(s.menu_vpn),
                Item::plain(s.menu_profile),
                Item::plain(s.menu_diagnostics),
                Item::plain(s.menu_settings),
                Item::plain(s.menu_credits),
            ]);
            let drawn = term.draw(|f| {
                let area = f.area();
                screens::paint_background(f, &pal);
                let (title_a, body_a, footer_a) = screens::frame_layout(area);
                screens::render_title(f, title_a, s.app_title, &pal);
                menu.render(f, body_a, s.tagline, &pal);
                screens::render_footer(f, footer_a, s.nav_hint, &pal);
            });
            check(
                &format!("render menu {}/{}", lang.code(), mode.label()),
                drawn.is_ok() && has_content(&term),
            );
        }
    }

    {
        let s = strings(Lang::Es);
        let pal = palette(ThemeMode::Dark);
        if let Ok(mut term) = Terminal::new(TestBackend::new(110, 44)) {
            let lines: Vec<Line> = vec![
                screens::section(s.profile_section_account, &pal),
                screens::field(s.profile_rank, "Pro Hacker", &pal),
                screens::bullet("una linea cualquiera", &pal),
            ];
            let drawn = term.draw(|f| {
                let area = f.area();
                screens::paint_background(f, &pal);
                let (title_a, body_a, footer_a) = screens::frame_layout(area);
                screens::render_title(f, title_a, s.profile_title, &pal);
                f.render_widget(ratatui::widgets::Paragraph::new(lines.clone()), body_a);
                screens::render_footer(f, footer_a, s.press_key, &pal);
            });
            check(
                "render pantalla de contenido",
                drawn.is_ok() && has_content(&term),
            );
        }
    }

    {
        let pal = palette(ThemeMode::Dark);
        if let Ok(mut term) = Terminal::new(TestBackend::new(80, 24)) {
            let art = crate::tui::screens::boot::block_lines(&brand::WORDMARK, &pal);
            let drawn = term.draw(|f| {
                f.render_widget(
                    ratatui::widgets::Paragraph::new(art.clone())
                        .alignment(ratatui::layout::Alignment::Center),
                    f.area(),
                );
            });
            check("render del wordmark", drawn.is_ok() && has_content(&term));
        }
    }

    all_ok
}
