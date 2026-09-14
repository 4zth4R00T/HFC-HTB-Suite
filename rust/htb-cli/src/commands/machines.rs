use super::{text, truncate};
use crate::http::Client;
use crate::ui;
use colored::*;
use serde_json::json;

pub fn list(
    client: &Client,
    state: &str,
    difficulty: Option<&str>,
    os: Option<&str>,
    search: Option<&str>,
) {
    let mut qs = format!("per_page=40&page=1");
    if !state.is_empty() {
        qs += &format!("&state={state}");
    }
    if let Some(d) = difficulty {
        qs += &format!("&difficulty={d}");
    }
    if let Some(o) = os {
        qs += &format!("&os={o}");
    }
    if let Some(s) = search {
        qs += &format!("&keyword={s}");
    }

    let rows = client.v5_get(&format!("/machines?{qs}")).array_at("/data");
    if rows.is_empty() {
        ui::note("No hay resultados con esos criterios.");
        return;
    }
    ui::heading(&format!("Maquinas ({state})"));
    println!(
        "  {:<3} {:<7} {:<26} {:<9} {:<9} {:>5} {:<7}",
        "", "ID", "Nombre", "OS", "Dificultad", "Pts", "Acceso"
    );
    for m in &rows {
        let user = m
            .get("authUserInUserOwns")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let root = m
            .get("authUserInRootOwns")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let mark = match (user, root) {
            (true, true) => "✓✓".green(),
            (true, false) | (false, true) => "✓ ".yellow(),
            _ => "  ".normal(),
        };
        let free = m.get("free").and_then(|v| v.as_bool()).unwrap_or(false);
        let diff = text(m, "difficultyText");
        println!(
            "  {:<3} {:<7} {:<26} {:<9} {:<9} {:>5} {:<7}",
            mark,
            text(m, "id"),
            truncate(&text(m, "name"), 26),
            text(m, "os"),
            ui::difficulty(&diff),
            text(m, "points"),
            if free { "Free" } else { "VIP" }
        );
    }
}

pub fn info(client: &Client, id: &str) {
    let raw = client.v4_get(&format!("/machine/profile/{id}"));
    let name = raw.str_at("/info/name");
    if name == "-" {
        ui::fail("No encontre esa maquina.");
        return;
    }
    ui::heading(&name);
    ui::field("ID", id);
    ui::field("Sistema", &raw.str_at("/info/os"));
    ui::field("Dificultad", &raw.str_at("/info/difficultyText"));
    ui::field("Puntos", &raw.str_at("/info/points"));
    ui::field("IP", &raw.str_at("/info/playInfo/ip"));
    ui::field(
        "Owns",
        &format!(
            "user {} · root {}",
            raw.str_at("/info/user_owns_count"),
            raw.str_at("/info/root_owns_count")
        ),
    );
    ui::field("Valoracion", &raw.str_at("/info/stars"));
    ui::field(
        "Publicada",
        &raw.str_at("/info/release").chars().take(10).collect::<String>(),
    );
}

pub fn active(client: &Client) {
    let act = client.v4_get("/machine/active");
    let name = act.str_at("/info/name");
    if name == "-" {
        ui::note("No tienes ninguna maquina corriendo.");
        return;
    }
    ui::heading(&name);
    ui::field("ID", &act.str_at("/info/id"));
    ui::field("IP", &act.str_at("/info/ip"));
    ui::field("Caduca", &act.str_at("/info/expires_at"));
}

pub fn spawn(client: &Client, id: &str) {
    let act = client.v4_get("/machine/active");
    if act.str_at("/info/name") != "-" {
        let current = act.str_at("/info/name");
        ui::note(&format!("Ya tienes activa {current}."));
        let answer = ui::prompt("La paro y lanzo esta? [s/N]");
        if !answer.eq_ignore_ascii_case("s") && !answer.eq_ignore_ascii_case("y") {
            return;
        }
        let aid = act.str_at("/info/id");
        client.v4_post(
            "/vm/terminate",
            json!({ "machine_id": aid.parse::<i64>().unwrap_or(0) }),
        );
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    let res = client.v4_post(
        "/vm/spawn",
        json!({ "machine_id": id.parse::<i64>().unwrap_or(0) }),
    );
    let msg = res.message();
    if crate::http::read_action_answer(&msg, res.ok) == crate::http::Verdict::Accepted {
        ui::ok(&msg);
        ui::field("IP", &wait_for_ip(client));
    } else {
        ui::fail(&msg);
    }
}

fn wait_for_ip(client: &Client) -> String {
    for _ in 0..12 {
        std::thread::sleep(std::time::Duration::from_secs(3));
        let ip = client.v4_get("/machine/active").str_at("/info/ip");
        if ip != "-" && !ip.is_empty() {
            return ip;
        }
    }
    "todavia sin asignar, revisa en unos segundos".to_string()
}

fn act_on(client: &Client, endpoint: &str, label: &str) {
    let act = client.v4_get("/machine/active");
    if act.str_at("/info/name") == "-" {
        ui::note("No tienes ninguna maquina corriendo.");
        return;
    }
    let aid = act.str_at("/info/id");
    let res = client.v4_post(
        endpoint,
        json!({ "machine_id": aid.parse::<i64>().unwrap_or(0) }),
    );
    let msg = res.message();
    if crate::http::read_action_answer(&msg, res.ok) == crate::http::Verdict::Accepted {
        ui::ok(&format!("{label}: {msg}"));
    } else {
        ui::fail(&msg);
    }
}

pub fn stop(client: &Client) {
    act_on(client, "/vm/terminate", "Parada");
}

pub fn reset(client: &Client) {
    act_on(client, "/vm/reset", "Reinicio");
}

pub fn extend(client: &Client) {
    act_on(client, "/vm/extend", "Extension");
}

pub fn flag(client: &Client, id: &str, flag_value: &str, difficulty: u8) {
    let res = client.v5_post(
        "/machine/own",
        json!({
            "flag": flag_value,
            "id": id.parse::<i64>().unwrap_or(0),
            "difficulty": difficulty
        }),
    );
    let msg = res.message();
    match crate::http::read_flag_answer(&msg) {
        crate::http::Verdict::Accepted => ui::ok(&format!("Flag aceptada — {msg}")),
        crate::http::Verdict::Rejected => ui::fail(&msg),
        crate::http::Verdict::Unclear => ui::note(&msg),
    }
}
