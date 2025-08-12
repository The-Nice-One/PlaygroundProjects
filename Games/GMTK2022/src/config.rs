use emerald::*;
use image::{load_from_memory_with_format, ImageFormat};

pub fn config_settings() -> GameSettings {
    let mut settings = GameSettings::default();

    // post game jam changes
    let small_icon: [u8; 1024] = load_from_memory_with_format(
        include_bytes!("../assets/icons/small_icon.png"),
        ImageFormat::Png,
    )
    .unwrap()
    .to_rgba8()
    .to_vec()
    .try_into()
    .unwrap();

    let medium_icon: [u8; 4096] = load_from_memory_with_format(
        include_bytes!("../assets/icons/medium_icon.png"),
        ImageFormat::Png,
    )
    .unwrap()
    .to_rgba8()
    .to_vec()
    .try_into()
    .unwrap();

    let big_icon: [u8; 16384] = load_from_memory_with_format(
        include_bytes!("../assets/icons/big_icon.png"),
        ImageFormat::Png,
    )
    .unwrap()
    .to_rgba8()
    .to_vec()
    .try_into()
    .unwrap();

    let render_settings = RenderSettings {
        fullscreen: false,
        resizable_window: false,
        icon: Some(Icon {
            small: small_icon,
            medium: medium_icon,
            big: big_icon,
        }),
        ..Default::default()
    };
    settings.render_settings = render_settings;
    settings.title = "Castle of the Dice".to_string();
    settings
}
