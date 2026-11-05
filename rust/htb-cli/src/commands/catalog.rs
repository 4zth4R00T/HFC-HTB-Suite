use super::{text, truncate};
use crate::config::Config;
use crate::http::{sanitize, Client, BASE_V4};
use crate::ui;
use serde_json::json;
use std::collections::HashMap;

pub struct ContentOptions {
    pub category: Option<String>,
    pub unsolved: bool,
    pub state: Option<String>,
    pub favorites: bool,
    pub search: Option<String>,
    pub difficulty: Option<String>,
    pub sort: Option<String>,
    pub asc: bool,
    pub completed: bool,
    pub page: u32,
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            b' ' => "+".to_string(),
            other => format!("%{other:02X}"),
        })
        .collect()
}

fn content_query(o: &ContentOptions, cats: &HashMap<String, String>) -> String {
    let mut qs = format!("per_page=40&page={}", o.page);
    if o.favorites {
        qs += "&todo=1";
    } else if let Some(st) = &o.state {
        if st != "all" {
            qs += &format!("&state={st}");
        }
    }
    if let Some(s) = &o.search {
        qs += &format!("&keyword={}", enc(s));
    }
    if let Some(d) = &o.difficulty {
        qs += &format!("&difficulty[]={}", enc(&d.to_lowercase().replace(' ', "-")));
    }
    if let Some(c) = &o.category {
        let id = cats
            .iter()
            .find(|(_, name)| name.eq_ignore_ascii_case(c))
            .map(|(id, _)| id.clone())
            .unwrap_or_else(|| c.clone());
        qs += &format!("&category[]={}", enc(&id));
    }
    if o.completed {
        qs += "&showCompleted=complete";
    } else if o.unsolved {
        qs += "&showCompleted=incomplete";
    }
    if let Some(sort) = &o.sort {
        qs += &format!("&sort_by={}&sort_type={}", enc(sort), if o.asc { "asc" } else { "desc" });
    }
    qs
}

pub fn challenges(client: &Client, o: &ContentOptions) {
    let cats: HashMap<String, String> = client
        .v4_get("/challenge/categories/list")
        .array_at("/info")
        .iter()
        .map(|c| (text(c, "id"), text(c, "name")))
        .collect();
    let r = client.v4_get(&format!("/challenges?{}", content_query(o, &cats)));
    let rows = r.first_array(&["/data", "/challenges"]);
    if rows.is_empty() {
        ui::note("La API no devolvio retos con esos criterios.");
        return;
    }
    let last = r.data.pointer("/meta/last_page").and_then(|v| v.as_u64()).unwrap_or(1);
    ui::heading(&format!("Challenges · {} · pagina {}/{last}", r.str_at("/meta/total"), o.page));
    for c in &rows {
        let name = cats
            .get(&text(c, "category_id"))
            .cloned()
            .unwrap_or_else(|| text(c, "category_name"));
        let solved = c.get("is_owned").or_else(|| c.get("authUserSolve")).and_then(|v| v.as_bool()).unwrap_or(false);
        let fav = c.get("todo").or_else(|| c.get("isTodo")).and_then(|v| v.as_bool()).unwrap_or(false);
        let votes = c.get("rating_count").and_then(|v| v.as_u64()).unwrap_or(0);
        let rating = c.get("rating").and_then(|v| v.as_f64()).unwrap_or(0.0);
        println!(
            "  {:<2}{:<2}{:<7} {:<32} {:<16} {:<10} {:>6} {:<10} {}",
            if solved { "✓" } else { "" },
            if fav { "★" } else { "" },
            text(c, "id"),
            truncate(&text(c, "name"), 32),
            truncate(&name, 16),
            text(c, "difficulty"),
            text(c, "solves"),
            if votes > 0 { format!("{rating:.1} ({votes})") } else { "-".into() },
            text(c, "release_date").chars().take(10).collect::<String>()
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

pub fn sherlocks(client: &Client, o: &ContentOptions) {
    let cats: HashMap<String, String> = client
        .v4_get("/sherlocks/categories/list")
        .first_array(&["/info", "/data"])
        .iter()
        .map(|c| (text(c, "id"), text(c, "name")))
        .collect();
    let r = client.v4_get(&format!("/sherlocks?{}", content_query(o, &cats)));
    let rows = r.first_array(&["/data", "/sherlocks"]);
    if rows.is_empty() {
        ui::note("Sin resultados con esos criterios.");
        return;
    }
    let last = r.data.pointer("/meta/last_page").and_then(|v| v.as_u64()).unwrap_or(1);
    ui::heading(&format!("Sherlocks · {} · pagina {}/{last}", r.str_at("/meta/total"), o.page));
    for s in &rows {
        let owned = s.get("is_owned").and_then(|v| v.as_bool()).unwrap_or(false);
        let fav = s.get("todo").or_else(|| s.get("isTodo")).and_then(|v| v.as_bool()).unwrap_or(false);
        let progress = s.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
        println!(
            "  {:<2}{:<2}{:<7} {:<32} {:<16} {:<10} {:>4.0}% {:>6} {}",
            if owned { "✓" } else { "" },
            if fav { "★" } else { "" },
            text(s, "id"),
            truncate(&text(s, "name"), 32),
            truncate(&text(s, "category_name"), 16),
            text(s, "difficulty"),
            progress,
            text(s, "solves"),
            text(s, "release_date").chars().take(10).collect::<String>()
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

pub fn seasons(client: &Client, leaderboard: bool, rewards: bool, page: u32) {
    let list = client.v4_get("/season/list");
    let active = list
        .array_at("/data")
        .into_iter()
        .find(|s| s.get("active").and_then(|v| v.as_bool()) == Some(true));
    let Some(a) = active else {
        ui::note("No hay season activa.");
        return;
    };
    let sid = text(&a, "id");
    if leaderboard {
        let r = client.v4_get(&format!("/season/players/leaderboard?per_page=15&season={sid}&page={page}"));
        let last = r.data.pointer("/meta/last_page").and_then(|v| v.as_u64()).unwrap_or(1);
        ui::heading(&format!("{} · leaderboard · pagina {page}/{last}", text(&a, "name")));
        println!("  {:<6}{:<24}{:<10}{:>7}{:>7}{:>7}", "#", "", "tier", "pts", "user", "root");
        for p in r.array_at("/data") {
            println!(
                "  {:<6}{:<24}{:<10}{:>7}{:>7}{:>7}",
                text(&p, "rank"),
                truncate(&text(&p, "name"), 22),
                text(&p, "league_rank"),
                text(&p, "points"),
                text(&p, "user_owns"),
                text(&p, "root_owns")
            );
        }
        let me = client.v4_get(&format!("/season/user/rank/{sid}"));
        println!();
        ui::field("Tu puesto", &format!("#{} · {} · {} pts", me.str_at("/data/rank"), me.str_at("/data/league"), me.str_at("/data/total_season_points")));
        return;
    }
    if rewards {
        let r = client.v4_get(&format!("/season/rewards/{sid}"));
        ui::heading(&format!("{} · recompensas", text(&a, "name")));
        for block in r.array_at("/data") {
            let Some(types) = block.get("reward_types") else { continue };
            let items: Vec<serde_json::Value> = match types {
                serde_json::Value::Array(x) => x.clone(),
                other => vec![other.clone()],
            };
            for t in items {
                println!("  {}", text(&t, "name"));
                for g in t.get("groups").and_then(|v| v.as_array()).cloned().unwrap_or_default() {
                    println!(
                        "    {:<12} {:<16} top {}  {} flags",
                        text(&g, "name"),
                        text(&g, "subtitle"),
                        text(&g, "rank_up_requirement"),
                        text(&g, "flags_needed")
                    );
                }
            }
        }
        return;
    }
    ui::heading("Seasons");
    for s in list.array_at("/data") {
        let act = s.get("active").and_then(|v| v.as_bool()).unwrap_or(false);
        println!(
            "  {:<5} {:<30} {:<12} {}",
            text(&s, "id"),
            truncate(&text(&s, "name"), 30),
            text(&s, "state"),
            if act { "activa" } else { "" }
        );
    }
    let rank = client.v4_get(&format!("/season/user/rank/{sid}"));
    println!();
    ui::field("Tier", &rank.str_at("/data/league"));
    ui::field(
        "Puesto",
        &format!("{}/{}", rank.str_at("/data/rank"), rank.str_at("/data/total_ranks")),
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
    let current = client.v4_get("/season/machine/active");
    if current.has("/data/name") {
        ui::field(
            "Maquina actual",
            &format!(
                "{} · user {} / root {} pts",
                current.str_at("/data/name"),
                current.str_at("/data/user_own_points"),
                current.str_at("/data/root_own_points")
            ),
        );
    }
    println!();
    for (i, m) in client.v4_get("/season/machines").first_array(&["/data"]).iter().enumerate() {
        let root = m.get("is_owned_root").and_then(|v| v.as_bool()).unwrap_or(false);
        let user = m.get("is_owned_user").and_then(|v| v.as_bool()).unwrap_or(false);
        let released = m.get("is_released").and_then(|v| v.as_bool()).unwrap_or(true);
        let unknown = m.get("unknown").and_then(|v| v.as_bool()).unwrap_or(false);
        println!(
            "  {:<3} semana {:<2} {:<22} {:<9} {:<8} {}",
            if root { "✓✓" } else if user { "✓" } else { "" },
            i + 1,
            if unknown { "?".to_string() } else { truncate(&text(m, "name"), 22) },
            text(m, "os"),
            text(m, "difficulty_text"),
            if released { String::new() } else { format!("sale {}", text(m, "release_time").chars().take(16).collect::<String>()) }
        );
    }
}

pub fn rankings(client: &Client, kind: &str) {
    let kind = match kind {
        "users" | "teams" | "universities" | "countries" => kind,
        "equipos" => "teams",
        "universidades" => "universities",
        "paises" => "countries",
        _ => "users",
    };
    let r = client.v4_get(&format!("/rankings/{kind}"));
    let rows = r.array_at("/data");
    ui::heading(&format!("Rankings · {kind}"));
    if rows.is_empty() {
        ui::fail(&r.message());
        return;
    }
    for e in rows.iter().take(100) {
        let name = if kind == "countries" {
            let cn = text(e, "country_name");
            if cn == "-" { text(e, "country") } else { format!("{} {cn}", text(e, "country")) }
        } else {
            text(e, "name")
        };
        println!(
            "  {:<6}{:<28}{:>10}{:>8}{:>8}",
            text(e, "rank"),
            truncate(&name, 26),
            text(e, "points"),
            text(e, "user_owns"),
            text(e, "root_owns")
        );
    }
}

pub fn pwnbox(client: &Client) {
    let status = client.v4_get_fresh("/pwnbox/status");
    let usage = client.v4_get_fresh("/pwnbox/usage");
    ui::heading("Pwnbox (Labs)");
    if status.str_at("/status") == "-" {
        ui::note(&status.message());
    } else {
        for key in ["status", "hostname", "expires_at", "location"] {
            let v = status.str_at(&format!("/{key}"));
            if v != "-" {
                ui::field(key, &v);
            }
        }
    }
    ui::field("Minutos usados", &format!("{} / {}", usage.str_at("/used"), usage.str_at("/allowed")));
    ui::field("Sesiones", &usage.str_at("/sessions"));
}
