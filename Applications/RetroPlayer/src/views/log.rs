use super::{View, ViewState};
use crate::{builders::update_log_view, handlers::handle_header_controls};
use retro_engine::components::*;
use retro_engine::feeders::GridFeeder;

pub struct LogView {
    controller: GridFeeder,
}

impl LogView {
    pub fn new() -> Self {
        let mut controller = GridFeeder::new((2, 1));
        controller.hovered = (1, 0);

        Self { controller }
    }
}

impl View<ViewState<'_>> for LogView {
    fn controller(&mut self) -> &mut GridFeeder {
        &mut self.controller
    }

    fn components<'a>(&self, state: &'a mut ViewState) -> Vec<Box<&'a mut dyn Component>> {
        vec![Box::new(state.null_component), Box::new(state.header)]
    }

    fn handle_event(&mut self, _event: &retro_engine::Event, state: &mut ViewState) {
        handle_header_controls(state.header, state.control_panel, state.running);
    }

    fn render(&self, state: &mut ViewState) -> String {
        update_log_view(
            state.side_bar,
            state.song_list,
            state.header,
            state.control_panel,
            state.log,
            state.terminal,
        )
    }
}
