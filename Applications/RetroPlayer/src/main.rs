use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, Tween,
    sound::static_sound::StaticSoundData, sound::static_sound::StaticSoundHandle,
};
use retro_engine::Stylize;
use retro_engine::components::trait_def::Component;
use retro_engine::components::*;
use retro_engine::core::Terminal;
use retro_engine::feeders::trait_def::Feeder;
use serde::{Deserialize, Serialize};
use std::{env::current_exe, fs::read, fs::write};
use toml::from_slice;

mod builders;
mod components;
mod handlers;
mod player;

use builders::*;
use components::*;
use handlers::*;
use player::*;

#[derive(Serialize, Deserialize)]
struct Configuration {
    songs_directory: String,
}

fn get_configuration_file() -> Option<(String, Vec<u8>)> {
    let exe_path = current_exe();
    if exe_path.is_err() {
        println!(
            "{} Could not find executable path. Read permission may be denied.",
            "[ Error: ".red()
        );
        return None;
    }

    if let Some(exe_dir) = exe_path.unwrap().parent() {
        let configuration_path = exe_dir
            .join("Configuration.toml")
            .to_str()
            .unwrap()
            .to_string();

        let file = read(&configuration_path);
        if file.is_err() {
            println!(
                "{} Configuration file not found in executable directory. Attempting to create one...",
                "[ Error: ".red()
            );
            let write_result = write(
                &configuration_path,
                "# Inside the \"\" (quotes) type the path to your song directory.\nsongs_directory = \"\"",
            );
            if write_result.is_err() {
                println!(
                    "{} Could not write configuration file. Write permission may be denied.",
                    "[ Error: ".red()
                );
            } else {
                println!(
                    "{} Configuration file created successfully. You can edit it at {}",
                    "[ Success: ".green(),
                    &configuration_path
                );
            }
            return None;
        }
        return Some((configuration_path, file.unwrap()));
    }
    None
}

fn main() {
    let configuration_file = get_configuration_file();
    if configuration_file.is_none() {
        return;
    }
    let configuration_path = configuration_file.clone().unwrap().0;
    let configuration_file = configuration_file.unwrap().1.clone();
    let configuration: Configuration = from_slice(&configuration_file).unwrap();

    let mut player = PlayerSession::default();
    player.add_songs(&configuration.songs_directory);

    if player.songs.is_empty() {
        println!(
            "{} No songs found in {}. Please add some songs to the directory.",
            "[ Error: ".red(),
            &configuration.songs_directory
        );
        return;
    }

    let mut terminal = Terminal::init();
    terminal.hide_cursor();

    let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();
    let mut sound: Option<StaticSoundHandle> = None;

    let mut side_bar = VerticalLine::default();
    side_bar
        .start("┌".blue())
        .middle("|".blue())
        .end("└".blue())
        .height(3);

    let mut song_list = Grid::new((1, 3));
    let mut audio_controls = new_audio_controls();
    let mut volume_bar = VolumeBar::new();
    let mut player_bar = new_player_bar();

    let mut header = new_header();
    header.set_state(State::Hovered);
    let mut control_panel = Text::new(
        "ARROW keys to navigate between items, ENTER key to select, and ESC key to go back   ",
        None,
        None,
        0,
        true,
    );

    let mut controller = retro_engine::feeders::GridFeeder::new((2, 2));
    controller.hovered = (1, 0);

    let mut running = true;
    while running {
        terminal.poll(50);
        terminal.top();

        if manager.main_track().num_sounds() == 0 {
            player.next();

            let sound_data = StaticSoundData::from_file(player.current().unwrap().1).unwrap();
            let duration = sound_data.unsliced_duration();
            player_bar.maximum = duration.as_secs() as u32;
            sound = Some(manager.play(sound_data.clone()).unwrap());

            update_song_list(&mut song_list, &player);
        }

        if terminal.event.is_some() {
            let event = terminal.event.as_ref().unwrap().to_owned();
            controller.feed(
                &event,
                vec![
                    Box::new(&mut retro_engine::components::Null::disabled()),
                    Box::new(&mut header),
                    Box::new(&mut audio_controls),
                    Box::new(&mut volume_bar),
                ],
            );

            handle_header_controls(
                &mut header,
                &mut control_panel,
                &configuration_path,
                &mut running,
            );
            handle_audio_controls(
                &audio_controls,
                &mut player,
                &mut sound,
                &mut player_bar,
                &mut song_list,
                &mut control_panel,
            );

            if volume_bar.get_state().unwrap_or(State::Disabled) == State::Hovered {
                control_panel.text.default("Volume Controls - Adjust   ");
                control_panel.offset = 0;
            }
            if volume_bar.get_state().unwrap_or(State::Disabled) == State::Active {
                control_panel
                    .text
                    .default("Adjust Volume - UP or DOWN arrow keys   ");
                control_panel.offset = 0;
            }

            sound
                .as_mut()
                .unwrap()
                .set_volume(volume_bar.db, Tween::default());
        }

        let view = update_view(
            &mut player_bar,
            &side_bar,
            &song_list,
            &header,
            &mut control_panel,
            &audio_controls,
            &volume_bar,
            &terminal,
            &sound,
        );

        terminal.print(&view);
    }

    terminal.deinit();
    sound.as_mut().unwrap().stop(Tween::default());
}
