use std::path::PathBuf;

use crate::PROJECT_DIRECTORY;
use crate::builders::{
    AudioControlButton, HeaderButton, set_recommendations_loading, update_song_list,
};
use crate::components::PlaylistDropdown;
use crate::logger::{Entry, EntryId};
use crate::streaming::{LibraryIndex, StreamingConfigSnapshot, StreamingSession};
use crate::{PlayerSession, logger};
use crate::{configuration, configuration::*};
use kira::Tween;
use kira::sound::PlaybackState;
use kira::sound::static_sound::StaticSoundHandle;
use log::warn;
use retro_engine::components::*;

pub fn handle_header_controls(header: &mut Grid, control_panel: &mut Text, running: &mut bool) {
    if header.get_state().unwrap_or(State::Disabled) == State::Hovered {
        control_panel
            .text
            .default("Header Controls - Views, Settings, Exit");
        control_panel.offset = 0;
    }

    if header.get_state().unwrap_or(State::Disabled) == State::Active {
        if header.data[HeaderButton::CONTROL]
            .get_state()
            .unwrap_or(State::Disabled)
            == State::Hovered
        {
            control_panel
                .text
                .default("Switch to Control View - ENTER key   ");
            control_panel.offset = 0;
        }
        if header.data[HeaderButton::STREAMING]
            .get_state()
            .unwrap_or(State::Disabled)
            == State::Hovered
        {
            control_panel
                .text
                .default("Switch to Streaming View - ENTER key   ");
            control_panel.offset = 0;
        }
        if header.data[HeaderButton::LOG]
            .get_state()
            .unwrap_or(State::Disabled)
            == State::Hovered
        {
            control_panel
                .text
                .default("Switch to Log View - ENTER key   ");
            control_panel.offset = 0;
        }
        if header.data[HeaderButton::SETTINGS]
            .get_state()
            .unwrap_or(State::Disabled)
            == State::Hovered
        {
            control_panel.text.default(format!(
                "Settings - Edit the configuration file at {}   ",
                get_configuration_path()
                    .unwrap_or(PathBuf::new())
                    .to_str()
                    .unwrap()
            ));
            control_panel.offset = 0;
        }
        if header.data[HeaderButton::EXIT]
            .get_state()
            .unwrap_or(State::Disabled)
            == State::Hovered
        {
            control_panel
                .text
                .default("Exit Application - ENTER key   ");
            control_panel.offset = 0;
        }
        if header.data[HeaderButton::EXIT]
            .get_state()
            .unwrap_or(State::Disabled)
            == State::Active
        {
            *running = false;
        }
    }
}

pub fn handle_audio_controls(
    audio_controls: &Grid,
    player: &mut PlayerSession,
    sound: &mut Option<StaticSoundHandle>,
    song_list: &mut Grid,
    control_panel: &mut Text,
) {
    if audio_controls.get_state().unwrap_or(State::Disabled) == State::Hovered {
        control_panel
            .text
            .default("Audio Controls - Shuffle, Previous, Pause, Next, Restart   ");
        control_panel.offset = 0;
    }
    if audio_controls.get_state().unwrap_or(State::Disabled) != State::Active {
        return;
    }

    if audio_controls.data[AudioControlButton::SHUFFLE]
        .get_state()
        .unwrap_or(State::Disabled)
        == State::Hovered
    {
        control_panel.text.default(" Shuffle Songs - ENTER key   ");
        control_panel.offset = 0;
    }
    if audio_controls.data[AudioControlButton::SHUFFLE]
        .get_state()
        .unwrap_or(State::Disabled)
        == State::Active
        && sound.is_some()
    {
        player.shuffle();
        update_song_list(song_list, player);
    }

    if audio_controls.data[AudioControlButton::PREVIOUS]
        .get_state()
        .unwrap_or(State::Disabled)
        == State::Hovered
    {
        control_panel.text.default("Previous Song - ENTER key   ");
        control_panel.offset = 0;
    }
    if audio_controls.data[AudioControlButton::PREVIOUS]
        .get_state()
        .unwrap_or(State::Disabled)
        == State::Active
        && sound.is_some()
    {
        player.previous();
        player.previous();
        sound.as_mut().unwrap().stop(Tween::default());
    }

    if audio_controls.data[AudioControlButton::PLAY_PAUSE]
        .get_state()
        .unwrap_or(State::Disabled)
        == State::Hovered
    {
        control_panel.text.default("Pause/Resume - ENTER key   ");
        control_panel.offset = 0;
    }
    if audio_controls.data[AudioControlButton::PLAY_PAUSE]
        .get_state()
        .unwrap_or(State::Disabled)
        == State::Active
        && sound.is_some()
    {
        match sound.as_mut().unwrap().state() {
            PlaybackState::Paused => {
                sound.as_mut().unwrap().resume(Tween::default());
            }
            PlaybackState::Playing => {
                sound.as_mut().unwrap().pause(Tween::default());
            }
            _ => {}
        }
    }

    if audio_controls.data[AudioControlButton::NEXT]
        .get_state()
        .unwrap_or(State::Disabled)
        == State::Hovered
    {
        control_panel.text.default("Next Song - ENTER key   ");
        control_panel.offset = 0;
    }
    if audio_controls.data[AudioControlButton::NEXT]
        .get_state()
        .unwrap_or(State::Disabled)
        == State::Active
        && sound.is_some()
    {
        sound.as_mut().unwrap().stop(Tween::default());
    }

    if audio_controls.data[AudioControlButton::RESTART]
        .get_state()
        .unwrap_or(State::Disabled)
        == State::Hovered
    {
        control_panel.text.default("Restart Song - ENTER key   ");
        control_panel.offset = 0;
    }
    if audio_controls.data[AudioControlButton::RESTART]
        .get_state()
        .unwrap_or(State::Disabled)
        == State::Active
        && sound.is_some()
    {
        sound.as_mut().unwrap().seek_to(0.0);
    }
}

pub fn trigger_recommendation_refresh(streaming: &mut StreamingSession, player: &PlayerSession) {
    if streaming.refreshing {
        return;
    }

    let library = LibraryIndex::from_song_keys(&player.songs);
    let artist_count = library.artist_count();

    logger!().add_entry(
        EntryId::Streaming,
        Entry::new(format!(
            "Refreshing recommendations (found {} artists in library)...",
            artist_count
        )),
    );

    let config = StreamingConfigSnapshot {
        recommendations_per_fetch: configuration!().streaming.recommendations_per_fetch,
        listenbrainz_mode: configuration!().streaming.listenbrainz_mode.clone(),
    };
    let mbid_cache_path = PROJECT_DIRECTORY.data_dir().join("mbid_cache.json");
    streaming.refresh_recommendations(library, config, mbid_cache_path);
}

pub fn handle_streaming_controls(
    recommendations: &mut Grid,
    streaming: &mut StreamingSession,
    player: &PlayerSession,
    playlist_dropdown: &mut PlaylistDropdown,
    control_panel: &mut Text,
) {
    if !configuration!().streaming.enabled {
        return;
    }

    if recommendations.get_state().unwrap_or(State::Disabled) == State::Hovered {
        control_panel
            .text
            .default("Recommendations - Download, Refresh   ");
        control_panel.offset = 0;
    }
    if recommendations.get_state().unwrap_or(State::Disabled) != State::Active {
        return;
    }

    let last_index = recommendations.data.len() - 1;

    for i in 0..=last_index {
        let is_refresh_row = i == last_index;

        if recommendations.data[i]
            .get_state()
            .unwrap_or(State::Disabled)
            == State::Hovered
        {
            if is_refresh_row {
                control_panel
                    .text
                    .default("Refresh Recommendations - ENTER key   ");
            } else if let Some(track) = streaming.recommendations.get(i) {
                if track.recording_mbid.is_some() {
                    control_panel
                        .text
                        .default(format!("Download This Track - ENTER key   ",));
                } else {
                    control_panel.text.default("N/A   ");
                }
            } else {
                control_panel
                    .text
                    .default("Download This Track - ENTER key   ");
            }
            control_panel.offset = 0;
        }

        if recommendations.data[i]
            .get_state()
            .unwrap_or(State::Disabled)
            == State::Active
        {
            if is_refresh_row {
                trigger_recommendation_refresh(streaming, player);
            } else if let Some(track) = streaming.recommendations.get(i).cloned() {
                let config = configuration!();
                let download_dir = if config.streaming.download_directory.is_empty() {
                    if let Some(current_playlist_path) = playlist_dropdown.current_path() {
                        PathBuf::from(current_playlist_path)
                    } else {
                        PathBuf::from(&config.songs_directory)
                    }
                } else {
                    PathBuf::from(&config.streaming.download_directory)
                };
                let ytdlp_path = config.streaming.ytdlp_path.clone();
                drop(config);
                streaming.start_download(track, download_dir, ytdlp_path);
            }
        }
    }
}

pub fn handle_playlist_controls(
    playlist_dropdown: &mut PlaylistDropdown,
    streaming: &mut StreamingSession,
    recommendations: &mut Grid,
    player: &mut PlayerSession,
    song_list: &mut Grid,
    sound: &mut Option<StaticSoundHandle>,
    control_panel: &mut Text,
) {
    if playlist_dropdown.get_state().unwrap_or(State::Disabled) == State::Hovered {
        control_panel.text.default("Playlist Controls - Switch   ");
        control_panel.offset = 0;
    }

    if playlist_dropdown.get_state().unwrap_or(State::Disabled) == State::Active {
        control_panel
            .text
            .default("Switch Playlist - UP or DOWN arrow keys, or Enter key   ");
        control_panel.offset = 0;
    }

    if playlist_dropdown.take_changed().is_none() {
        return;
    }

    let Some(path) = playlist_dropdown.current_path().map(|p| p.to_string()) else {
        return;
    };

    if let Some(current_sound) = sound.as_mut() {
        current_sound.stop(Tween::default());
    }

    player.load_playlist(&path);
    update_song_list(song_list, player);

    set_recommendations_loading(recommendations);
    streaming.recommendations.clear();
    trigger_recommendation_refresh(streaming, player);

    if configuration!().restore_last_playlist
        && let Err(e) = save_last_playlist(path)
    {
        warn!("Failed to save last playlist: {}", e);
    }
}
