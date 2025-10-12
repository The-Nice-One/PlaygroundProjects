use discord_presence::{
    models::{Activity, ActivityType},
    Client,
};
use kira::{
    sound::static_sound::StaticSoundData, sound::static_sound::StaticSoundHandle, AudioManager,
    AudioManagerSettings, DefaultBackend, Tween,
};
use retro_engine::components::trait_def::Component;
use retro_engine::components::*;
use retro_engine::core::Terminal;
use retro_engine::feeders::trait_def::Feeder;
use retro_engine::Stylize;
use std::sync::{Arc, Mutex};
use toml::from_slice;

mod builders;
mod components;
mod configuration;
mod handlers;
mod player;
mod presence;
mod theme;

use builders::*;
use components::*;
use configuration::*;
use handlers::*;
use player::*;
use presence::*;
use theme::*;

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

    let mut discord_rpc_handles = vec![];
    let discord_rpc = if configuration.discord_presence {
        Arc::new(Mutex::new(Some(Client::new(1421950568858910758))))
    } else {
        Arc::new(Mutex::new(None))
    };
    start_discord_rpc(&discord_rpc, &mut discord_rpc_handles);

    init_theme(
        configuration.theme.primary,
        configuration.theme.secondary,
        configuration.theme.accent,
    );

    let mut terminal = Terminal::init();
    terminal.hide_cursor();

    let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();
    let mut sound: Option<StaticSoundHandle> = None;

    let mut side_bar = VerticalLine::default();
    side_bar
        .start("┌".with(THEME.get().unwrap().accent))
        .middle("|".with(THEME.get().unwrap().accent))
        .end("└".with(THEME.get().unwrap().accent))
        .height(3);

    let mut song_list = Grid::new((1, 3));
    let mut audio_controls = new_audio_controls();
    let mut volume_bar = VolumeBar::new();
    let mut player_bar = new_player_bar();

    let mut header = new_header();
    header.set_state(State::Hovered);
    let mut control_panel = Text::new(
        "ARROW keys to navigate between items, ENTER key to select, and ESC key to go back   "
            .with(THEME.get().unwrap().primary),
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
            let activity = Activity::new()
                .activity_type(ActivityType::Listening)
                .state(
                    player
                        .current()
                        .unwrap_or((&String::from("None"), &String::from("None")))
                        .0
                        .clone(),
                );
            update_discord_rpc(&discord_rpc, &mut discord_rpc_handles, activity);
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
                control_panel
                    .text
                    .default("Volume Controls - Adjust   ".with(THEME.get().unwrap().primary));
                control_panel.offset = 0;
            }
            if volume_bar.get_state().unwrap_or(State::Disabled) == State::Active {
                control_panel.text.default(
                    "Adjust Volume - UP or DOWN arrow keys   ".with(THEME.get().unwrap().primary),
                );
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

    for handle in discord_rpc_handles {
        handle.join().unwrap();
    }

    if let Ok(client) = Arc::try_unwrap(discord_rpc) {
        let mut client = client.into_inner().unwrap();
        if let Some(client) = client.take() {
            client.shutdown().unwrap();
        }
    }
}
