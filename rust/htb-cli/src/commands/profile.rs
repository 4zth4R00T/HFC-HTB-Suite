use crate::brand;
use crate::http::{Client, STORE_URL};
use crate::ui;

fn round_points(raw: &str) -> String {
    match raw.parse::<f64>() {
        Ok(v) if v.fract().abs() < f64::EPSILON => format!("{v:.0}"),
        Ok(v) => format!("{v:.2}"),
        Err(_) => raw.to_string(),
    }
}

pub fn show(client: &Client) {
    let info = client.v4_get("/user/info");
    if !info.ok {
        ui::fail("El token no autentica contra la API.");
        return;
    }
    let uid = info.str_at("/info/id");
    let basic = client.v4_get(&format!("/user/profile/basic/{uid}"));

    ui::heading(&basic.str_at("/profile/name"));
    ui::field(
        "Rango",
        &format!(
            "{} (#{})",
            basic.str_at("/profile/rank"),
            basic.str_at("/profile/ranking")
        ),
    );
    ui::field(
        "Siguiente rango",
        &format!(
            "{} — {}% ({} pts)",
            basic.str_at("/profile/next_rank"),
            basic.str_at("/profile/current_rank_progress"),
            round_points(&basic.str_at("/profile/next_rank_points"))
        ),
    );
    ui::field("Puntos", &basic.str_at("/profile/points"));
    ui::field(
        "Owns",
        &format!(
            "user {} · system {}",
            basic.str_at("/profile/user_owns"),
            basic.str_at("/profile/system_owns")
        ),
    );
    ui::field(
        "Bloods",
        &format!(
            "user {} · system {} · challenge {}",
            basic.str_at("/profile/user_bloods"),
            basic.str_at("/profile/system_bloods"),
            basic.str_at("/profile/challenge_bloods")
        ),
    );
    ui::field("Respects", &basic.str_at("/profile/respects"));
    ui::field("Pais", &basic.str_at("/profile/country_name"));
    ui::field("Servidor VPN", &basic.str_at("/profile/server"));
    ui::field(
        "En HTB desde",
        &basic
            .str_at("/profile/joined_date")
            .chars()
            .take(10)
            .collect::<String>(),
    );
    ui::field("Equipo", &basic.str_at("/profile/team"));
    ui::rule();
    ui::field("Plan", &info.str_at("/info/subscriptionType"));
    ui::field(
        "Acceso VIP",
        if info.str_at("/info/canAccessVIP") == "true" {
            "si"
        } else {
            "no"
        },
    );

    let subs = client.v4_get("/user/subscriptions").array_at("/subscriptions");
    if subs.is_empty() {
        ui::note(&format!("Sin suscripcion de pago. {STORE_URL}"));
    } else {
        for s in subs {
            println!(
                "    · {} ({})",
                s.get("name").and_then(|v| v.as_str()).unwrap_or("?"),
                s.get("status").and_then(|v| v.as_str()).unwrap_or("?")
            );
        }
    }

    let list = client.v4_get("/season/list");
    let active = list
        .array_at("/data")
        .into_iter()
        .find(|s| s.get("active").and_then(|v| v.as_bool()) == Some(true));
    let Some(a) = active else {
        ui::rule();
        ui::note("Ahora mismo no hay season activa.");
        return;
    };
    let sid = a.get("id").map(|v| v.to_string()).unwrap_or_default();
    let rank = client.v4_get(&format!("/season/user/rank/{sid}"));
    ui::rule();
    ui::field(
        "Season",
        a.get("name").and_then(|v| v.as_str()).unwrap_or("-"),
    );
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

pub fn credits() {
    ui::banner();
    ui::field("Programa de", brand::AUTHOR);
    ui::field("", brand::AUTHOR_INSTAGRAM);
    ui::field("", brand::AUTHOR_GITHUB);
    println!();
    ui::field("Crew", brand::CREW);
    ui::field("", brand::CREW_INSTAGRAM);
    ui::field("", brand::CREW_BLOG);
    println!();
    ui::field("Gracias a", &brand::thanks_line());
    ui::note("por el apoyo y por la inspiracion que llevo a hacer esto");
}
