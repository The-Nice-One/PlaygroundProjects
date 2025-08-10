use crate::PlayerSession;
use crate::update_song_list;
use kira::{Tween, sound::PlaybackState, sound::static_sound::StaticSoundHandle};
use retro_engine::components::*;

pub fn handle_header_controls(
    header: &mut Grid,
    control_panel: &mut Text,
    configuration_path: &String,
    running: &mut bool,
) {
    if header.get_state().unwrap_or_else(|| State::Disabled) == State::Hovered {
        control_panel
            .text
            .default("Header Controls - Settings, Exit");
        control_panel.offset = 0;
    }

    if header.get_state().unwrap_or(State::Disabled) == State::Active {
        if header.data[0].get_state().unwrap_or(State::Disabled) == State::Hovered {
            control_panel.text.default(format!(
                "Settings - Edit the configuration file at {}   ",
                configuration_path
            ));
            control_panel.offset = 0;
        }
        if header.data[1].get_state().unwrap_or(State::Disabled) == State::Hovered {
            control_panel
                .text
                .default("Exit Application - ENTER key   ");
            control_panel.offset = 0;
        }
        if header.data[1].get_state().unwrap_or(State::Disabled) == State::Active {
            *running = false;
        }
    }
}

pub fn handle_audio_controls(
    audio_controls: &Grid,
    player: &mut PlayerSession,
    sound: &mut Option<StaticSoundHandle>,
    player_bar: &mut ProgressBar,
    song_list: &mut Grid,
    control_panel: &mut Text,
) {
    if audio_controls.get_state().unwrap_or(State::Disabled) == State::Hovered {
        control_panel
            .text
            .default("Audio Controls - Shuffle, Previous, Pause, Next, Restart   ");
        control_panel.offset = 0;
    }
    if audio_controls
        .get_state()
        .unwrap_or_else(|| State::Disabled)
        != State::Active
    {
        return;
    }

    if audio_controls.data[0]
        .get_state()
        .unwrap_or(State::Disabled)
        == State::Hovered
    {
        control_panel.text.default(" Shuffle Songs - ENTER key   ");
        control_panel.offset = 0;
    }
    if audio_controls.data[0]
        .get_state()
        .unwrap_or(State::Disabled)
        == State::Active
        && sound.is_some()
    {
        player.shuffle();
        update_song_list(song_list, &player);
    }

    if audio_controls.data[1]
        .get_state()
        .unwrap_or(State::Disabled)
        == State::Hovered
    {
        control_panel.text.default("Previous Song - ENTER key   ");
        control_panel.offset = 0;
    }
    if audio_controls.data[1]
        .get_state()
        .unwrap_or(State::Disabled)
        == State::Active
        && sound.is_some()
    {
        player.previous();
        player.previous();
        sound.as_mut().unwrap().seek_to(player_bar.maximum as f64);
    }

    if audio_controls.data[2]
        .get_state()
        .unwrap_or(State::Disabled)
        == State::Hovered
    {
        control_panel.text.default("Pause/Resume - ENTER key   ");
        control_panel.offset = 0;
    }
    if audio_controls.data[2]
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

    if audio_controls.data[3]
        .get_state()
        .unwrap_or(State::Disabled)
        == State::Hovered
    {
        control_panel.text.default("Next Song - ENTER key   ");
        control_panel.offset = 0;
    }
    if audio_controls.data[3]
        .get_state()
        .unwrap_or(State::Disabled)
        == State::Active
        && sound.is_some()
    {
        sound.as_mut().unwrap().seek_to(player_bar.maximum as f64);
    }

    if audio_controls.data[4]
        .get_state()
        .unwrap_or(State::Disabled)
        == State::Hovered
    {
        control_panel.text.default("Restart Song - ENTER key   ");
        control_panel.offset = 0;
    }
    if audio_controls.data[4]
        .get_state()
        .unwrap_or(State::Disabled)
        == State::Active
        && sound.is_some()
    {
        sound.as_mut().unwrap().seek_to(0.0);
        //.set_loop_region(0.0..player_bar.maximum as f64);
    }
}
