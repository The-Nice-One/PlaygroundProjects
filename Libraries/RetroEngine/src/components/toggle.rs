use super::{Component, State, StatefulString};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Toggle {
    pub state: State,
    pub on: StatefulString,
    pub off: StatefulString,
    pub is_on: bool,
}

impl Toggle {
    pub fn new(on: StatefulString, off: StatefulString, is_on: bool) -> Self {
        Self {
            state: State::Default,
            on,
            off,
            is_on: is_on,
        }
    }
}

impl fmt::Display for Toggle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_on {
            match self.state {
                State::Default => write!(f, "{}", self.on.default),
                State::Hovered => write!(f, "{}", self.on.hovered),
                State::Active => write!(f, "{}", self.on.active),
                State::Disabled => write!(f, "{}", self.on.disabled),
            }
        } else {
            match self.state {
                State::Default => write!(f, "{}", self.off.default),
                State::Hovered => write!(f, "{}", self.off.hovered),
                State::Active => write!(f, "{}", self.off.active),
                State::Disabled => write!(f, "{}", self.off.disabled),
            }
        }
    }
}

impl From<StatefulString> for Toggle {
    fn from(value: StatefulString) -> Self {
        Self {
            state: State::Default,
            on: value.clone(),
            off: value.clone(),
            is_on: false,
        }
    }
}

impl From<&mut StatefulString> for Toggle {
    fn from(value: &mut StatefulString) -> Self {
        Self {
            state: State::Default,
            on: value.clone(),
            off: value.clone(),
            is_on: false,
        }
    }
}

impl Component for Toggle {
    fn display(&self) -> String {
        format!("{}", self)
    }
    fn feed(&mut self, event: &crossterm::event::Event) {
        if self.state == State::Hovered || self.state == State::Active {
            if let crossterm::event::Event::Key(event) = event {
                if event.code == crossterm::event::KeyCode::Enter {
                    self.state = State::Active;
                    self.is_on = !self.is_on;
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
