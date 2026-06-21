use retro_engine::Color;

#[macro_export]
macro_rules! theme {
    () => {
        $crate::configuration!().theme
    };
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
