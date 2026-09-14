use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    pub fn toggled(self) -> Self {
        match self {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ThemeMode::Dark => "dark",
            ThemeMode::Light => "light",
        }
    }

    pub fn parse(s: &str) -> Self {
        if s.trim() == "light" {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        }
    }
}

pub struct Palette {
    pub bg: Color,
    pub bg_panel: Color,
    pub border: Color,
    pub text: Color,
    pub text_secondary: Color,
    pub text_dim: Color,
    pub accent: Color,
    pub accent_bright: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub special: Color,
    pub warm: Color,
}

fn rgb(hex: u32) -> Color {
    Color::Rgb(
        ((hex >> 16) & 0xFF) as u8,
        ((hex >> 8) & 0xFF) as u8,
        (hex & 0xFF) as u8,
    )
}

pub fn palette(mode: ThemeMode) -> Palette {
    match mode {
        ThemeMode::Dark => Palette {
            bg: rgb(0x060505),
            bg_panel: rgb(0x0b1712),
            border: rgb(0x123028),
            text: rgb(0xc9d6d2),
            text_secondary: rgb(0x7a9a90),
            text_dim: rgb(0x3d5e54),
            accent: rgb(0x295e49),
            accent_bright: rgb(0x47836b),
            warning: rgb(0xc9a227),
            error: rgb(0xb0453f),
            info: rgb(0x4f9ec4),
            special: rgb(0xa67ec4),
            warm: rgb(0xc17a3f),
        },
        ThemeMode::Light => Palette {
            bg: rgb(0xf3f6f4),
            bg_panel: rgb(0xdde8e2),
            border: rgb(0xb9d0c5),
            text: rgb(0x0c1a16),
            text_secondary: rgb(0x295e49),
            text_dim: rgb(0x5b7a70),
            accent: rgb(0x1b453a),
            accent_bright: rgb(0x295e49),
            warning: rgb(0x96731a),
            error: rgb(0x8c332e),
            info: rgb(0x2f6f8c),
            special: rgb(0x6b4a8c),
            warm: rgb(0x8a5527),
        },
    }
}

impl Palette {
    pub fn title(&self) -> Style {
        Style::default().fg(self.text).add_modifier(Modifier::BOLD)
    }
    pub fn label(&self) -> Style {
        Style::default().fg(self.text_secondary)
    }
    pub fn value(&self) -> Style {
        Style::default().fg(self.text)
    }
    pub fn dim(&self) -> Style {
        Style::default().fg(self.text_dim)
    }
    pub fn accent_style(&self) -> Style {
        Style::default()
            .fg(self.accent_bright)
            .add_modifier(Modifier::BOLD)
    }
    pub fn ok(&self) -> Style {
        Style::default().fg(self.accent_bright)
    }
    pub fn err(&self) -> Style {
        Style::default().fg(self.error).add_modifier(Modifier::BOLD)
    }
    pub fn warn(&self) -> Style {
        Style::default().fg(self.warning)
    }
    pub fn info_style(&self) -> Style {
        Style::default().fg(self.info)
    }
    pub fn special_style(&self) -> Style {
        Style::default().fg(self.special)
    }
    pub fn warm_style(&self) -> Style {
        Style::default().fg(self.warm)
    }
    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border)
    }
    pub fn selected(&self) -> Style {
        Style::default()
            .bg(self.accent)
            .fg(self.text)
            .add_modifier(Modifier::BOLD)
    }
    pub fn key(&self) -> Style {
        Style::default()
            .fg(self.accent_bright)
            .add_modifier(Modifier::BOLD)
    }
    pub fn difficulty(&self, text: &str) -> Style {
        match text.to_lowercase().as_str() {
            "easy" | "very easy" => self.ok(),
            "medium" => self.warn(),
            "hard" => self.err(),
            "insane" => self.special_style(),
            _ => self.value(),
        }
    }
}
