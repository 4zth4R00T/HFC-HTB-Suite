mod brand;
mod clipboard;
mod commands;
mod config;
mod dates;
mod hardening;
mod http;
mod session;
mod tui;
mod ui;
mod vault;
#[cfg(windows)]
mod win;

#[cfg(debug_assertions)]
mod selftest;

use clap::{Parser, Subcommand};
use commands::{catalog, machines, profile, vpn};
use config::Config;
use http::Client;

#[derive(Parser)]
#[command(
    name = "htb",
    version,
    about = "Cliente de terminal para Hack The Box Labs — maquinas, retos, VPN y perfil"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[cfg(debug_assertions)]
    #[arg(long, hide = true)]
    self_test: bool,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Maquinas: listar, info, lanzar, parar, reiniciar, extender y enviar flag")]
    Machines {
        #[command(subcommand)]
        action: MachineAction,
    },
    #[command(about = "Retos: listar, descargar archivos y enviar flag")]
    Challenges {
        #[command(subcommand)]
        action: ChallengeAction,
    },
    #[command(about = "Pro Labs: listar o ver el detalle de uno")]
    Prolabs {
        #[arg(long)]
        id: Option<String>,
    },
    #[command(about = "Fortress: listar, ver el detalle o enviar flag")]
    Fortress {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        flag: Option<String>,
    },
    #[command(about = "Sherlocks: listar (--search, --state, --favorites, --difficulty, --category, --sort, --completed) o descargar el material")]
    Sherlocks {
        #[arg(long, default_value_t = 1)]
        page: u32,
        #[arg(long)]
        download: Option<String>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        state: Option<String>,
        #[arg(long)]
        favorites: bool,
        #[arg(long)]
        difficulty: Option<String>,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        sort: Option<String>,
        #[arg(long)]
        asc: bool,
        #[arg(long)]
        completed: bool,
    },
    #[command(about = "Tracks: listar o ver el detalle de uno")]
    Tracks {
        #[arg(long)]
        id: Option<String>,
    },
    #[command(about = "Seasons: temporada activa, tier y progreso; --leaderboard [--page N], --rewards")]
    Seasons {
        #[arg(long)]
        leaderboard: bool,
        #[arg(long)]
        rewards: bool,
        #[arg(long, default_value_t = 1)]
        page: u32,
    },
    #[command(about = "Rankings: users, teams, universities o countries")]
    Rankings {
        #[arg(default_value = "users")]
        kind: String,
    },
    #[command(about = "Pwnbox de Labs: estado y minutos usados")]
    Pwnbox,
    #[command(about = "VPN: listar servidores, cambiar el asignado y bajar el .ovpn")]
    Vpn {
        #[command(subcommand)]
        action: VpnAction,
    },
    #[command(about = "Perfil de HTB de la cuenta del token")]
    Profile,
    #[command(about = "Quien hizo esto y a quien se le agradece")]
    Credits,
    #[command(about = "Guardar, cambiar o borrar el token cifrado de la API")]
    Token {
        #[arg(long)]
        set: bool,
        #[arg(long)]
        passphrase: bool,
        #[arg(long)]
        forget: bool,
        #[arg(long)]
        portable: bool,
    },
}

#[derive(Subcommand)]
enum MachineAction {
    #[command(about = "Listar: --state active|retired|unreleased|all, --favorites, --sort rating|release_date|user_difficulty|user_owns|system_owns|name, --asc, --completed, --incomplete")]
    List {
        #[arg(long, default_value = "active")]
        state: String,
        #[arg(long)]
        difficulty: Option<String>,
        #[arg(long)]
        os: Option<String>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        favorites: bool,
        #[arg(long)]
        sort: Option<String>,
        #[arg(long)]
        asc: bool,
        #[arg(long)]
        completed: bool,
        #[arg(long)]
        incomplete: bool,
        #[arg(long, default_value_t = 1)]
        page: u32,
    },
    Info {
        id: String,
    },
    #[command(about = "Marcar o quitar favorito (★)")]
    Favorite {
        id: String,
    },
    #[command(about = "Opiniones de la maquina")]
    Reviews {
        id: String,
    },
    #[command(about = "Actividad reciente: quien la resolvio y cuando")]
    Activity {
        id: String,
    },
    #[command(about = "Registro de cambios")]
    Changelog {
        id: String,
    },
    #[command(about = "Writeup oficial, video y guias de la comunidad")]
    Walkthroughs {
        id: String,
    },
    #[command(about = "Ficha completa: sinopsis, etiquetas, matriz y bloods")]
    Tags {
        id: String,
    },
    #[command(about = "Busqueda global: maquinas, retos, usuarios y equipos")]
    Search {
        term: String,
    },
    #[command(about = "En progreso, para ti o favoritos, como la home de la web")]
    Dashboard {
        #[arg(default_value = "inprogress")]
        which: String,
    },
    Active,
    Spawn {
        id: String,
    },
    Stop,
    Reset,
    Extend,
    Flag {
        id: String,
        flag: String,
        #[arg(long, default_value_t = 5)]
        difficulty: u8,
    },
}

#[derive(Subcommand)]
enum ChallengeAction {
    #[command(about = "Listar: --state, --favorites, --search, --difficulty, --category, --sort, --completed, --incomplete, --page")]
    List {
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        unsolved: bool,
        #[arg(long)]
        state: Option<String>,
        #[arg(long)]
        favorites: bool,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        difficulty: Option<String>,
        #[arg(long)]
        sort: Option<String>,
        #[arg(long)]
        asc: bool,
        #[arg(long)]
        completed: bool,
        #[arg(long, default_value_t = 1)]
        page: u32,
    },
    #[command(about = "Marcar o quitar favorito")]
    Favorite {
        id: String,
    },
    #[command(about = "Opiniones del reto")]
    Reviews {
        id: String,
    },
    #[command(about = "Actividad reciente del reto")]
    Activity {
        id: String,
    },
    Download {
        id: String,
    },
    Flag {
        id: String,
        flag: String,
        #[arg(long, default_value_t = 5)]
        difficulty: u8,
    },
}

#[derive(Subcommand)]
enum VpnAction {
    List {
        #[arg(default_value = "labs")]
        product: String,
    },
    Switch {
        server_id: String,
    },
    Download {
        server_id: String,
        #[arg(long, default_value = "labs")]
        product: String,
        #[arg(long)]
        tcp: bool,
    },
}

fn manage_token(cfg: &Config, set: bool, passphrase: bool, forget: bool, portable: bool) {
    let kdf = if portable {
        vault::Kdf::pbkdf2()
    } else {
        vault::Kdf::argon2id()
    };
    if forget {
        cfg.forget_token();
        ui::ok("Token borrado del disco.");
        return;
    }
    if set {
        let Some(token) = ui::prompt_secret("App Token (no se ve al pegarlo)") else {
            ui::fail("Cancelado.");
            return;
        };
        let token = token.trim().to_string();
        if token.is_empty() {
            ui::fail("Token vacio: no cambio nada.");
            return;
        }
        let Some(pass) = session::pedir_passphrase_nueva() else {
            ui::fail("Cancelado: el token no se ha guardado.");
            return;
        };
        match cfg.write_vault_with(&token, &pass, kdf) {
            Ok(()) => {
                cfg.purge_legacy_token();
                ui::ok("Token cifrado y guardado.");
                ui::field("Cifrado", &kdf.label());
            }
            Err(e) => ui::fail(&format!("No pude guardar el token: {e}")),
        }
        return;
    }
    if passphrase {
        if !cfg.has_vault() {
            ui::note("No hay vault todavia. Guarda un token primero: htb token --set");
            return;
        }
        let token = session::unlock(cfg);
        ui::note("Ahora la passphrase nueva.");
        let Some(pass) = session::pedir_passphrase_nueva() else {
            ui::fail("Cancelado: la passphrase sigue siendo la de antes.");
            return;
        };
        let kdf = if portable {
            kdf
        } else {
            cfg.vault_kdf().unwrap_or(kdf)
        };
        match cfg.write_vault_with(&token, &pass, kdf) {
            Ok(()) => ui::ok("Passphrase cambiada."),
            Err(e) => ui::fail(&format!("No pude reescribir el vault: {e}")),
        }
        return;
    }
    if cfg.has_vault() {
        ui::ok(&format!("Hay un token cifrado en {}", cfg.vault_file.display()));
        if let Some(kdf) = cfg.vault_kdf() {
            ui::field("Cifrado", &kdf.label());
        }
    } else if cfg.has_legacy_token() {
        ui::note("Hay un token en claro de una version anterior; se cifrara al arrancar.");
    } else {
        ui::note("No hay ningun token guardado todavia.");
    }
}

fn main() {
    hardening::apply();
    let cli = Cli::parse();

    #[cfg(debug_assertions)]
    if cli.self_test {
        std::process::exit(if selftest::run() { 0 } else { 1 });
    }

    let cfg = Config::load();
    if let Some(Command::Credits) = cli.command {
        profile::credits();
        return;
    }
    if let Some(Command::Token {
        set,
        passphrase,
        forget,
        portable,
    }) = &cli.command
    {
        manage_token(&cfg, *set, *passphrase, *forget, *portable);
        return;
    }

    let unlocked = session::unlock(&cfg);
    let client = Client::new(&unlocked);
    drop(unlocked);

    let Some(command) = cli.command else {
        let probe = client.v4_get("/user/info");
        if !probe.ok {
            ui::fail(&format!("El token no autentica (HTTP {}).", probe.status));
            ui::note(&format!(
                "Genera otro en {} y guardalo con: htb token --set <token>",
                http::TOKEN_URL
            ));
        }
        if let Err(e) = tui::run(&client, &cfg) {
            eprintln!("La interfaz no pudo arrancar: {e}");
            std::process::exit(1);
        }
        return;
    };

    match command {
        Command::Machines { action } => match action {
            MachineAction::List {
                state,
                difficulty,
                os,
                search,
                favorites,
                sort,
                asc,
                completed,
                incomplete,
                page,
            } => machines::list(
                &client,
                &machines::ListOptions {
                    state,
                    difficulty,
                    os,
                    search,
                    favorites,
                    sort,
                    asc,
                    completed: if completed {
                        Some(true)
                    } else if incomplete {
                        Some(false)
                    } else {
                        None
                    },
                    page,
                },
            ),
            MachineAction::Info { id } => machines::info(&client, &id),
            MachineAction::Favorite { id } => machines::favorite(&client, "machine", &id),
            MachineAction::Reviews { id } => machines::reviews(&client, "machine", &id),
            MachineAction::Activity { id } => machines::activity(&client, "machine", &id),
            MachineAction::Changelog { id } => machines::changelog(&client, "machine", &id),
            MachineAction::Walkthroughs { id } => machines::walkthroughs(&client, &id),
            MachineAction::Tags { id } => machines::tags(&client, &id),
            MachineAction::Search { term } => machines::search(&client, &term),
            MachineAction::Dashboard { which } => machines::dashboard(&client, &which),
            MachineAction::Active => machines::active(&client),
            MachineAction::Spawn { id } => machines::spawn(&client, &id),
            MachineAction::Stop => machines::stop(&client),
            MachineAction::Reset => machines::reset(&client),
            MachineAction::Extend => machines::extend(&client),
            MachineAction::Flag {
                id,
                flag,
                difficulty,
            } => machines::flag(&client, &id, &flag, difficulty),
        },
        Command::Challenges { action } => match action {
            ChallengeAction::List {
                category,
                unsolved,
                state,
                favorites,
                search,
                difficulty,
                sort,
                asc,
                completed,
                page,
            } => catalog::challenges(
                &client,
                &catalog::ContentOptions {
                    category,
                    unsolved,
                    state,
                    favorites,
                    search,
                    difficulty,
                    sort,
                    asc,
                    completed,
                    page,
                },
            ),
            ChallengeAction::Favorite { id } => machines::favorite(&client, "challenge", &id),
            ChallengeAction::Reviews { id } => machines::reviews(&client, "challenge", &id),
            ChallengeAction::Activity { id } => machines::activity(&client, "challenge", &id),
            ChallengeAction::Download { id } => catalog::challenge_download(&client, &cfg, &id),
            ChallengeAction::Flag {
                id,
                flag,
                difficulty,
            } => catalog::challenge_flag(&client, &id, &flag, difficulty),
        },
        Command::Prolabs { id } => catalog::prolabs(&client, id.as_deref()),
        Command::Fortress { id, flag } => match (id, flag) {
            (Some(id), Some(flag)) => catalog::fortress_flag(&client, &id, &flag),
            (id, _) => catalog::fortress(&client, id.as_deref()),
        },
        Command::Sherlocks {
            page,
            download,
            search,
            state,
            favorites,
            difficulty,
            category,
            sort,
            asc,
            completed,
        } => match download {
            Some(id) => catalog::sherlock_download(&client, &cfg, &id),
            None => catalog::sherlocks(
                &client,
                &catalog::ContentOptions {
                    category,
                    unsolved: false,
                    state,
                    favorites,
                    search,
                    difficulty,
                    sort,
                    asc,
                    completed,
                    page,
                },
            ),
        },
        Command::Tracks { id } => catalog::tracks(&client, id.as_deref()),
        Command::Seasons { leaderboard, rewards, page } => catalog::seasons(&client, leaderboard, rewards, page),
        Command::Rankings { kind } => catalog::rankings(&client, &kind),
        Command::Pwnbox => catalog::pwnbox(&client),
        Command::Vpn { action } => match action {
            VpnAction::List { product } => vpn::list(&client, &product),
            VpnAction::Switch { server_id } => vpn::switch(&client, &server_id),
            VpnAction::Download {
                server_id,
                product,
                tcp,
            } => vpn::download(&client, &cfg, &product, &server_id, tcp),
        },
        Command::Profile => profile::show(&client),
        Command::Credits | Command::Token { .. } => {}
    }
}
