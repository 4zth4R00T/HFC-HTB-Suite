use crate::brand;
use colored::*;
use std::io::{self, Write};
use zeroize::Zeroizing;

pub fn banner() {
    for row in brand::HFC {
        println!("{}", row.green());
    }
    println!();
    println!(
        "{} — {}",
        brand::AUTHOR.white(),
        brand::AUTHOR_INSTAGRAM.bright_black()
    );
    println!(
        "{} — {}",
        brand::CREW.green(),
        brand::CREW_INSTAGRAM.bright_black()
    );
    rule();
}

pub fn rule() {
    println!("{}", "─".repeat(72).bright_black());
}

pub fn heading(title: &str) {
    println!();
    println!("{}", title.bold().green());
    rule();
}

pub fn ok(msg: &str) {
    println!("  {} {}", "OK".green().bold(), msg);
}

pub fn fail(msg: &str) {
    println!("  {} {}", "!!".red().bold(), msg);
}

pub fn note(msg: &str) {
    println!("  {}", msg.yellow());
}

pub fn field(label: &str, value: &str) {
    println!("  {:<18}{}", label.bright_black(), value);
}

pub fn prompt(label: &str) -> String {
    print!("  {label}: ");
    let _ = io::stdout().flush();
    let mut s = String::new();
    let _ = io::stdin().read_line(&mut s);
    s.trim().to_string()
}

pub fn prompt_secret(label: &str) -> Option<Zeroizing<String>> {
    use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
    use crossterm::terminal;

    if !std::io::IsTerminal::is_terminal(&io::stdin()) {
        return None;
    }
    print!("  {label}: ");
    let _ = io::stdout().flush();

    if terminal::enable_raw_mode().is_err() {
        println!();
        return None;
    }
    let mut secret = Zeroizing::new(String::new());
    let mut cancelled = false;
    loop {
        let Ok(ev) = event::read() else {
            cancelled = true;
            break;
        };
        let Event::Key(key) = ev else { continue };
        if key.kind == KeyEventKind::Release {
            continue;
        }
        match key.code {
            KeyCode::Enter => break,
            KeyCode::Esc => {
                cancelled = true;
                break;
            }
            KeyCode::Backspace => {
                secret.pop();
            }
            KeyCode::Char('c') | KeyCode::Char('d')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                cancelled = true;
                break;
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                secret.clear();
            }
            KeyCode::Char(c) => secret.push(c),
            _ => {}
        }
    }
    let _ = terminal::disable_raw_mode();
    println!();
    if cancelled {
        return None;
    }
    Some(secret)
}

pub fn difficulty(text: &str) -> ColoredString {
    match text.to_lowercase().as_str() {
        "easy" | "very easy" => text.green(),
        "medium" => text.yellow(),
        "hard" => text.red(),
        "insane" => text.magenta(),
        _ => text.normal(),
    }
}
