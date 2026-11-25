use directories::ProjectDirs;
use discord_presence::Client;
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
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
};
use std::sync::{Arc, LazyLock, Mutex};

mod builders;
mod components;
mod configuration;
mod handlers;
mod logger;
mod player;
mod presence;
mod streaming;
mod theme;
mod views;

use builders::*;
use components::*;
use configuration::*;
use logger::*;
use player::*;
use presence::*;
use views::*;

use crate::{
    handlers::trigger_recommendation_refresh,
    streaming::{session::StreamingEvent, ytdlp},
    views::{control::ControlView, log::LogView, streaming::StreamingView},
};

pub static PROJECT_DIRECTORY: LazyLock<ProjectDirs> = LazyLock::new(|| {
    let project_directory = ProjectDirs::from("is-a.dev", "The-Nice-One", "RetroPlayer");
    project_directory.expect("Application directory not found")
});

fn main() {
    let (mut playlist_paths, restore_last_playlist) = {
        let config = configuration!();
        let mut paths = Vec::<String>::new();
        if !config.songs_directory.is_empty() {
            paths.push(config.songs_directory.clone());
        }
        for dir in &config.song_directories {
            if !dir.is_empty() && !paths.contains(dir) {
                paths.push(dir.clone());
            }
        }
        (paths, config.restore_last_playlist)
    };
    if playlist_paths.is_empty() {
        playlist_paths.push(String::new());
    }

    let mut selected_playlist = 0;
    if restore_last_playlist
        && let Some(previous_path) = load_last_playlist()
        && let Some(previous_index) = playlist_paths.iter().position(|p| *p == previous_path)
    {
        selected_playlist = previous_index;
    }
    let mut player = PlayerSession::default();
    if let Some(path) = playlist_paths.get(selected_playlist) {
        player.load_playlist(path);
        if configuration!().preshuffle {
            player.shuffle();
        }
    }

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
    ytdlp::is_available(&configuration!().streaming.ytdlp_path);

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
                    0,
                    0,
                    0,
                    0,
                    0,
                    std::ptr::null_mut(), // No parent (top-level)
                    std::ptr::null_mut(), // No menu
                    std::ptr::null_mut(), // No instance
                    std::ptr::null(),
                );
                if hwnd == std::ptr::null_mut() {
                    None
                } else {
                    Some(hwnd as _)
                }
            },
            #[cfg(not(target_os = "windows"))]
            hwnd: None,
        };
        let mut controls = MediaControls::new(config).expect("Failed to init media controls");
        controls
            .attach(move |event| {
                let _ = media_tx.send(event);
            })
            .ok();
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

    let (view_group, control_radio, log_radio, streaming_radio) = new_view_group();
    let mut header = new_header(
        view_group,
        control_radio.clone(),
        log_radio.clone(),
        streaming_radio.clone(),
    );
    header.set_state(State::Hovered);
    let mut control_panel = Text::new(
        "ARROW keys to navigate between items, ENTER key to select, and ESC key to go back   ",
        None,
        None,
        0,
        true,
    );

    let log = LogDisplay::from_logger(&LOGGER);

    let mut search = SearchBox::new();
    let mut recommendations =
        new_recommendations_grid(configuration!().streaming.recommendations_per_fetch);
    let mut playlist_dropdown = PlaylistDropdown::new(playlist_paths, selected_playlist);
    let mut streaming_session = streaming::StreamingSession::new();
    let mut streaming_coordinator = streaming::StreamingCoordinator::new();
    if configuration!().streaming.enabled {
        trigger_recommendation_refresh(&mut streaming_session, &player);
    }

    let mut null_component = retro_engine::components::Null::disabled();
    let mut null_component_2 = retro_engine::components::Null::disabled();
    let mut view_registry = ViewRegistry::<AppView>::new();

    view_registry.register(Box::new(ControlView::new()));
    view_registry.register(Box::new(StreamingView::new()));
    view_registry.register(Box::new(LogView::new()));

    let mut running = true;
    let mut is_navigating_previous = false;
    let mut last_topup_refresh_attempt: Option<std::time::Instant> = None;
    while running {
        terminal.poll(100);
        terminal.top();

        if manager.main_track().num_sounds() == 0 && !player.songs.is_empty() {
            if is_navigating_previous {
                is_navigating_previous = false;
            } else {
                let is_auto_stream = player
                    .current()
                    .map(|(_, path)| path.contains("recommendations"))
                    .unwrap_or(false);

                if is_auto_stream {
                    if let Some((removed_name, _)) = player.advance_and_remove_current() {
                        streaming_coordinator.mark_played(&removed_name);
                    }
                } else {
                    player.next();
                }

                if configuration!().streaming.streaming_mode {
                    if streaming_coordinator.on_song_played() {
                        trigger_recommendation_refresh(&mut streaming_session, &player);
                    }
                }
            }

            if let Some((_, path)) = player.current() {
                let sound_data = StaticSoundData::from_file(path).unwrap();
                let duration = sound_data.unsliced_duration();
                player_bar.maximum = duration.as_secs() as u32;
                sound = Some(manager.play(sound_data.clone()).unwrap());
                sound
                    .as_mut()
                    .unwrap()
                    .set_volume(volume_bar.db, kira::Tween::default());

                let song_name = player.current().unwrap().0.clone();
                media_controls
                    .set_metadata(MediaMetadata {
                        title: Some(&song_name),
                        artist: Some("Retro Player"),
                        album: None,
                        duration: Some(std::time::Duration::from_secs(player_bar.maximum as u64)),
                        cover_url: None,
                    })
                    .ok();

                update_song_list(&mut song_list, &player);
                let activity = build_activity(&player);
                update_discord_rpc(&discord_rpc, &mut discord_rpc_handles, activity);
            }
        }

        if let Some(s) = &sound {
            let playback = match s.state() {
                kira::sound::PlaybackState::Playing => MediaPlayback::Playing {
                    progress: Some(MediaPosition(std::time::Duration::from_secs_f64(
                        s.position(),
                    ))),
                },
                kira::sound::PlaybackState::Paused => MediaPlayback::Paused {
                    progress: Some(MediaPosition(std::time::Duration::from_secs_f64(
                        s.position(),
                    ))),
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
                            kira::sound::PlaybackState::Playing => {
                                s.pause(Tween::default());
                            }
                            kira::sound::PlaybackState::Paused => {
                                s.resume(Tween::default());
                            }
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
                    is_navigating_previous = true;
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

        // Handle streaming events
        for event in streaming_session.poll() {
            match event {
                StreamingEvent::RecommendationsReady(tracks) => {
                    let library = crate::streaming::LibraryIndex::from_song_keys(&player.songs);
                    let seed_artists = library.seed_artists(8);
                    streaming_coordinator.add_candidates(tracks.clone(), &seed_artists);

                    if streaming_session.display_mode == crate::streaming::session::StreamingDisplayMode::Recommendations {
                        update_recommendation_labels(&mut recommendations, &tracks);
                    }
                    if let Some(entry) = logger!().get_entry(EntryId::Streaming) {
                        if tracks.is_empty() {
                            entry.fail("Found 0 recommendations. Try adding more songs with 'Artist - Title' filenames.");
                        } else {
                            entry.complete(format!(
                                "Found {} tracks based on your library",
                                tracks.len()
                            ));
                        }
                    }
                }
                StreamingEvent::RecommendationsFailed(err) => {
                    if let Some(entry) = logger!().get_entry(EntryId::Streaming) {
                        entry.fail(format!("Streaming error: {}", err));
                    }
                }
                StreamingEvent::SearchResultsReady(query, tracks) => {
                    if matches!(&streaming_session.display_mode, crate::streaming::session::StreamingDisplayMode::Search { query: q } if q == &query) {
                        update_recommendation_labels(&mut recommendations, &tracks);
                    }
                    if let Some(entry) = logger!().get_entry(EntryId::Streaming) {
                        if tracks.is_empty() {
                            entry.fail(format!("Found 0 results for '{}'", query));
                        } else {
                            entry.complete(format!(
                                "Found {} results for '{}'",
                                tracks.len(),
                                query
                            ));
                        }
                    }
                }
                StreamingEvent::SearchResultsFailed(query, err) => {
                    if let Some(entry) = logger!().get_entry(EntryId::Streaming) {
                        entry.fail(format!("Search failed for '{}': {}", query, err));
                    }
                }
                StreamingEvent::DownloadStarted(key) => {
                    logger!().add_entry(
                        EntryId::Streaming,
                        Entry::new(format!("Downloading: {}", key)),
                    );
                }
                StreamingEvent::DownloadComplete {
                    song_key,
                    path,
                    origin,
                    ..
                } => {
                    let current_playlist = playlist_dropdown.current_path().map(std::path::PathBuf::from);
                    let target_playlist = match &origin {
                        crate::streaming::session::DownloadOrigin::Manual { playlist_dir, .. } => playlist_dir,
                        crate::streaming::session::DownloadOrigin::Streaming { playlist_dir } => playlist_dir,
                    };

                    let limit = configuration!().streaming.recommendations_cache_limit;

                    if current_playlist.as_ref() != Some(target_playlist) {
                        // User switched playlists while this download was in flight.
                        // Keep file on disk in target_playlist cache, but drop from current queue.
                        let rec_dir = target_playlist.join("recommendations");
                        let _ = crate::streaming::record_download(&rec_dir, &format!("{}.mp3", song_key));
                        let _ = crate::streaming::prune_cache(target_playlist, limit, &[]);
                        if matches!(origin, crate::streaming::session::DownloadOrigin::Streaming { .. }) {
                            streaming_coordinator.is_downloading = false;
                        }
                    } else {
                        match origin {
                            crate::streaming::session::DownloadOrigin::Manual {
                                insert_after_key,
                                playlist_dir,
                            } => {
                                player.insert_after(
                                    insert_after_key.as_deref(),
                                    song_key.clone(),
                                    path.to_string_lossy().to_string(),
                                );
                                let active: Vec<&str> =
                                    player.songs.values().map(|s| s.as_str()).collect();
                                let rec_dir = playlist_dir.join("recommendations");
                                let _ = crate::streaming::record_download(&rec_dir, &format!("{}.mp3", song_key));
                                let _ = crate::streaming::prune_cache(&playlist_dir, limit, &active);
                            }
                            crate::streaming::session::DownloadOrigin::Streaming { playlist_dir } => {
                                player.insert_streaming_jitter(
                                    song_key.clone(),
                                    path.to_string_lossy().to_string(),
                                );
                                streaming_coordinator.is_downloading = false;
                                let active: Vec<&str> =
                                    player.songs.values().map(|s| s.as_str()).collect();
                                let rec_dir = playlist_dir.join("recommendations");
                                let _ = crate::streaming::record_download(&rec_dir, &format!("{}.mp3", song_key));
                                let _ = crate::streaming::prune_cache(&playlist_dir, limit, &active);
                            }
                        }
                        update_song_list(&mut song_list, &player);
                    }
                    if let Some(entry) = logger!().get_entry(EntryId::Streaming) {
                        entry.complete(format!("Download finished: {}", song_key));
                    }
                }
                StreamingEvent::DownloadFailed(key, err) => {
                    streaming_coordinator.is_downloading = false;
                    if let Some(entry) = logger!().get_entry(EntryId::Streaming) {
                        entry.fail(format!("Failed to download {}: {}", key, err));
                    }
                }
            }
        }

        // Streaming mode queue buffer maintenance
        if configuration!().streaming.enabled && configuration!().streaming.streaming_mode {
            if !streaming_coordinator.is_downloading && !streaming_session.refreshing {
                let lookahead_end = (player.current_index + 5).min(player.songs.len());
                let start = (player.current_index + 1).min(player.songs.len());
                let mut auto_stream_count = 0;
                for i in start..lookahead_end {
                    if let Some((_, path)) = player.songs.get_index(i) {
                        if path.contains("recommendations") {
                            auto_stream_count += 1;
                        }
                    }
                }

                if auto_stream_count < 2 {
                    if let Some(candidate) = streaming_coordinator.pick_candidate(|name| player.songs.contains_key(name)) {
                        streaming_coordinator.is_downloading = true;
                        let config = configuration!();
                        let base_dir = if let Some(current_playlist_path) = playlist_dropdown.current_path() {
                            std::path::PathBuf::from(current_playlist_path)
                        } else if !config.songs_directory.is_empty() {
                            std::path::PathBuf::from(&config.songs_directory)
                        } else {
                            std::path::PathBuf::from(".")
                        };
                        let download_dir = base_dir.join("recommendations");
                        let ytdlp_path = config.streaming.ytdlp_path.clone();
                        let lyrics_config = config.lyrics.clone();
                        drop(config);

                        let playlist_dir = base_dir.clone();
                        streaming_session.start_download(
                            candidate,
                            download_dir,
                            ytdlp_path,
                            lyrics_config,
                            crate::streaming::session::DownloadOrigin::Streaming { playlist_dir },
                        );
                    } else if streaming_coordinator.candidate_pool.len() < 3 && !streaming_session.refreshing {
                        let can_refresh = last_topup_refresh_attempt.map_or(true, |t| t.elapsed() >= std::time::Duration::from_secs(60));
                        let has_artists = crate::streaming::LibraryIndex::from_song_keys(&player.songs).artist_count() > 0;
                        if can_refresh && has_artists {
                            last_topup_refresh_attempt = Some(std::time::Instant::now());
                            trigger_recommendation_refresh(&mut streaming_session, &player);
                        }
                    }
                }
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
                null_component_2: &mut null_component_2,
                player_bar: &mut player_bar,
                audio_controls: &mut audio_controls,
                volume_bar: &mut volume_bar,
                sound: &mut sound,
                player: &mut player,
                is_navigating_previous: &mut is_navigating_previous,
                log: &log,
                search: &mut search,
                recommendations: &mut recommendations,
                playlist_dropdown: &mut playlist_dropdown,
                streaming: &mut streaming_session,
                streaming_coordinator: &mut streaming_coordinator,
            };

            if terminal.event.is_some() {
                let event = terminal.event.as_ref().unwrap().to_owned();

                let components = view.components(&mut view_state);
                view.controller().feed(&event, components);

                let is_release = matches!(
                    &event,
                    retro_engine::Event::Key(key_event)
                        if key_event.kind == retro_engine::KeyEventKind::Release
                );
                if !is_release {
                    view.handle_event(&event, &mut view_state);
                }
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
