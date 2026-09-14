use crate::config::Config;
use crate::http::{Client, BASE_V4};
use crate::ui;
use serde_json::json;

pub fn list(client: &Client, product: &str) {
    let r = client.v4_get(&format!("/connections/servers?product={product}"));
    if r.str_at("/status") != "true" {
        ui::fail(&format!(
            "No pude listar '{product}'. Puede que tu plan no lo incluya."
        ));
        ui::note(&r.message());
        return;
    }
    ui::heading(&format!("VPN · {product}"));
    ui::field(
        "Servidor asignado",
        &format!(
            "{} (ID {}, {})",
            r.str_at("/data/assigned/friendly_name"),
            r.str_at("/data/assigned/id"),
            r.str_at("/data/assigned/location")
        ),
    );
    println!();
    println!(
        "  {:<8} {:<30} {:<12} {}",
        "ID", "Servidor", "Localidad", "Clientes"
    );
    if let Some(regions) = r.data.pointer("/data/options").and_then(|v| v.as_object()) {
        for (_, kinds) in regions {
            let Some(kinds) = kinds.as_object() else { continue };
            for (_, kind) in kinds {
                let Some(servers) = kind.get("servers").and_then(|v| v.as_object()) else {
                    continue;
                };
                for (_, srv) in servers {
                    let full = srv.get("full").and_then(|v| v.as_bool()).unwrap_or(false);
                    println!(
                        "  {:<8} {:<30} {:<12} {}{}",
                        srv.get("id").map(|v| v.to_string()).unwrap_or_default(),
                        srv.get("friendly_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?"),
                        srv.get("location").and_then(|v| v.as_str()).unwrap_or("?"),
                        srv.get("current_clients")
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        if full { " (lleno)" } else { "" }
                    );
                }
            }
        }
    }
}

pub fn switch(client: &Client, server_id: &str) {
    let res = client.v4_post(&format!("/connections/servers/switch/{server_id}"), json!({}));
    if res.str_at("/status") == "true" || res.ok {
        ui::ok(&format!(
            "{} → {}",
            res.message(),
            res.str_at("/data/friendly_name")
        ));
    } else {
        ui::fail(&res.message());
    }
}

pub fn download(client: &Client, cfg: &Config, product: &str, server_id: &str, tcp: bool) {
    let assigned = client
        .v4_get(&format!("/connections/servers?product={product}"))
        .str_at("/data/assigned/id");
    if assigned != server_id {
        ui::note("HTB solo entrega el .ovpn del servidor asignado, asi que te cambio primero.");
        switch(client, server_id);
    }
    let proto = if tcp { 1 } else { 0 };
    let name = if tcp { "tcp" } else { "udp" };
    let out = cfg
        .download_dir
        .join(format!("htb_{product}_{server_id}_{name}.ovpn"));
    match client.download(
        &format!("{BASE_V4}/access/ovpnfile/{server_id}/{proto}"),
        &out,
    ) {
        Ok(size) if size > 500 => {
            let _ = std::fs::copy(&out, cfg.vpn_dir.join("current.ovpn"));
            ui::ok(&format!("Guardado en {}", out.display()));
            ui::note(&format!("Copia como current.ovpn en {}", cfg.vpn_dir.display()));
            ui::note(&format!("Para conectarte: sudo openvpn {}", out.display()));
        }
        _ => {
            let _ = std::fs::remove_file(&out);
            ui::fail("La descarga no salio bien. Revisa el ID de servidor o tu plan.");
        }
    }
}
