use super::trait_def::{Component, State};

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Null {
    pub state: State,
}

impl Null {
    pub fn disabled() -> Self {
        Self {
            state: State::Disabled,
        }
    }
}

impl Component for Null {
    fn display(&self) -> String {
        "".to_string()
    }
    fn feed(&mut self, _event: &crossterm::event::Event) {}
    fn set_state(&mut self, state: State) {
        self.state = state;
    }
    fn get_state(&self) -> Option<State> {
        Some(self.state)
    }
}
