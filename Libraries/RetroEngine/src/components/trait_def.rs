#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum State {
    #[default]
    // Component is not in use
    Default,
    // Component can be directly interacted with
    Hovered,
    // Component is being interacted with
    Active,
    // Component is disabled
    Disabled,
}

pub trait Component {
    fn display(&self) -> String;
    fn feed(&mut self, _event: &crossterm::event::Event) {}

    fn set_state(&mut self, _state: State) {}
    fn get_state(&self) -> Option<State> {
        None
    }
}
