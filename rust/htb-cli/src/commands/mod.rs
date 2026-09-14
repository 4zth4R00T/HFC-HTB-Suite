pub mod catalog;
pub mod machines;
pub mod profile;
pub mod vpn;

use serde_json::Value;

pub fn text(v: &Value, key: &str) -> String {
    v.get(key)
        .map(|x| {
            x.as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| x.to_string())
        })
        .unwrap_or_else(|| "-".into())
}

pub fn truncate(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        s.to_string()
    } else {
        s.chars().take(width.saturating_sub(1)).collect::<String>() + "…"
    }
}
