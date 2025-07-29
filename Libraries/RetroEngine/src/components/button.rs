use super::{Component, State, StatefulString};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Button {
    pub state: State,
    pub text: StatefulString,
}

impl fmt::Display for Button {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.state {
            State::Default => write!(f, "{}", self.text.default),
            State::Hovered => write!(f, "{}", self.text.hovered),
            State::Active => write!(f, "{}", self.text.active),
            State::Disabled => write!(f, "{}", self.text.disabled),
        }
    }
}

impl From<StatefulString> for Button {
    fn from(value: StatefulString) -> Self {
        Self {
            state: State::Default,
            text: value,
        }
    }
}

impl From<&mut StatefulString> for Button {
    fn from(value: &mut StatefulString) -> Self {
        Self {
            state: State::Default,
            text: value.clone(),
        }
    }
}

impl Component for Button {
    fn display(&self) -> String {
        format!("{}", self)
    }
    fn feed(&mut self, event: &crossterm::event::Event) {
        if self.state == State::Hovered || self.state == State::Active {
            if let crossterm::event::Event::Key(event) = event {
                if event.code == crossterm::event::KeyCode::Enter {
                    self.state = State::Active;
                } else {
                    self.state = State::Hovered;
                }
            }
        }
    }

    fn set_state(&mut self, state: State) {
        self.state = state;
    }
    fn get_state(&self) -> Option<State> {
        Some(self.state)
    }
}
