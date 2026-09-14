use super::{text, truncate};
use crate::config::Config;
use crate::http::{sanitize, Client, BASE_V4};
use crate::ui;
use serde_json::json;
use std::collections::HashMap;

pub fn challenges(client: &Client, category: Option<&str>, unsolved: bool) {
    let cats: HashMap<String, String> = client
        .v4_get("/challenge/categories/list")
        .array_at("/info")
        .iter()
        .map(|c| (text(c, "id"), text(c, "name")))
        .collect();

    let rows = client
        .v4_get("/challenge/list")
        .first_array(&["/challenges", "/data"]);
    if rows.is_empty() {
        ui::note("La API no devolvio retos.");
        return;
    }
    ui::heading("Challenges");
    for c in &rows {
        let name = cats
            .get(&text(c, "challenge_category_id"))
            .cloned()
            .unwrap_or_else(|| text(c, "category_name"));
        if let Some(filter) = category {
            if !name.eq_ignore_ascii_case(filter) {
                continue;
            }
        }
        let solved = c
            .get("authUserSolve")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if unsolved && solved {
            continue;
        }
        println!(
            "  {:<3} {:<7} {:<32} {:<16} {:<9} {:>5}",
            if solved { "✓" } else { "" },
            text(c, "id"),
            truncate(&text(c, "name"), 32),
            truncate(&name, 16),
            text(c, "difficulty"),
            text(c, "points")
        );
    }
}

pub fn challenge_download(client: &Client, cfg: &Config, id: &str) {
    let raw = client.v4_get(&format!("/challenge/info/{id}"));
    let c = raw
        .data
        .pointer("/challenge")
        .or_else(|| raw.data.pointer("/data"))
        .cloned()
        .unwrap_or_else(|| raw.data.clone());
    let name = text(&c, "name");
    if !c.get("download").and_then(|v| v.as_bool()).unwrap_or(false) {
        ui::note("Ese reto no trae archivos.");
        return;
    }
    let out = cfg
        .download_dir
        .join(format!("challenge_{id}_{}.zip", sanitize(&name)));
    match client.download(&format!("{BASE_V4}/challenge/download/{id}"), &out) {
        Ok(size) if size > 100 => {
            ui::ok(&format!("Guardado en {} ({size} B)", out.display()));
            ui::note("El zip esta cifrado. Contrasena: hackthebox");
        }
        _ => {
            let _ = std::fs::remove_file(&out);
            ui::fail("La descarga no salio bien.");
        }
    }
}

pub fn challenge_flag(client: &Client, id: &str, flag: &str, difficulty: u8) {
    let res = client.v4_post(
        "/challenge/own",
        json!({
            "flag": flag,
            "challenge_id": id.parse::<i64>().unwrap_or(0),
            "difficulty": difficulty.clamp(1, 10) * 10
        }),
    );
    let msg = res.message();
    if crate::http::read_flag_answer(&msg) == crate::http::Verdict::Accepted {
        ui::ok(&msg);
    } else {
        ui::fail(&msg);
    }
}

pub fn prolabs(client: &Client, id: Option<&str>) {
    match id {
        None => {
            let labs = client.v4_get("/prolabs").first_array(&["/data/labs", "/data"]);
            ui::heading("Pro Labs");
            for l in &labs {
                println!(
                    "  {:<5} {:<26} {:<12} {:>3} maquinas  {}/{} flags",
                    text(l, "id"),
                    truncate(&text(l, "name"), 26),
                    text(l, "skill_level"),
                    text(l, "pro_machines_count"),
                    text(l, "pro_flags_owns_count"),
                    text(l, "pro_flags_count")
                );
            }
        }
        Some(id) => {
            let info = client.v4_get(&format!("/prolab/{id}/info"));
            let progress = client.v4_get(&format!("/prolab/{id}/progress"));
            ui::heading(&info.str_at("/data/name"));
            ui::field("Version", &info.str_at("/data/version"));
            ui::field("Progreso", &format!("{}%", progress.str_at("/data/ownership")));
            println!();
            ui::note("Maquinas");
            for m in client
                .v4_get(&format!("/prolab/{id}/machines"))
                .array_at("/data")
            {
                println!("    · {} ({})", text(&m, "name"), text(&m, "os"));
            }
            println!();
            ui::note("Flags");
            for f in client.v4_get(&format!("/prolab/{id}/flags")).array_at("/data") {
                let owned = f.get("owned").and_then(|v| v.as_bool()).unwrap_or(false);
                println!(
                    "    · {} — {} pts {}",
                    text(&f, "title"),
                    text(&f, "points"),
                    if owned { "✓" } else { "" }
                );
            }
        }
    }
}

pub fn fortress(client: &Client, id: Option<&str>) {
    match id {
        None => {
            ui::heading("Fortress");
            for f in client.v4_get("/fortresses").first_array(&["/data"]) {
                println!(
                    "  {:<5} {:<26} {} flags",
                    text(&f, "id"),
                    truncate(&text(&f, "name"), 26),
                    text(&f, "number_of_flags")
                );
            }
        }
        Some(id) => {
            let d = client.v4_get(&format!("/fortress/{id}"));
            ui::heading(&d.str_at("/data/name"));
            ui::field("IP", &d.str_at("/data/ip"));
            ui::field("Puntos", &d.str_at("/data/points"));
            ui::field("Progreso", &format!("{}%", d.str_at("/data/progress_percent")));
            println!();
            for f in d.array_at("/data/flags") {
                let owned = f.get("owned").and_then(|v| v.as_bool()).unwrap_or(false);
                println!(
                    "    · {} — {} pts {}",
                    text(&f, "title"),
                    text(&f, "points"),
                    if owned { "✓" } else { "" }
                );
            }
        }
    }
}

pub fn fortress_flag(client: &Client, id: &str, flag: &str) {
    let res = client.v4_post(&format!("/fortress/{id}/flag"), json!({ "flag": flag }));
    let msg = res.message();
    if crate::http::read_flag_answer(&msg) == crate::http::Verdict::Accepted {
        ui::ok(&msg);
    } else {
        ui::fail(&msg);
    }
}

pub fn sherlocks(client: &Client, page: u32) {
    let r = client.v4_get(&format!("/sherlocks?page={page}"));
    let rows = r.first_array(&["/data", "/sherlocks"]);
    if rows.is_empty() {
        ui::note("Sin resultados en esa pagina.");
        return;
    }
    let last = r
        .data
        .pointer("/meta/last_page")
        .and_then(|v| v.as_u64())
        .unwrap_or(1);
    ui::heading(&format!("Sherlocks — pagina {page}/{last}"));
    for s in &rows {
        let owned = s.get("is_owned").and_then(|v| v.as_bool()).unwrap_or(false);
        println!(
            "  {:<3} {:<7} {:<32} {:<16} {:<9}",
            if owned { "✓" } else { "" },
            text(s, "id"),
            truncate(&text(s, "name"), 32),
            truncate(&text(s, "category_name"), 16),
            text(s, "difficulty")
        );
    }
}

pub fn sherlock_download(client: &Client, cfg: &Config, id: &str) {
    let meta = client.v4_get(&format!("/sherlocks/{id}"));
    let name = meta.str_at("/data/name");
    let password = {
        let raw = meta.str_at("/data/file_password");
        if raw == "-" || raw.trim().is_empty() {
            "hacktheblue".to_string()
        } else {
            raw
        }
    };
    let link = client
        .v4_get(&format!("/sherlocks/{id}/download_link"))
        .str_at("/url");
    if link == "-" || link.is_empty() {
        ui::note("HTB no da enlace de descarga para ese sherlock.");
        return;
    }
    let out = cfg
        .download_dir
        .join(format!("sherlock_{id}_{}.zip", sanitize(&name)));
    match client.download(&link, &out) {
        Ok(size) if size > 100 => {
            ui::ok(&format!("Guardado en {} ({size} B)", out.display()));
            ui::note(&format!("El zip esta cifrado. Contrasena: {password}"));
        }
        _ => {
            let _ = std::fs::remove_file(&out);
            ui::fail("La descarga no salio bien.");
        }
    }
}

pub fn tracks(client: &Client, id: Option<&str>) {
    match id {
        None => {
            ui::heading("Tracks");
            for t in client.v4_get("/tracks").first_array(&["/data"]) {
                println!(
                    "  {:<5} {:<36} {:<10} {} likes",
                    text(&t, "id"),
                    truncate(&text(&t, "name"), 36),
                    text(&t, "difficulty"),
                    text(&t, "likes")
                );
            }
        }
        Some(id) => {
            let raw = client.v4_get(&format!("/tracks/{id}"));
            let t = raw
                .data
                .pointer("/data")
                .cloned()
                .unwrap_or_else(|| raw.data.clone());
            ui::heading(&text(&t, "name"));
            ui::field("Dificultad", &text(&t, "difficulty"));
            ui::field("Likes", &text(&t, "likes"));
            println!();
            if let Some(items) = t.get("items").and_then(|v| v.as_array()) {
                for it in items {
                    println!(
                        "    · [{}] {} ({})",
                        text(it, "type"),
                        text(it, "name"),
                        text(it, "difficulty")
                    );
                }
            }
        }
    }
}

pub fn seasons(client: &Client) {
    let list = client.v4_get("/season/list");
    ui::heading("Seasons");
    for s in list.array_at("/data") {
        let active = s.get("active").and_then(|v| v.as_bool()).unwrap_or(false);
        println!(
            "  {:<5} {:<30} {:<12} {}",
            text(&s, "id"),
            truncate(&text(&s, "name"), 30),
            text(&s, "state"),
            if active { "activa" } else { "" }
        );
    }
    let active = list
        .array_at("/data")
        .into_iter()
        .find(|s| s.get("active").and_then(|v| v.as_bool()) == Some(true));
    let Some(a) = active else { return };
    let sid = text(&a, "id");
    let rank = client.v4_get(&format!("/season/user/rank/{sid}"));
    println!();
    ui::field("Tier", &rank.str_at("/data/league"));
    ui::field(
        "Puesto",
        &format!(
            "{}/{}",
            rank.str_at("/data/rank"),
            rank.str_at("/data/total_ranks")
        ),
    );
    ui::field("Puntos", &rank.str_at("/data/total_season_points"));
    ui::field(
        "Flags",
        &format!(
            "{}/{}",
            rank.str_at("/data/total_season_flags/obtained"),
            rank.str_at("/data/total_season_flags/total")
        ),
    );
}
