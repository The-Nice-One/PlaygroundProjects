use retro_engine::Color;
use std::sync::OnceLock;

pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
}

pub static THEME: OnceLock<Theme> = OnceLock::new();

pub fn init_theme(primary: String, secondary: String, accent: String) {
    THEME.get_or_init(|| {
        let primary = color_from_string(primary);
        let secondary = color_from_string(secondary);
        let accent = color_from_string(accent);
        Theme {
            primary,
            secondary,
            accent,
        }
    });
}

pub fn color_from_string(color: String) -> Color {
    match color.as_str() {
        "default" => Color::Reset,
        "white" => Color::White,
        "grey" => Color::Grey,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "black" => Color::Black,
        "dark-grey" => Color::DarkGrey,
        "dark-red" => Color::DarkRed,
        "dark-green" => Color::DarkGreen,
        "dark-yellow" => Color::DarkYellow,
        "dark-blue" => Color::DarkBlue,
        "dark-magenta" => Color::DarkMagenta,
        "dark-cyan" => Color::DarkCyan,
        _ => Color::Reset,
    }
}
