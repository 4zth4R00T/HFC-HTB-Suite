
use crate::ui;

pub fn apply() {
    silence_panics();
    refuse_root();
    seal_process();
    sanitize_path();
}

fn silence_panics() {
    std::env::remove_var("RUST_BACKTRACE");
    std::env::remove_var("RUST_LIB_BACKTRACE");
    std::panic::set_hook(Box::new(|_| {
        eprintln!("htb: error interno. La sesion se cierra sin guardar nada.");
    }));
}

fn refuse_root() {
    #[cfg(unix)]
    {
        if unsafe { libc::geteuid() } != 0 {
            return;
        }
        if std::env::var("HTB_ALLOW_ROOT").is_ok_and(|v| v == "1") {
            ui::note("Ejecutando como root por HTB_ALLOW_ROOT=1. Los ficheros que");
            ui::note("cree seran de root: revisa quien puede escribirlos despues.");
            return;
        }
        ui::fail("No me ejecuto como root.");
        ui::note("La suite solo necesita tu usuario. Como root dejaria ficheros de");
        ui::note("root en tu HOME y la VPN tampoco lo necesita para descargarse.");
        ui::note("Si de verdad lo quieres: HTB_ALLOW_ROOT=1 htb");
        std::process::exit(1);
    }
}

fn seal_process() {
    #[cfg(unix)]
    unsafe {
        let sin_core = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        libc::setrlimit(libc::RLIMIT_CORE, &sin_core);
        libc::umask(0o077);
    }
    #[cfg(target_os = "linux")]
    unsafe {
        libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
    }
}

fn sanitize_path() {
    let Ok(path) = std::env::var("PATH") else {
        return;
    };
    let clean: Vec<String> = path
        .split(':')
        .filter(|entry| !entry.is_empty() && entry.starts_with('/') && !writable_by_others(entry))
        .map(|s| s.to_string())
        .collect();
    if clean.join(":") != path {
        std::env::set_var("PATH", clean.join(":"));
    }
}

#[cfg(unix)]
fn writable_by_others(dir: &str) -> bool {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let Ok(meta) = std::fs::metadata(dir) else {
        return true;
    };
    let mode = meta.permissions().mode();
    let ajeno = meta.uid() != 0 && meta.uid() != unsafe { libc::getuid() };
    mode & 0o022 != 0 || ajeno
}

#[cfg(not(unix))]
fn writable_by_others(_dir: &str) -> bool {
    false
}
