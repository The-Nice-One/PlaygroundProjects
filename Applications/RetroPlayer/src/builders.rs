use crate::theme::THEME;
use crate::{PlayerSession, Terminal, VolumeBar};
use kira::sound::static_sound::StaticSoundHandle;
use retro_engine::components::*;
use retro_engine::utilities::length;
use retro_engine::Stylize;

pub fn new_header() -> retro_engine::components::Grid {
    let mut header = Grid::new((2, 1));
    header.data[0] = GridItem::Button(Button::from(
        StatefulString::from("⋮".with(THEME.get().unwrap().primary))
            .hovered("⋮".with(THEME.get().unwrap().accent))
            .active("⋮".with(THEME.get().unwrap().accent).bold()),
    ));
    header.data[1] = GridItem::Button(Button::from(
        StatefulString::from("x".with(THEME.get().unwrap().primary))
            .hovered("x".with(THEME.get().unwrap().accent))
            .active("x".with(THEME.get().unwrap().accent).bold()),
    ));

    header.left_spacer = StatefulString::from("[  ".with(THEME.get().unwrap().primary))
        .hovered("[  ".with(THEME.get().unwrap().accent))
        .active("[  ".with(THEME.get().unwrap().accent).bold())
        .clone();
    header.right_spacer = StatefulString::from("  ]".with(THEME.get().unwrap().primary))
        .hovered("  ]".with(THEME.get().unwrap().accent))
        .active("  ]".with(THEME.get().unwrap().accent).bold())
        .clone();
    header.horizontal_spacer = "  ".into();
    header
}

pub fn new_audio_controls() -> retro_engine::components::Grid {
    let mut audio_controls = Grid::new((5, 1));
    audio_controls.data[0] = GridItem::Button(Button::from(
        StatefulString::from("⇆".with(THEME.get().unwrap().primary))
            .hovered("⇆".with(THEME.get().unwrap().accent))
            .active("⇆".with(THEME.get().unwrap().accent).bold()),
    ));
    audio_controls.data[1] = GridItem::Button(Button::from(
        StatefulString::from("⭰".with(THEME.get().unwrap().primary))
            .hovered("⭰".with(THEME.get().unwrap().accent))
            .active("⭰".with(THEME.get().unwrap().accent).bold()),
    ));
    audio_controls.data[2] = GridItem::Toggle(Toggle::new(
        StatefulString::from("⏵".with(THEME.get().unwrap().primary))
            .hovered("⏵".with(THEME.get().unwrap().accent))
            .active("⏵".with(THEME.get().unwrap().accent).bold())
            .into(),
        StatefulString::from("⏸".with(THEME.get().unwrap().primary))
            .hovered("Ⅱ".with(THEME.get().unwrap().accent))
            .active("Ⅱ".with(THEME.get().unwrap().accent).bold())
            .into(),
        true,
    ));
    audio_controls.data[3] = GridItem::Button(Button::from(
        StatefulString::from("⭲".with(THEME.get().unwrap().primary))
            .hovered("⭲".with(THEME.get().unwrap().accent))
            .active("⭲".with(THEME.get().unwrap().accent).bold()),
    ));
    audio_controls.data[4] = GridItem::Button(Button::from(
        StatefulString::from("⮌".with(THEME.get().unwrap().primary))
            .hovered("⮌".with(THEME.get().unwrap().accent))
            .active("⮌".with(THEME.get().unwrap().accent).bold()),
    ));
    // audio_controls.data[2] = StatefulString::from("⏵Ⅱ⏸‖")
    //     .hovered("⏵".with(THEME.get().unwrap().accent))
    //     .active("⏵".with(THEME.get().unwrap().accent).bold())
    //     .into();

    audio_controls.left_spacer = StatefulString::from("[  ".with(THEME.get().unwrap().primary))
        .hovered("[  ".with(THEME.get().unwrap().accent))
        .active("[  ".with(THEME.get().unwrap().accent).bold())
        .clone();
    audio_controls.right_spacer = StatefulString::from("  ]".with(THEME.get().unwrap().primary))
        .hovered("  ]".with(THEME.get().unwrap().accent))
        .active("  ]".with(THEME.get().unwrap().accent).bold())
        .clone();
    audio_controls.horizontal_spacer = "  ".into();
    audio_controls
}

pub fn new_player_bar() -> retro_engine::components::ProgressBar {
    let mut player_bar = ProgressBar::default();
    player_bar.left("━".with(THEME.get().unwrap().accent));
    player_bar.pointer(vec![
        "╺".with(THEME.get().unwrap().primary),
        "╸".with(THEME.get().unwrap().accent),
    ]);
    player_bar.right("━".with(THEME.get().unwrap().primary));

    player_bar.minimum = 0;
    player_bar.maximum = 500;
    player_bar.value = 0;

    player_bar.width = 100;
    player_bar
}

pub fn update_view(
    player_bar: &mut ProgressBar,
    side_bar: &VerticalLine,
    song_list: &Grid,
    header: &Grid,
    control_panel: &mut Text,
    audio_controls: &Grid,
    volume_bar: &VolumeBar,
    terminal: &Terminal,
    sound: &Option<StaticSoundHandle>,
) -> String {
    player_bar.value = if let Some(sound) = sound.as_ref() {
        sound.position() as u32
    } else {
        0
    };
    let timer = format!(
        " {:02}:{:02} - {:02}:{:02}",
        player_bar.value / 60,
        player_bar.value % 60,
        player_bar.maximum / 60,
        player_bar.maximum % 60
    )
    .with(THEME.get().unwrap().primary)
    .to_string();

    let mut view = retro_engine::scene::align_horizontally(
        side_bar.display(),
        song_list.display(),
        "".to_string(),
    );

    let mut song_names = vec![];
    for song in song_list.data.iter() {
        if let GridItem::StatefulString(song_name) = song {
            song_names.push(song_name.default.clone());
        }
    }

    let song_list_length = retro_engine::utilities::max_length(&song_names);
    let occupied_space = song_list_length + 2;
    let left_space = terminal.screen.width - occupied_space as u16 - 1;

    let application_name = "Retro Player ♪"
        .with(THEME.get().unwrap().accent)
        .to_string();
    let header_spacer = Text::new(
        "",
        Some(
            left_space
                - retro_engine::utilities::length(&header.display()) as u16
                - retro_engine::utilities::length(&application_name) as u16,
        ),
        Some(
            left_space
                - retro_engine::utilities::length(&header.display()) as u16
                - retro_engine::utilities::length(&application_name) as u16,
        ),
        0,
        false,
    );

    let middle_spacer_length = left_space
        - retro_engine::utilities::length(&audio_controls.display()) as u16
        - retro_engine::utilities::length(&volume_bar.display()) as u16
        - 4;
    control_panel.min_width = Some(middle_spacer_length);
    control_panel.max_width = Some(middle_spacer_length);

    if length(&control_panel.text.default) > control_panel.max_width.unwrap() as usize {
        control_panel.offset += if terminal.polls % 8 == 0 { 1 } else { 0 };
        if control_panel.offset as usize > length(&control_panel.text.default) {
            control_panel.offset = 0
        }
    } else {
        control_panel.offset = 0;
    }

    let control_top = format!(
        "{}{}{}",
        application_name,
        header_spacer.display(),
        header.display()
    );

    let control_middle = format!(
        "{}  {}  {}",
        audio_controls.display(),
        control_panel.display().with(THEME.get().unwrap().primary),
        volume_bar.display()
    );
    // println!(
    //     "{:?} {:?}",
    //     header_spacer.min_width, control_panel.min_width
    // );

    let player_bar_width = (left_space - 14) as u32;
    player_bar.width = player_bar_width;

    let control_bottom =
        retro_engine::scene::align_horizontally(player_bar.display(), timer, "".to_string());

    let control = format!("{}\n{}\n{}", control_top, control_middle, control_bottom);

    view = retro_engine::scene::align_horizontally(view, control, "  ".to_string());
    view
}

pub fn update_song_list(song_list: &mut Grid, player: &PlayerSession) {
    song_list.data[0] = GridItem::StatefulString(StatefulString::from(
        " ".with(THEME.get().unwrap().secondary).to_string()
            + &player
                .peek_previous()
                .unwrap_or((&String::from("None"), &String::from("None")))
                .0
                .clone()
                .with(THEME.get().unwrap().secondary)
                .to_string(),
    ));

    song_list.data[1] = GridItem::StatefulString(StatefulString::from(
        "   ".to_string()
            + &player
                .current()
                .unwrap_or((&String::from("None"), &String::from("None")))
                .0
                .clone()
                .with(THEME.get().unwrap().primary)
                .to_string(),
    ));
    song_list.data[2] = GridItem::StatefulString(StatefulString::from(
        "     ".with(THEME.get().unwrap().secondary).to_string()
            + &player
                .peek_next()
                .unwrap_or((&String::from("None"), &String::from("None")))
                .0
                .clone()
                .with(THEME.get().unwrap().secondary)
                .to_string(),
    ));
}
