
use crate::config::Config;
use crate::http::TOKEN_URL;
use crate::ui;
use crate::vault::{Secret, VaultError, MIN_PASSPHRASE};

const INTENTOS: u8 = 3;

pub fn unlock(cfg: &Config) -> Secret {
    if let Ok(t) = std::env::var("HTB_TOKEN") {
        let t = t.trim().to_string();
        if !t.is_empty() {
            ui::note("Usando HTB_TOKEN del entorno: el token queda visible para");
            ui::note("cualquier proceso tuyo que lea /proc/<pid>/environ. Para uso");
            ui::note("interactivo es mejor el vault; esta variable es para scripts.");
            return Secret::new(t);
        }
    }

    if cfg.has_vault() {
        return abrir_vault(cfg);
    }

    if cfg.has_legacy_token() || cfg.read_legacy_token().is_some() {
        return migrar(cfg);
    }

    alta(cfg)
}

fn abrir_vault(cfg: &Config) -> Secret {
    if let Some(kdf) = cfg.vault_kdf() {
        ui::field("Vault", &kdf.label());
    }
    for intento in 1..=INTENTOS {
        let Some(pass) = ui::prompt_secret("Passphrase del vault") else {
            ui::fail("Sin passphrase no puedo abrir el vault.");
            std::process::exit(1);
        };
        match cfg.open_vault(&pass) {
            Ok(token) => return token,
            Err(VaultError::WrongPassphrase) => {
                if intento < INTENTOS {
                    ui::fail(&format!(
                        "Passphrase incorrecta ({} de {}).",
                        intento, INTENTOS
                    ));
                }
            }
            Err(e) => {
                ui::fail(&format!("No pude abrir el vault: {e}"));
                ui::note(&format!(
                    "Si perdiste la passphrase: htb token --forget y vuelve a guardar el token desde {TOKEN_URL}"
                ));
                std::process::exit(1);
            }
        }
    }
    ui::fail("Passphrase incorrecta tres veces.");
    std::process::exit(1);
}

fn migrar(cfg: &Config) -> Secret {
    let Some(token) = cfg.read_legacy_token() else {
        return alta(cfg);
    };
    ui::banner();
    println!();
    ui::note("Encontre un token guardado en claro por una version anterior.");
    ui::note("Voy a cifrarlo y a borrar el fichero viejo sobrescribiendolo.");
    println!();
    let Some(pass) = pedir_passphrase_nueva() else {
        ui::fail("Migracion cancelada: el token sigue en claro en el disco.");
        std::process::exit(1);
    };
    match cfg.write_vault(&token, &pass) {
        Ok(()) => {
            cfg.purge_legacy_token();
            ui::ok("Token cifrado. El fichero en claro ya no existe.");
            token
        }
        Err(e) => {
            ui::fail(&format!("No pude cifrar el token: {e}"));
            std::process::exit(1);
        }
    }
}

fn alta(cfg: &Config) -> Secret {
    ui::banner();
    println!();
    ui::note("Necesito tu App Token de Hack The Box para empezar.");
    ui::note(&format!("Lo generas en {TOKEN_URL} → Create App Token"));
    ui::note(&format!("Lo guardo cifrado en {}", cfg.vault_file.display()));
    println!();
    let Some(token) = ui::prompt_secret("App Token (no se ve al pegarlo)") else {
        ui::fail("Sin token no puedo hablar con la API.");
        std::process::exit(1);
    };
    let token = Secret::new(token.trim().to_string());
    if token.is_empty() {
        ui::fail("Sin token no puedo hablar con la API.");
        std::process::exit(1);
    }
    println!();
    ui::note("Elige una passphrase para cifrarlo. Se pedira cada vez que abras");
    ui::note("la suite y no hay forma de recuperarla si la olvidas.");
    let Some(pass) = pedir_passphrase_nueva() else {
        ui::fail("Sin passphrase no guardo el token.");
        std::process::exit(1);
    };
    match cfg.write_vault(&token, &pass) {
        Ok(()) => ui::ok("Token cifrado y guardado."),
        Err(e) => {
            ui::fail(&format!("No pude guardar el token: {e}"));
            std::process::exit(1);
        }
    }
    token
}

pub fn pedir_passphrase_nueva() -> Option<Secret> {
    for _ in 0..INTENTOS {
        let pass = ui::prompt_secret("Passphrase nueva")?;
        if pass.chars().count() < MIN_PASSPHRASE {
            ui::fail(&format!(
                "Demasiado corta: minimo {MIN_PASSPHRASE} caracteres."
            ));
            continue;
        }
        let repeat = ui::prompt_secret("Repite la passphrase")?;
        if *pass != *repeat {
            ui::fail("No coinciden.");
            continue;
        }
        return Some(pass);
    }
    None
}
