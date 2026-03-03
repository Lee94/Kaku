use ratatui::style::Color;
use std::sync::{LazyLock, Mutex};

struct Theme {
    primary: Color,
    secondary: Color,
    accent: Color,
    error: Color,
    text: Color,
    muted: Color,
    bg: Color,
    panel: Color,
}

fn parse_hex(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() < 6 {
        return Color::Rgb(0, 0, 0);
    }

    let r = hex
        .get(0..2)
        .and_then(|s| u8::from_str_radix(s, 16).ok())
        .unwrap_or(0);
    let g = hex
        .get(2..4)
        .and_then(|s| u8::from_str_radix(s, 16).ok())
        .unwrap_or(0);
    let b = hex
        .get(4..6)
        .and_then(|s| u8::from_str_radix(s, 16).ok())
        .unwrap_or(0);
    Color::Rgb(r, g, b)
}

fn detect_light_theme_from_config() -> bool {
    let config_path = config::user_config_path();

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("--") {
            continue;
        }
        if trimmed.starts_with("config.color_scheme") {
            if let Some(eq_pos) = trimmed.find('=') {
                let value = trimmed[eq_pos + 1..]
                    .trim()
                    .trim_matches('\'')
                    .trim_matches('"');
                return value == "Kaku Light";
            }
        }
    }
    false
}

// Cache the theme detection result for the process lifetime.
static LIGHT_THEME_CACHE: Mutex<Option<bool>> = Mutex::new(None);

pub fn is_light_theme() -> bool {
    let cached = *LIGHT_THEME_CACHE.lock().unwrap();
    if let Some(v) = cached {
        return v;
    }
    let v = detect_light_theme_from_config();
    *LIGHT_THEME_CACHE.lock().unwrap() = Some(v);
    v
}

/// Clear the cached theme detection result so that the next call to
/// `is_light_theme()` re-reads the config file.
pub fn clear_theme_cache() {
    *LIGHT_THEME_CACHE.lock().unwrap() = None;
}

// Parse each theme exactly once; color accessors borrow these statics.
static DARK_THEME: LazyLock<Theme> = LazyLock::new(|| Theme {
    primary: parse_hex("#a277ff"),
    secondary: parse_hex("#61ffca"),
    accent: parse_hex("#ffca85"),
    error: parse_hex("#ff6767"),
    text: parse_hex("#edecee"),
    muted: parse_hex("#6d6d6d"),
    bg: parse_hex("#15141b"),
    panel: parse_hex("#1f1d28"),
});

static LIGHT_THEME: LazyLock<Theme> = LazyLock::new(|| Theme {
    primary: parse_hex("#5E3DB3"),
    secondary: parse_hex("#24837B"),
    accent: parse_hex("#A08200"),
    error: parse_hex("#AF3029"),
    text: parse_hex("#403E3C"),
    muted: parse_hex("#6F6E69"),
    bg: parse_hex("#FFFCF0"),
    panel: parse_hex("#F2F0E5"),
});

fn current_theme() -> &'static Theme {
    if is_light_theme() {
        &LIGHT_THEME
    } else {
        &DARK_THEME
    }
}

pub fn purple() -> Color {
    current_theme().primary
}

pub fn green() -> Color {
    current_theme().secondary
}

pub fn yellow() -> Color {
    current_theme().accent
}

pub fn red() -> Color {
    current_theme().error
}

pub fn text_fg() -> Color {
    current_theme().text
}

pub fn muted() -> Color {
    current_theme().muted
}

pub fn bg() -> Color {
    current_theme().bg
}

pub fn panel() -> Color {
    current_theme().panel
}
