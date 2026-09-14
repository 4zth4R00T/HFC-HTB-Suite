use crate::tui::event::{self, Input};
use crate::tui::screens;
use crate::tui::widgets::input::{ask, confirm, Prompt};
use crate::tui::widgets::picker::{Item, Outcome, Picker};
use crate::tui::Ctx;
use crate::vault::MIN_PASSPHRASE;
use ratatui::backend::Backend;
use ratatui::text::{Line, Span};
use ratatui::Terminal;
use std::io;
use zeroize::Zeroizing;

fn entries(ctx: &Ctx) -> Vec<Item> {
    let s = ctx.strings();
    let p = ctx.palette();
    vec![
        Item::with_detail(
            Line::from(vec![
                Span::styled(format!("{}: ", s.settings_theme), p.value()),
                Span::styled(ctx.theme.label().to_string(), p.accent_style()),
            ]),
            Line::from(""),
        ),
        Item::with_detail(
            Line::from(vec![
                Span::styled(format!("{}: ", s.settings_language), p.value()),
                Span::styled(ctx.lang.label().to_string(), p.accent_style()),
            ]),
            Line::from(""),
        ),
        Item::with_detail(
            Line::from(Span::styled(s.settings_token.to_string(), p.value())),
            Line::from(Span::styled(format!("    {}", s.settings_token_desc), p.dim())),
        ),
        Item::with_detail(
            Line::from(Span::styled(s.settings_passphrase.to_string(), p.value())),
            Line::from(Span::styled(
                format!("    {}", s.settings_passphrase_desc),
                p.dim(),
            )),
        ),
        Item::with_detail(
            Line::from(Span::styled(s.settings_forget.to_string(), p.warn())),
            Line::from(Span::styled(format!("    {}", s.settings_forget_desc), p.dim())),
        ),
        Item::with_detail(
            Line::from(Span::styled(s.settings_paths.to_string(), p.value())),
            Line::from(Span::styled(
                format!("    {}", ctx.cfg.dir.display()),
                p.dim(),
            )),
        ),
    ]
}

fn ask_new_passphrase<B: Backend>(
    terminal: &mut Terminal<B>,
    ctx: &mut Ctx,
) -> io::Result<Option<Zeroizing<String>>> {
    let palette = ctx.palette();
    let strings = ctx.strings();
    for _ in 0..3 {
        let Some(first) = ask(
            terminal,
            &palette,
            Prompt::new(strings.prompt_passphrase, strings.prompt_passphrase_label)
                .hint(strings.prompt_passphrase_hint)
                .masked(),
        )?
        else {
            return Ok(None);
        };
        let first = Zeroizing::new(first);
        if first.chars().count() < MIN_PASSPHRASE {
            screens::notice(
                terminal,
                ctx,
                strings.settings_passphrase,
                strings.passphrase_short,
                false,
            )?;
            continue;
        }
        let Some(again) = ask(
            terminal,
            &palette,
            Prompt::new(
                strings.prompt_passphrase_repeat,
                strings.prompt_passphrase_label,
            )
            .masked(),
        )?
        else {
            return Ok(None);
        };
        if *first != again {
            screens::notice(
                terminal,
                ctx,
                strings.settings_passphrase,
                strings.passphrase_mismatch,
                false,
            )?;
            continue;
        }
        return Ok(Some(first));
    }
    Ok(None)
}

pub fn run<B: Backend>(terminal: &mut Terminal<B>, ctx: &mut Ctx) -> io::Result<()> {
    let mut picker = Picker::new(entries(ctx));
    loop {
        let palette = ctx.palette();
        let strings = ctx.strings();
        let ctx_ref: &Ctx = ctx;
        terminal.draw(|f| {
            let area = f.area();
            screens::paint_background(f, &palette);
            let (title_a, body_a, footer_a) = screens::frame_layout(area);
            screens::render_title(f, title_a, strings.settings_title, &palette);
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
                picker.replace(entries(ctx));
                continue;
            }
            _ => {}
        }
        match picker.handle(&input) {
            Outcome::Leave => return Ok(()),
            Outcome::Chosen(i) => {
                match i {
                    0 => ctx.flip_theme(),
                    1 => {
                        if let Some(lang) = screens::language::run(terminal, ctx)? {
                            ctx.lang = lang;
                            let _ = ctx.cfg.write_lang(lang.code());
                        }
                    }
                    2 => {
                        if let Some(token) = ask(
                            terminal,
                            &palette,
                            Prompt::new(strings.prompt_token, strings.prompt_token_label)
                                .hint(strings.prompt_token_hint)
                                .paste_hint(strings.paste_hint)
                                .masked(),
                        )? {
                            let token = Zeroizing::new(token.trim().to_string());
                            if !token.is_empty() {
                                match ask_new_passphrase(terminal, ctx)? {
                                    Some(pass) => match ctx.cfg.write_vault(&token, &pass) {
                                        Ok(()) => {
                                            ctx.cfg.purge_legacy_token();
                                            ctx.client.set_token(&token);
                                            let probe = ctx.client.v4_get("/user/info");
                                            let (msg, ok) = if probe.ok {
                                                (strings.token_updated, true)
                                            } else {
                                                (strings.token_invalid, false)
                                            };
                                            screens::notice(
                                                terminal,
                                                ctx,
                                                strings.settings_token,
                                                msg,
                                                ok,
                                            )?;
                                        }
                                        Err(_) => {
                                            screens::notice(
                                                terminal,
                                                ctx,
                                                strings.settings_token,
                                                strings.vault_write_failed,
                                                false,
                                            )?;
                                        }
                                    },
                                    None => {
                                        screens::notice(
                                            terminal,
                                            ctx,
                                            strings.settings_token,
                                            strings.vault_cancelled,
                                            false,
                                        )?;
                                    }
                                }
                            }
                        }
                    }
                    3 => {
                        let token = Zeroizing::new(ctx.client.token_copy());
                        match ask_new_passphrase(terminal, ctx)? {
                            Some(pass) => {
                                let kdf = ctx
                                    .cfg
                                    .vault_kdf()
                                    .unwrap_or_else(crate::vault::Kdf::argon2id);
                                let (msg, ok) = match ctx.cfg.write_vault_with(&token, &pass, kdf) {
                                    Ok(()) => (strings.passphrase_updated, true),
                                    Err(_) => (strings.vault_write_failed, false),
                                };
                                screens::notice(
                                    terminal,
                                    ctx,
                                    strings.settings_passphrase,
                                    msg,
                                    ok,
                                )?;
                            }
                            None => {
                                screens::notice(
                                    terminal,
                                    ctx,
                                    strings.settings_passphrase,
                                    strings.vault_cancelled,
                                    false,
                                )?;
                            }
                        }
                    }
                    4 => {
                        if confirm(
                            terminal,
                            &palette,
                            strings.confirm_title,
                            strings.settings_forget_desc,
                            strings.confirm_yes_hint,
                        )? {
                            ctx.cfg.forget_token();
                            screens::notice(
                                terminal,
                                ctx,
                                strings.settings_forget,
                                strings.token_removed,
                                true,
                            )?;
                        }
                    }
                    5 => {
                        let lines = vec![
                            Line::from(""),
                            screens::field(
                                strings.diag_config_dir,
                                &ctx.cfg.dir.display().to_string(),
                                &palette,
                            ),
                            screens::field(
                                strings.diag_downloads,
                                &ctx.cfg.download_dir.display().to_string(),
                                &palette,
                            ),
                            screens::field("VPN", &ctx.cfg.vpn_dir.display().to_string(), &palette),
                            screens::field(
                                strings.diag_token,
                                &ctx.cfg.vault_file.display().to_string(),
                                &palette,
                            ),
                            screens::field(
                                strings.diag_vault,
                                &ctx.cfg
                                    .vault_kdf()
                                    .map(|k| k.label())
                                    .unwrap_or_else(|| "-".to_string()),
                                &palette,
                            ),
                        ];
                        screens::page(
                            terminal,
                            ctx,
                            strings.settings_paths,
                            lines,
                            strings.nav_hint,
                        )?;
                    }
                    _ => {}
                }
                picker.replace(entries(ctx));
            }
            _ => {}
        }
    }
}
