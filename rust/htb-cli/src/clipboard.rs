
use std::io::Write;
use std::process::{Command, Stdio};

fn feed(program: &str, args: &[&str], text: &str) -> bool {
    let child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = child else { return false };
    if let Some(stdin) = child.stdin.as_mut() {
        if stdin.write_all(text.as_bytes()).is_err() {
            return false;
        }
    }
    drop(child.stdin.take());
    matches!(child.wait(), Ok(status) if status.success())
}

fn read_from(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program)
        .args(args)
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn en_wsl() -> bool {
    std::fs::read_to_string("/proc/version")
        .map(|v| v.to_lowercase().contains("microsoft"))
        .unwrap_or(false)
}

pub fn copy(text: &str) -> bool {
    feed("wl-copy", &[], text)
        || feed("xclip", &["-selection", "clipboard"], text)
        || feed("xsel", &["--clipboard", "--input"], text)
        || feed("pbcopy", &[], text)
        || (en_wsl() && feed("clip.exe", &[], text))
}

pub fn paste() -> Option<String> {
    read_from("wl-paste", &["--no-newline"])
        .or_else(|| read_from("xclip", &["-selection", "clipboard", "-o"]))
        .or_else(|| read_from("xsel", &["--clipboard", "--output"]))
        .or_else(|| read_from("pbpaste", &[]))
        .or_else(|| {
            en_wsl()
                .then(|| read_from("powershell.exe", &["-NoProfile", "-Command", "Get-Clipboard"]))
                .flatten()
        })
        .map(|s| s.trim_end_matches(['\n', '\r']).to_string())
}
