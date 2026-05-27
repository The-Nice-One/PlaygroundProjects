use super::{View, ViewState};
use crate::{
    builders::update_control_view,
    handlers::{handle_audio_controls, handle_header_controls},
};
use retro_engine::components::*;
use retro_engine::feeders::GridFeeder;

pub struct ControlView {
    controller: GridFeeder,
}

impl ControlView {
    pub fn new() -> Self {
        let mut controller = GridFeeder::new((2, 2));
        controller.hovered = (1, 0);

        Self { controller }
    }
}

impl View<ViewState<'_>> for ControlView {
    fn controller(&mut self) -> &mut GridFeeder {
        &mut self.controller
    }

    fn components<'a>(&self, state: &'a mut ViewState) -> Vec<Box<&'a mut dyn Component>> {
        vec![
            Box::new(state.null_component),
            Box::new(state.header),
            Box::new(state.audio_controls),
            Box::new(state.volume_bar),
        ]
    }

    fn handle_event(&mut self, _event: &retro_engine::Event, state: &mut ViewState) {
        handle_header_controls(state.header, state.control_panel, state.running);
        handle_audio_controls(
            state.audio_controls,
            state.player,
            state.sound,
            state.song_list,
            state.control_panel,
        );

        if state.volume_bar.get_state().unwrap_or(State::Disabled) == State::Hovered {
            state
                .control_panel
                .text
                .default("Volume Controls - Adjust   ");
            state.control_panel.offset = 0;
        }
        if state.volume_bar.get_state().unwrap_or(State::Disabled) == State::Active {
            state
                .control_panel
                .text
                .default("Adjust Volume - UP or DOWN arrow keys   ");
            state.control_panel.offset = 0;
        }

        use kira::Tween;
        state
            .sound
            .as_mut()
            .unwrap()
            .set_volume(state.volume_bar.db, Tween::default());
    }

    fn render(&self, state: &mut ViewState) -> String {
        update_control_view(
            state.player_bar,
            state.side_bar,
            state.song_list,
            state.header,
            state.control_panel,
            state.audio_controls,
            state.volume_bar,
            state.terminal,
            state.sound,
        )
    }
}
