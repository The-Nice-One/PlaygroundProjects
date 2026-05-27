use directories::ProjectDirs;
use discord_presence::{
    Client,
    models::{Activity, ActivityType},
};
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, Tween,
    sound::static_sound::StaticSoundData, sound::static_sound::StaticSoundHandle,
};
use retro_engine::Stylize;
use retro_engine::components::trait_def::Component;
use retro_engine::components::*;
use retro_engine::core::Terminal;
use retro_engine::feeders::trait_def::Feeder;
use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition,
    PlatformConfig,
};
use std::sync::{Arc, LazyLock, Mutex};

mod builders;
mod components;
mod configuration;
mod handlers;
mod logger;
mod player;
mod presence;
mod theme;
mod views;

use builders::*;
use components::*;
use configuration::*;
use logger::*;
use player::*;
use presence::*;
use views::*;

pub static PROJECT_DIRECTORY: LazyLock<ProjectDirs> = LazyLock::new(|| {
    let project_directory = ProjectDirs::from("is-a.dev", "The-Nice-One", "RetroPlayer");
    project_directory.expect("Application directory not found")
});

fn main() {
    let mut player = PlayerSession::default();
    player.add_songs(&configuration!().songs_directory);

    let mut discord_rpc_handles = vec![];
    let discord_rpc = if configuration!().discord_presence {
        LOGGER.lock().unwrap().add_entry(
            EntryId::DiscordPresence,
            Entry::new(String::from("Starting discord presence")),
        );
        Arc::new(Mutex::new(Some(Client::new(1421950568858910758))))
    } else {
        Arc::new(Mutex::new(None))
    };
    start_discord_rpc(&discord_rpc, &mut discord_rpc_handles);

    let (media_tx, media_rx) = std::sync::mpsc::channel();
    let mut media_controls = {
        let config = PlatformConfig {
            dbus_name: "retro_player",
            display_name: "Retro Player",
            #[cfg(target_os = "windows")]
            hwnd: unsafe {
                use windows_sys::Win32::UI::WindowsAndMessaging::*;
                // Create a hidden window to host SMTC
                let hwnd = CreateWindowExA(
                    0,
                    "STATIC\0".as_ptr(),
                    "RetroPlayerMediaHost\0".as_ptr() as *const u8, // Explicit cast for clarity
                    0, 0, 0, 0, 0,
                    std::ptr::null_mut(), // No parent (top-level)
                    std::ptr::null_mut(), // No menu
                    std::ptr::null_mut(), // No instance
                    std::ptr::null()
                );
                if hwnd == std::ptr::null_mut() { None } else { Some(hwnd as _) }
            },
            #[cfg(not(target_os = "windows"))]
            hwnd: None,
        };
        let mut controls = MediaControls::new(config).expect("Failed to init media controls");
        controls.attach(move |event| { let _ = media_tx.send(event); }).ok();
        controls
    };

    let mut terminal = Terminal::init();
    terminal.hide_cursor();
    terminal.configuration.overwrite_lines = true;

    let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();
    let mut sound: Option<StaticSoundHandle> = None;

    let mut side_bar = VerticalLine::default();
    side_bar
        .start("┌".with(theme!().accent))
        .middle("|".with(theme!().accent))
        .end("└".with(theme!().accent))
        .height(3);

    let mut song_list = Grid::new((1, 3));
    let mut audio_controls = new_audio_controls();
    let mut volume_bar = VolumeBar::new();
    let mut player_bar = new_player_bar();

    let (view_group, control_radio, log_radio) = new_view_group();
    let mut header = new_header(view_group, control_radio.clone(), log_radio.clone());
    header.set_state(State::Hovered);
    let mut control_panel = Text::new(
        "ARROW keys to navigate between items, ENTER key to select, and ESC key to go back   ",
        None,
        None,
        0,
        true,
    );

    let log = LogDisplay::from_logger(&LOGGER);

    let mut null_component = retro_engine::components::Null::disabled();
    let mut view_registry = ViewRegistry::<AppView>::new();

    view_registry.register(Box::new(ControlView::new()));
    view_registry.register(Box::new(LogView::new()));

    let mut running = true;
    while running {
        terminal.poll(50);
        terminal.top();

        if manager.main_track().num_sounds() == 0 && !player.songs.is_empty() {
            player.next();

            let sound_data = StaticSoundData::from_file(player.current().unwrap().1).unwrap();
            let duration = sound_data.unsliced_duration();
            player_bar.maximum = duration.as_secs() as u32;
            sound = Some(manager.play(sound_data.clone()).unwrap());

                let song_name = player.current().unwrap().0.clone();
                media_controls.set_metadata(MediaMetadata {
                    title: Some(&song_name),
                    artist: Some("Retro Player"),
                    album: None,
                    duration: Some(std::time::Duration::from_secs(player_bar.maximum as u64)),
                    cover_url: None,
                }).ok();

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

        if let Some(s) = &sound {
            let playback = match s.state() {
                kira::sound::PlaybackState::Playing => MediaPlayback::Playing { 
                    progress: Some(MediaPosition(std::time::Duration::from_secs_f64(s.position()))) 
                },
                kira::sound::PlaybackState::Paused => MediaPlayback::Paused { 
                    progress: Some(MediaPosition(std::time::Duration::from_secs_f64(s.position()))) 
                },
                _ => MediaPlayback::Stopped,
            };
            media_controls.set_playback(playback).ok();

            if let GridItem::Toggle(toggle) = &mut audio_controls.data[2] {
                toggle.is_on = s.state() == kira::sound::PlaybackState::Playing;
            }
        }

        while let Ok(event) = media_rx.try_recv() {
            match event {
                MediaControlEvent::Play | MediaControlEvent::Pause | MediaControlEvent::Toggle => {
                    if let Some(s) = sound.as_mut() {
                        match s.state() {
                            kira::sound::PlaybackState::Playing => { s.pause(Tween::default()); }
                            kira::sound::PlaybackState::Paused => { s.resume(Tween::default()); }
                            _ => {}
                        }
                    }
                }
                MediaControlEvent::Next => {
                    if let Some(s) = sound.as_mut() {
                        s.stop(Tween::default());
                    }
                }
                MediaControlEvent::Previous => {
                    player.previous();
                    player.previous();
                    if let Some(s) = sound.as_mut() {
                        s.stop(Tween::default());
                    }
                }
                _ => {}
            }
        }

        if let Some(radio) = header.get_active_radio(view_group) {
            if let Some(id) = radio.id {
                view_registry.switch_to(id);
            }
        }
        if let Some(view) = view_registry.current_view() {
            let mut view_state = ViewState {
                header: &mut header,
                control_panel: &mut control_panel,
                side_bar: &side_bar,
                song_list: &mut song_list,
                terminal: &terminal,
                running: &mut running,
                null_component: &mut null_component,
                player_bar: &mut player_bar,
                audio_controls: &mut audio_controls,
                volume_bar: &mut volume_bar,
                sound: &mut sound,
                player: &mut player,
                log: &log,
            };

            if terminal.event.is_some() {
                let event = terminal.event.as_ref().unwrap().to_owned();

                let components = view.components(&mut view_state);
                view.controller().feed(&event, components);
                view.handle_event(&event, &mut view_state);
            }

            let rendered = view.render(&mut view_state);
            terminal.print(&rendered);
        }
    }

    terminal.deinit();
    if let Some(mut s) = sound {
        s.stop(Tween::default());
    }

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
