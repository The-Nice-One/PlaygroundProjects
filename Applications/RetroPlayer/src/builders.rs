use crate::PlayerSession;
use crate::components::LogDisplay;
use crate::components::PlaylistDropdown;
use crate::components::VolumeBar;
use crate::theme;
use kira::sound::static_sound::StaticSoundHandle;
use retro_engine::Stylize;
use retro_engine::components::*;
use retro_engine::core::Terminal;
use retro_engine::utilities::length;

pub struct HeaderButton;
impl HeaderButton {
    pub const CONTROL: usize = 0;
    pub const STREAMING: usize = 1;
    pub const LOG: usize = 2;
    pub const SETTINGS: usize = 3;
    pub const EXIT: usize = 4;
}

pub struct AudioControlButton;
impl AudioControlButton {
    pub const SHUFFLE: usize = 0;
    pub const PREVIOUS: usize = 1;
    pub const PLAY_PAUSE: usize = 2;
    pub const NEXT: usize = 3;
    pub const RESTART: usize = 4;
}

pub fn new_view_group() -> (RadioGroup, Radio, Radio, Radio) {
    let view_group = RadioGroup::from(0);
    let mut control_radio = Radio::new(
        StatefulString::from("♪".with(theme!().accent).bold())
            .hovered("♪".with(theme!().accent).bold())
            .active("♪".with(theme!().accent).bold())
            .into(),
        StatefulString::from("♪".with(theme!().primary))
            .hovered("♪".with(theme!().accent))
            .active("♪".with(theme!().accent))
            .into(),
        true,
    );
    control_radio.id = Some(HeaderButton::CONTROL);

    let mut stream_radio = Radio::new(
        StatefulString::from("⇊".with(theme!().accent).bold())
            .hovered("⇊".with(theme!().accent).bold())
            .active("⇊".with(theme!().accent).bold())
            .into(),
        StatefulString::from("⇊".with(theme!().primary))
            .hovered("⇊".with(theme!().accent))
            .active("⇊".with(theme!().accent))
            .into(),
        false,
    );
    stream_radio.id = Some(HeaderButton::STREAMING);

    let mut log_radio = Radio::new(
        StatefulString::from("☰".with(theme!().accent).bold())
            .hovered("☰".with(theme!().accent).bold())
            .active("☰".with(theme!().accent).bold())
            .into(),
        StatefulString::from("☰".with(theme!().primary))
            .hovered("☰".with(theme!().accent))
            .active("☰".with(theme!().accent))
            .into(),
        false,
    );
    log_radio.id = Some(HeaderButton::LOG);

    (view_group, control_radio, log_radio, stream_radio)
}

pub fn new_header(
    view_group: RadioGroup,
    control_radio: Radio,
    log_radio: Radio,
    stream_radio: Radio,
) -> Grid {
    let mut header = Grid::new((5, 1));
    // ♪ ☰ ⇊ ⋮ x

    header.data[HeaderButton::CONTROL] = GridItem::Radio(control_radio, view_group);
    header.data[HeaderButton::STREAMING] = GridItem::Radio(stream_radio, view_group);
    header.data[HeaderButton::LOG] = GridItem::Radio(log_radio, view_group);

    header.data[HeaderButton::SETTINGS] = GridItem::Button(Button::from(
        StatefulString::from("⋮".with(theme!().primary))
            .hovered("⋮".with(theme!().accent))
            .active("⋮".with(theme!().accent).bold()),
    ));
    header.data[HeaderButton::EXIT] = GridItem::Button(Button::from(
        StatefulString::from("x".with(theme!().primary))
            .hovered("x".with(theme!().accent))
            .active("x".with(theme!().accent).bold()),
    ));

    header.left_spacer = StatefulString::from("[  ".with(theme!().primary))
        .hovered("[  ".with(theme!().accent))
        .active("[  ".with(theme!().accent).bold())
        .clone();
    header.right_spacer = StatefulString::from("  ]".with(theme!().primary))
        .hovered("  ]".with(theme!().accent))
        .active("  ]".with(theme!().accent).bold())
        .clone();
    header.horizontal_spacer = "  ".into();
    header
}

pub fn new_audio_controls() -> retro_engine::components::Grid {
    let mut audio_controls = Grid::new((5, 1));
    audio_controls.data[AudioControlButton::SHUFFLE] = GridItem::Button(Button::from(
        StatefulString::from("⇆".with(theme!().primary))
            .hovered("⇆".with(theme!().accent))
            .active("⇆".with(theme!().accent).bold()),
    ));
    audio_controls.data[AudioControlButton::PREVIOUS] = GridItem::Button(Button::from(
        StatefulString::from("⭰".with(theme!().primary))
            .hovered("⭰".with(theme!().accent))
            .active("⭰".with(theme!().accent).bold()),
    ));
    audio_controls.data[AudioControlButton::PLAY_PAUSE] = GridItem::Toggle(Toggle::new(
        StatefulString::from("⏵".with(theme!().primary))
            .hovered("⏵".with(theme!().accent))
            .active("⏵".with(theme!().accent).bold())
            .into(),
        StatefulString::from("⏸".with(theme!().primary))
            .hovered("Ⅱ".with(theme!().accent))
            .active("Ⅱ".with(theme!().accent).bold())
            .into(),
        true,
    ));
    audio_controls.data[AudioControlButton::NEXT] = GridItem::Button(Button::from(
        StatefulString::from("⭲".with(theme!().primary))
            .hovered("⭲".with(theme!().accent))
            .active("⭲".with(theme!().accent).bold()),
    ));
    audio_controls.data[AudioControlButton::RESTART] = GridItem::Button(Button::from(
        StatefulString::from("⮌".with(theme!().primary))
            .hovered("⮌".with(theme!().accent))
            .active("⮌".with(theme!().accent).bold()),
    ));
    // audio_controls.data[2] = StatefulString::from("⏵Ⅱ⏸‖")
    //     .hovered("⏵".with(theme!().accent))
    //     .active("⏵".with(theme!().accent).bold())
    //     .into();

    audio_controls.left_spacer = StatefulString::from("[  ".with(theme!().primary))
        .hovered("[  ".with(theme!().accent))
        .active("[  ".with(theme!().accent).bold())
        .clone();
    audio_controls.right_spacer = StatefulString::from("  ]".with(theme!().primary))
        .hovered("  ]".with(theme!().accent))
        .active("  ]".with(theme!().accent).bold())
        .clone();
    audio_controls.horizontal_spacer = "  ".into();
    audio_controls
}

fn track_button(label: &str) -> Button {
    Button::from(
        StatefulString::from(label.to_string().with(theme!().primary))
            .hovered(label.to_string().with(theme!().accent))
            .active(label.to_string().with(theme!().accent).bold()),
    )
}

fn loading_button() -> Button {
    track_button("Loading...")
}

fn refresh_button() -> Button {
    track_button("\u{27f3} Refresh recommendations")
}

pub fn new_recommendations_grid(track_count: usize) -> Grid {
    let mut grid = Grid::new((1, track_count as u32 + 1));
    for i in 0..track_count {
        grid.data[i] = GridItem::Button(loading_button());
    }
    grid.data[track_count] = GridItem::Button(refresh_button());

    grid.top_left_spacer = Some(
        StatefulString::from("┌  ".with(theme!().primary))
            .hovered("┌  ".with(theme!().accent))
            .active("┌  ".with(theme!().accent).bold())
            .clone(),
    );
    grid.left_spacer = StatefulString::from("|  ".with(theme!().primary))
        .hovered("|  ".with(theme!().accent))
        .active("|  ".with(theme!().accent).bold())
        .clone();
    grid.bottom_left_spacer = Some(
        StatefulString::from("└  ".with(theme!().primary))
            .hovered("└  ".with(theme!().accent))
            .active("└  ".with(theme!().accent).bold())
            .clone(),
    );
    grid.top_right_spacer = Some(
        StatefulString::from("  ┐".with(theme!().primary))
            .hovered("  ┐".with(theme!().accent))
            .active("  ┐".with(theme!().accent).bold())
            .clone(),
    );
    grid.right_spacer = StatefulString::from("  |".with(theme!().primary))
        .hovered("  |".with(theme!().accent))
        .active("  |".with(theme!().accent).bold())
        .clone();
    grid.bottom_right_spacer = Some(
        StatefulString::from("  ┘".with(theme!().primary))
            .hovered("  ┘".with(theme!().accent))
            .active("  ┘".with(theme!().accent).bold())
            .clone(),
    );
    grid.padded_cells = true;
    grid
}

pub fn update_recommendation_labels(
    recommendations: &mut Grid,
    tracks: &[crate::streaming::RecommendedTrack],
) {
    let last_index = recommendations.data.len() - 1;

    if tracks.is_empty() {
        recommendations.data[0] = GridItem::Button(track_button("No results found."));
        for i in 1..last_index {
            recommendations.data[i] = GridItem::Button(track_button("N/A"));
        }
        return;
    }

    for i in 0..last_index {
        let label = match tracks.get(i) {
            Some(track) => format!(
                "⤓ {} - {} ← {}",
                track.artists.join(", "),
                track.title,
                track.source_artist
            ),
            None => "N/A".to_string(),
        };
        recommendations.data[i] = GridItem::Button(track_button(&label));
    }
}

pub fn set_recommendations_loading(recommendations: &mut Grid) {
    let last_index = recommendations.data.len() - 1;
    for i in 0..last_index {
        recommendations.data[i] = GridItem::Button(loading_button());
    }
}

pub fn new_player_bar() -> retro_engine::components::ProgressBar {
    let mut player_bar = ProgressBar::default();
    player_bar.left("━".with(theme!().accent));
    player_bar.pointer(vec!["╺".with(theme!().primary), "╸".with(theme!().accent)]);
    player_bar.right("━".with(theme!().primary));

    player_bar.minimum = 0;
    player_bar.maximum = 500;
    player_bar.value = 0;

    player_bar.width = 100;
    player_bar
}

pub fn calculate_left_space(
    song_list: &Grid,
    playlist_dropdown: &PlaylistDropdown,
    terminal: &Terminal,
) -> u16 {
    let mut sidebar_strings = vec![];
    for song in song_list.data.iter() {
        if let GridItem::StatefulString(song_name) = song {
            sidebar_strings.push(String::from("[ ") + song_name.default.as_str() + "  ");
        }
    }
    sidebar_strings.push(playlist_dropdown.display() + "  ");

    let sidebar_width = retro_engine::utilities::max_length(&sidebar_strings);
    terminal.screen.width - sidebar_width as u16
}

pub fn update_control_panel(control_panel: &mut Text, terminal: &Terminal) {
    if length(&control_panel.text.default) > control_panel.max_width.unwrap() as usize {
        control_panel.offset += if terminal.polls % 8 == 0 { 1 } else { 0 };
        if control_panel.offset as usize > length(&control_panel.text.default) {
            control_panel.offset = 0
        }
    } else {
        control_panel.offset = 0;
    }
}

pub fn update_sidebar(
    side_bar: &VerticalLine,
    song_list: &Grid,
    playlist_dropdown: &PlaylistDropdown,
) -> String {
    let song_list_display = retro_engine::scene::align_horizontally(
        side_bar.display(),
        song_list.display(),
        "".to_string(),
    );
    format!("{}\n{}", playlist_dropdown.display(), song_list_display)
}

pub fn update_header(title: &str, left_space: u16, header: &Grid) -> String {
    let application_name = title.with(theme!().accent).to_string();
    let header_spacer = Text::new(
        "",
        Some(left_space - length(&header.display()) as u16 - length(&application_name) as u16),
        Some(left_space - length(&header.display()) as u16 - length(&application_name) as u16),
        0,
        false,
    );

    format!(
        "{}{}{}",
        application_name,
        header_spacer.display(),
        header.display()
    )
}

pub fn update_control_view(
    player_bar: &mut ProgressBar,
    side_bar: &VerticalLine,
    song_list: &Grid,
    header: &Grid,
    control_panel: &mut Text,
    audio_controls: &Grid,
    volume_bar: &VolumeBar,
    playlist_dropdown: &PlaylistDropdown,
    terminal: &Terminal,
    sound: &Option<StaticSoundHandle>,
) -> String {
    let left_space = calculate_left_space(song_list, playlist_dropdown, terminal);

    let mut view = update_sidebar(side_bar, song_list, playlist_dropdown);
    let control_top = update_header("Retro Player - Control View ♪", left_space, header);

    player_bar.value = if let Some(sound) = sound.as_ref() {
        sound.position() as u32
    } else {
        0
    };
    let timer = format!(
        "{:02}:{:02} - {:02}:{:02}",
        player_bar.value / 60,
        player_bar.value % 60,
        player_bar.maximum / 60,
        player_bar.maximum % 60
    )
    .with(theme!().primary)
    .to_string();

    let middle_spacer_length = left_space
        - retro_engine::utilities::length(&audio_controls.display()) as u16
        - retro_engine::utilities::length(&volume_bar.display()) as u16
        - 4;
    control_panel.min_width = Some(middle_spacer_length);
    control_panel.max_width = Some(middle_spacer_length);
    update_control_panel(control_panel, terminal);

    let control_middle = format!(
        "{}  {}  {}",
        audio_controls.display(),
        control_panel.display().with(theme!().primary),
        volume_bar.display()
    );

    let player_bar_width = (left_space - 14) as u32;
    player_bar.width = player_bar_width;

    let control_bottom =
        retro_engine::scene::align_horizontally(player_bar.display(), timer, " ".to_string());

    let control = format!("{}\n{}\n{}", control_top, control_middle, control_bottom);

    view = retro_engine::scene::align_horizontally(view, control, "  ".to_string());
    view
}

pub fn update_log_view(
    side_bar: &VerticalLine,
    song_list: &Grid,
    header: &Grid,
    control_panel: &mut Text,
    log: &LogDisplay,
    playlist_dropdown: &PlaylistDropdown,
    terminal: &Terminal,
) -> String {
    let left_space = calculate_left_space(song_list, playlist_dropdown, terminal);

    let mut view = update_sidebar(side_bar, song_list, playlist_dropdown);
    // subtract 1 because glyph width is 2 characters for log icon in some terminal emulators.
    let control_top = update_header("Retro Player - Log View ☰", left_space - 1, header);

    control_panel.min_width = Some(left_space);
    control_panel.max_width = Some(left_space);

    if length(&control_panel.text.default) > control_panel.max_width.unwrap() as usize {
        control_panel.offset += if terminal.polls % 8 == 0 { 1 } else { 0 };
        if control_panel.offset as usize > length(&control_panel.text.default) {
            control_panel.offset = 0
        }
    } else {
        control_panel.offset = 0;
    }

    let control_middle = format!("{}", control_panel.display().with(theme!().primary));
    let control_bottom = format!("{}", log.display());

    let control = format!("{}\n{}\n{}", control_top, control_middle, control_bottom);

    view = retro_engine::scene::align_horizontally(view, control, "  ".to_string());
    view
}

pub fn update_streaming_view(
    side_bar: &VerticalLine,
    song_list: &Grid,
    header: &Grid,
    control_panel: &mut Text,
    recommendations: &mut Grid,
    playlist_dropdown: &PlaylistDropdown,
    terminal: &retro_engine::core::Terminal,
) -> String {
    let left_space = calculate_left_space(song_list, playlist_dropdown, terminal);

    let mut view = update_sidebar(side_bar, song_list, playlist_dropdown);
    let control_top = update_header("Retro Player - Streaming View ⇊", left_space, header);

    control_panel.min_width = Some(left_space);
    control_panel.max_width = Some(left_space);

    if retro_engine::utilities::length(&control_panel.text.default)
        > control_panel.max_width.unwrap() as usize
    {
        control_panel.offset += if terminal.polls % 8 == 0 { 1 } else { 0 };
        if control_panel.offset as usize
            > retro_engine::utilities::length(&control_panel.text.default)
        {
            control_panel.offset = 0
        }
    } else {
        control_panel.offset = 0;
    }

    let refresh_string = String::from("\u{27f3} Refresh recommendations");
    let refresh_string =
        refresh_string.clone() + &" ".repeat(left_space as usize - length(&refresh_string) - 6);
    recommendations.update_cell_stateful_string(
        recommendations.data.len() - 1,
        StatefulString::from(refresh_string.to_string().with(theme!().primary))
            .hovered(refresh_string.to_string().with(theme!().accent))
            .active(refresh_string.to_string().with(theme!().accent).bold())
            .clone(),
    );

    let control_middle = format!("{}", control_panel.display().with(theme!().primary));
    let control_bottom = format!("{}", recommendations.display());

    let control = format!("{}\n{}\n{}", control_top, control_middle, control_bottom);

    view = retro_engine::scene::align_horizontally(view, control, "  ".to_string());
    view
}

pub fn update_song_list(song_list: &mut Grid, player: &PlayerSession) {
    song_list.data[0] = GridItem::StatefulString(StatefulString::from(
        " ".with(theme!().secondary).to_string()
            + &player
                .peek_previous()
                .unwrap_or((&String::from("None"), &String::from("None")))
                .0
                .clone()
                .with(theme!().secondary)
                .to_string(),
    ));

    song_list.data[1] = GridItem::StatefulString(StatefulString::from(
        "   ".to_string()
            + &player
                .current()
                .unwrap_or((&String::from("None"), &String::from("None")))
                .0
                .clone()
                .with(theme!().primary)
                .to_string(),
    ));
    song_list.data[2] = GridItem::StatefulString(StatefulString::from(
        "     ".with(theme!().secondary).to_string()
            + &player
                .peek_next()
                .unwrap_or((&String::from("None"), &String::from("None")))
                .0
                .clone()
                .with(theme!().secondary)
                .to_string(),
    ));
}
