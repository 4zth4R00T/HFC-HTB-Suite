use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeMode {
    Dark,
    Noche,
    Tinta,
    Zero,
    Light,
}

pub const THEMES: [ThemeMode; 5] = [ThemeMode::Dark, ThemeMode::Noche, ThemeMode::Tinta, ThemeMode::Zero, ThemeMode::Light];

impl ThemeMode {
    pub fn toggled(self) -> Self {
        if self.is_dark() {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        }
    }

    pub fn next(self) -> Self {
        let i = THEMES.iter().position(|t| *t == self).unwrap_or(0);
        THEMES[(i + 1) % THEMES.len()]
    }

    pub fn is_dark(self) -> bool {
        self != ThemeMode::Light
    }

    pub fn label(self) -> &'static str {
        match self {
            ThemeMode::Dark => "dark",
            ThemeMode::Noche => "noche",
            ThemeMode::Tinta => "tinta",
            ThemeMode::Zero => "0day",
            ThemeMode::Light => "light",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            ThemeMode::Dark => "HFC bosque",
            ThemeMode::Noche => "HFC noche",
            ThemeMode::Tinta => "HFC tinta",
            ThemeMode::Zero => "0day-notes",
            ThemeMode::Light => "HFC claro",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim() {
            "light" | "claro" => ThemeMode::Light,
            "noche" => ThemeMode::Noche,
            "tinta" => ThemeMode::Tinta,
            "0day" | "zero" | "underground" => ThemeMode::Zero,
            _ => ThemeMode::Dark,
        }
    }
}

pub struct Palette {
    pub underground: bool,
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
        ThemeMode::Zero => Palette {
            underground: true,
            bg: rgb(0x060505),
            bg_panel: rgb(0x0b1410),
            border: rgb(0x1d3b30),
            text: rgb(0xc9d6d2),
            text_secondary: rgb(0x7a9a90),
            text_dim: rgb(0x4d665d),
            accent: rgb(0x47836b),
            accent_bright: rgb(0x7ab89a),
            warning: rgb(0xc9a86a),
            error: rgb(0xc77a6a),
            info: rgb(0x73a7c0),
            special: rgb(0xa98bd1),
            warm: rgb(0xd9c07a),
        },
        ThemeMode::Dark => Palette {
            underground: false,
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
        ThemeMode::Noche => Palette {
            underground: false,
            bg: rgb(0x05080a),
            bg_panel: rgb(0x0b1316),
            border: rgb(0x16302c),
            text: rgb(0xd2dad8),
            text_secondary: rgb(0x82a39c),
            text_dim: rgb(0x43605a),
            accent: rgb(0x2b6657),
            accent_bright: rgb(0x55a18a),
            warning: rgb(0xd1ab3a),
            error: rgb(0xb85049),
            info: rgb(0x62abd0),
            special: rgb(0xb08ad0),
            warm: rgb(0xcb8549),
        },
        ThemeMode::Tinta => Palette {
            underground: false,
            bg: rgb(0x000000),
            bg_panel: rgb(0x0a120e),
            border: rgb(0x1e3d32),
            text: rgb(0xe8eeeb),
            text_secondary: rgb(0x9bbcb2),
            text_dim: rgb(0x557a6f),
            accent: rgb(0x2f7a60),
            accent_bright: rgb(0x66c49a),
            warning: rgb(0xe0bd45),
            error: rgb(0xd25c54),
            info: rgb(0x72bde2),
            special: rgb(0xc39be3),
            warm: rgb(0xdd9456),
        },
        ThemeMode::Light => Palette {
            underground: false,
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
