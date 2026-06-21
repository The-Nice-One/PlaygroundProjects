use super::{View, ViewState};
use crate::builders::update_streaming_view;
use crate::handlers::{
    handle_header_controls, handle_playlist_controls, handle_streaming_controls,
};
use retro_engine::components::*;
use retro_engine::feeders::GridFeeder;

pub struct StreamingView {
    controller: GridFeeder,
}

impl StreamingView {
    pub fn new() -> Self {
        let mut controller = GridFeeder::new((2, 2));
        controller.hovered = (1, 0);

        Self { controller }
    }
}

impl View<ViewState<'_>> for StreamingView {
    fn controller(&mut self) -> &mut GridFeeder {
        &mut self.controller
    }

    fn components<'a>(&self, state: &'a mut ViewState) -> Vec<Box<&'a mut dyn Component>> {
        vec![
            Box::new(state.playlist_dropdown),
            Box::new(state.header),
            Box::new(state.null_component),
            Box::new(state.recommendations),
        ]
    }

    fn handle_event(&mut self, _event: &retro_engine::Event, state: &mut ViewState) {
        handle_header_controls(state.header, state.control_panel, state.running);
        handle_streaming_controls(
            state.recommendations,
            state.streaming,
            state.player,
            state.playlist_dropdown,
            state.control_panel,
        );
        handle_playlist_controls(
            state.playlist_dropdown,
            state.streaming,
            state.recommendations,
            state.player,
            state.song_list,
            state.sound,
            state.control_panel,
        );
    }

    fn render(&self, state: &mut ViewState) -> String {
        update_streaming_view(
            state.side_bar,
            state.song_list,
            state.header,
            state.control_panel,
            state.recommendations,
            state.playlist_dropdown,
            state.terminal,
        )
    }
}
