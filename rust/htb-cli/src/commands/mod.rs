pub mod catalog;
pub mod machines;
pub mod profile;
pub mod vpn;

use serde_json::Value;

pub fn text(v: &Value, key: &str) -> String {
    v.get(key)
        .map(|x| match x {
            Value::String(s) => s.clone(),
            Value::Null => "-".to_string(),
            other => other.to_string(),
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

#[cfg(windows)]
pub fn open_externally(target: &str) -> bool {
    crate::win::abrir(std::ffi::OsStr::new(target))
}

#[cfg(not(windows))]
pub fn open_externally(target: &str) -> bool {
    use std::process::{Command, Stdio};
    #[cfg(target_os = "macos")]
    let cmd = ("/usr/bin/open", vec![std::ffi::OsString::from(target)]);
    #[cfg(not(target_os = "macos"))]
    let cmd = ("xdg-open", vec![std::ffi::OsString::from(target)]);
    Command::new(cmd.0)
        .args(cmd.1)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}
