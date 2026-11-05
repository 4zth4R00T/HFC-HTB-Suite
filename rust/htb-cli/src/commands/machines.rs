use super::{text, truncate};
use crate::http::Client;
use crate::ui;
use colored::*;
use serde_json::json;

pub struct ListOptions {
    pub state: String,
    pub difficulty: Option<String>,
    pub os: Option<String>,
    pub search: Option<String>,
    pub favorites: bool,
    pub sort: Option<String>,
    pub asc: bool,
    pub completed: Option<bool>,
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

pub fn list(client: &Client, o: &ListOptions) {
    let mut qs = format!("per_page=40&page={}", o.page);
    if o.favorites {
        qs += "&todo=1";
    } else if !o.state.is_empty() && o.state != "all" {
        qs += &format!("&state={}", o.state);
    }
    if let Some(d) = &o.difficulty {
        qs += &format!("&difficulty[]={}", enc(&d.to_lowercase().replace(' ', "-")));
    }
    if let Some(x) = &o.os {
        qs += &format!("&os[]={}", enc(&x.to_lowercase()));
    }
    if let Some(s) = &o.search {
        qs += &format!("&keyword={}", enc(s));
    }
    if let Some(c) = o.completed {
        qs += &format!("&showCompleted={}", if c { "complete" } else { "incomplete" });
    }
    if let Some(sort) = &o.sort {
        qs += &format!("&sort_by={}&sort_type={}", enc(sort), if o.asc { "asc" } else { "desc" });
    }

    let r = client.v5_get(&format!("/machines?{qs}"));
    let rows = r.array_at("/data");
    if rows.is_empty() {
        ui::note("No hay resultados con esos criterios.");
        return;
    }
    let last = r.data.pointer("/meta/last_page").and_then(|v| v.as_u64()).unwrap_or(1);
    ui::heading(&format!(
        "Maquinas ({}) · {} · pagina {}/{last}",
        if o.favorites { "favoritas" } else { &o.state },
        r.str_at("/meta/total"),
        o.page
    ));
    println!(
        "  {:<3}{:<2}{:<7} {:<26} {:<9} {:<9} {:>5} {:<7} {:<11} {:>7} {:>7} Etiquetas",
        "", "", "ID", "Nombre", "OS", "Dificultad", "Pts", "Acceso", "Rating", "user", "root"
    );
    for m in &rows {
        let user = m.get("authUserInUserOwns").and_then(|v| v.as_bool()).unwrap_or(false);
        let root = m.get("authUserInRootOwns").and_then(|v| v.as_bool()).unwrap_or(false);
        let mark = match (user, root) {
            (true, true) => "✓✓".green(),
            (true, false) | (false, true) => "✓ ".yellow(),
            _ => "  ".normal(),
        };
        let fav = m.get("todo").and_then(|v| v.as_bool()).unwrap_or(false);
        let free = m.get("free").and_then(|v| v.as_bool()).unwrap_or(false);
        let diff = text(m, "difficultyText");
        let rating = m.get("rating").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let votes = m.get("ratingCount").and_then(|v| v.as_u64()).unwrap_or(0);
        let mut labels: Vec<String> = m
            .get("labels")
            .and_then(|l| l.as_array())
            .map(|a| a.iter().map(|x| text(x, "name").to_uppercase()).collect())
            .unwrap_or_default();
        if let Some(name) = m.get("retiring").and_then(|r| r.get("name")).and_then(|v| v.as_str()) {
            labels.push(format!("retira {name}"));
        }
        if text(m, "state") == "unreleased" {
            labels.push(format!("sale {}", text(m, "releaseDate").chars().take(10).collect::<String>()));
        }
        println!(
            "  {:<3}{:<2}{:<7} {:<26} {:<9} {:<9} {:>5} {:<7} {:<11} {:>7} {:>7} {}",
            mark,
            if fav { "★".yellow() } else { " ".normal() },
            text(m, "id"),
            truncate(&text(m, "name"), 26),
            text(m, "os"),
            ui::difficulty(&diff),
            text(m, "points"),
            if free { "Free" } else { "VIP" },
            if votes > 0 { format!("{rating:.1} ({votes})") } else { "-".into() },
            text(m, "userOwnsCount"),
            text(m, "rootOwnsCount"),
            labels.join(" · ")
        );
    }
}

pub fn favorite(client: &Client, kind: &str, id: &str) {
    let path = match kind {
        "challenge" => format!("/challenge/todo/update/{id}"),
        "sherlock" => format!("/sherlock/todo/update/{id}"),
        _ => format!("/machine/todo/update/{id}"),
    };
    let r = client.v4_post(&path, json!({}));
    if !r.ok {
        ui::fail(&r.message());
        return;
    }
    let ids = r.first_array(&["/info", "/data"]);
    if ids.iter().any(|v| text(v, "id") == id || v == id) {
        ui::ok("Marcado como favorito.");
    } else {
        ui::ok("Quitado de favoritos.");
    }
}

pub fn reviews(client: &Client, kind: &str, id: &str) {
    let r = client.v4_get(&format!(
        "/review/{kind}/{id}/paginated?sort_by[]=created_at&sort_type=desc&per_page=30"
    ));
    let rows = r.array_at("/data");
    ui::heading(&format!("Opiniones · {}", r.str_at("/meta/total")));
    if rows.is_empty() {
        ui::note("Todavia no hay opiniones.");
        return;
    }
    for rv in &rows {
        let st = rv.get("stars").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let user = rv.get("user").map(|u| text(u, "name")).unwrap_or_else(|| "-".into());
        println!(
            "  {} {}   {} · {}",
            format!("{}{}", "★".repeat(st.min(5)), "☆".repeat(5 - st.min(5))).yellow(),
            text(rv, "title").bold(),
            user.bright_black(),
            text(rv, "created_at").chars().take(10).collect::<String>().bright_black()
        );
        let msg = text(rv, "message");
        if msg != "-" {
            println!("      {}", truncate(&msg.replace('\n', " "), 200));
        }
    }
}

pub fn activity(client: &Client, kind: &str, id: &str) {
    let r = client.v4_get(&format!("/{kind}/activity/{id}"));
    let rows = r.first_array(&["/info/activity", "/data"]);
    ui::heading("Actividad reciente");
    if rows.is_empty() {
        ui::note("Sin actividad reciente.");
        return;
    }
    for a in rows.iter().take(60) {
        let blood = text(a, "blood_type");
        let kind = text(a, "type");
        let what = if blood != "-" {
            format!("{blood} blood").red().to_string()
        } else {
            match kind.as_str() {
                "blood" => "blood".red().to_string(),
                "own" => "solved".to_string(),
                other => format!("{other} flag"),
            }
        };
        println!("  {:<24} {:<14} {}", truncate(&text(a, "user_name"), 24), what, text(a, "date_diff").bright_black());
    }
}

pub fn changelog(client: &Client, kind: &str, id: &str) {
    let r = client.v4_get(&format!("/{kind}/changelog/{id}"));
    let rows = r.first_array(&["/info", "/data"]);
    ui::heading("Registro de cambios");
    if rows.is_empty() {
        ui::note("Sin cambios registrados.");
        return;
    }
    for c in &rows {
        let kind = match text(c, "type").as_str() {
            "1" | "enhancement" => "ENHANCEMENT",
            "2" | "info" => "INFO",
            "3" | "bug" => "BUG",
            other => other,
        }
        .to_string();
        println!(
            "  {}  [{}] {}",
            text(c, "created_at").chars().take(10).collect::<String>().bright_black(),
            kind.yellow(),
            text(c, "title").bold()
        );
        let d = text(c, "description");
        if d != "-" {
            println!("      {}", truncate(&d.replace('\n', " "), 200));
        }
    }
}

pub fn walkthroughs(client: &Client, id: &str) {
    let r = client.v4_get(&format!("/machine/walkthroughs/{id}"));
    let msg = r.data.pointer("/message").cloned().unwrap_or(serde_json::Value::Null);
    ui::heading("Guias y writeup");
    let official = msg.get("official").cloned().unwrap_or(serde_json::Value::Null);
    if text(&official, "filename") != "-" {
        let w = client.v4_get_fresh(&format!("/machine/writeup/{id}"));
        let url = w.str_at("/url");
        let shown = if url == "-" { w.message() } else { url };
        ui::field("Writeup oficial", &shown);
    }
    let video = msg.get("video").cloned().unwrap_or(serde_json::Value::Null);
    let yt = text(&video, "youtube_id");
    if yt != "-" {
        ui::field(
            "Video",
            &format!("{} · https://www.youtube.com/watch?v={yt}", text(&video, "creator_name")),
        );
    }
    let community = msg.get("writeups").and_then(|w| w.as_array()).cloned().unwrap_or_default();
    if !community.is_empty() {
        println!();
        println!("  {}", "Guias de la comunidad".bold());
        for w in community.iter().take(40) {
            println!(
                "  {:<18} {:>3}% ({:>3})  {}  {}",
                truncate(&text(w, "user_name"), 18),
                w.get("rating").and_then(|v| v.as_u64()).unwrap_or(0),
                w.get("total_ratings").and_then(|v| v.as_u64()).unwrap_or(0),
                text(w, "created_at").chars().take(10).collect::<String>().bright_black(),
                text(w, "url")
            );
        }
    }
}

pub fn tags(client: &Client, id: &str) {
    let raw = client.v4_get(&format!("/machine/profile/{id}"));
    let name = raw.str_at("/info/name");
    if name == "-" {
        ui::fail("No encontre esa maquina.");
        return;
    }
    ui::heading(&format!("{name} · ficha completa"));
    let synopsis = raw.str_at("/info/synopsis");
    if synopsis != "-" {
        println!("  {}", synopsis);
        println!();
    }
    for (who, ptr) in [("Blood user", "/info/userBlood"), ("Blood root", "/info/rootBlood")] {
        if raw.has(&format!("{ptr}/user/name")) {
            ui::field(
                who,
                &format!(
                    "{} · {}",
                    raw.str_at(&format!("{ptr}/user/name")),
                    raw.str_at(&format!("{ptr}/created_at")).chars().take(10).collect::<String>()
                ),
            );
        }
    }
    let tags = client.v4_get(&format!("/machine/tags/{id}")).array_at("/info");
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    for t in &tags {
        let cat = text(t, "category");
        let n = text(t, "name");
        match groups.iter_mut().find(|(c, _)| *c == cat) {
            Some((_, list)) => list.push(n),
            None => groups.push((cat, vec![n])),
        }
    }
    for (cat, list) in groups {
        ui::field(&cat, &list.join(", "));
    }
    let matrix = client.v4_get(&format!("/machine/graph/matrix/{id}"));
    if let Some(agg) = matrix.data.pointer("/info/aggregate") {
        let parts: Vec<String> = ["enum", "real", "cve", "custom", "ctf"]
            .iter()
            .map(|k| format!("{k} {:.1}", agg.get(*k).and_then(|v| v.as_f64()).unwrap_or(0.0)))
            .collect();
        ui::field("Matriz", &parts.join(" · "));
    }
}

pub fn search(client: &Client, term: &str) {
    let r = client.v4_get(&format!("/search/fetch?query={}", enc(term)));
    ui::heading(&format!("Busqueda global: {term}"));
    let mut any = false;
    for (key, label) in [("machines", "Maquina"), ("challenges", "Reto"), ("users", "Usuario"), ("teams", "Equipo")] {
        for v in r.array_at(&format!("/{key}")) {
            any = true;
            println!("  {:<9} {:<8} {}", label.bright_black(), text(&v, "id"), text(&v, "value"));
        }
    }
    if !any {
        ui::note("Sin resultados.");
    }
}

pub fn dashboard(client: &Client, which: &str) {
    let which = match which {
        "progress" | "inprogress" | "en-progreso" => "inprogress",
        "recommended" | "para-ti" | "foryou" => "recommended",
        "favorites" | "favoritos" => "favorites",
        other => other,
    };
    let r = client.v5_get_fresh(&format!("/user/dashboard/{which}"));
    ui::heading(&format!("Dashboard · {which}"));
    let mut any = false;
    for (key, label) in [("machines", "maquina"), ("startingPoints", "sp"), ("sherlocks", "sherlock"), ("challenges", "reto")] {
        for v in r.array_at(&format!("/{key}")) {
            any = true;
            let progress = v.get("progress").and_then(|p| p.as_f64()).unwrap_or(0.0);
            println!(
                "  {:<9} {:<7} {:<28} {:<9} {:<10} {:>4.0}%  {}/{}",
                label.bright_black(),
                text(&v, "id"),
                truncate(&text(&v, "name"), 28),
                text(&v, "os").replace('-', ""),
                ui::difficulty(&text(&v, "difficulty")),
                progress,
                text(&v, "tasksCompleted"),
                text(&v, "tasksTotal")
            );
        }
    }
    if !any {
        ui::note("Nada por aqui todavia.");
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
