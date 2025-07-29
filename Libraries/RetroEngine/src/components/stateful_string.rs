use super::trait_def::{Component, State};
use std::fmt;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StatefulString {
    pub default: String,
    pub hovered: String,
    pub active: String,
    pub disabled: String,
    pub state: State,
}

impl StatefulString {
    pub fn new(default: String, hovered: String, active: String, disabled: String) -> Self {
        Self {
            default,
            hovered,
            active,
            disabled,
            state: State::Default,
        }
    }
    pub fn default<T: ToString + fmt::Display>(&mut self, default: T) -> &mut Self {
        self.default = default.to_string();
        self
    }
    pub fn hovered<T: ToString + fmt::Display>(&mut self, hovered: T) -> &mut Self {
        self.hovered = hovered.to_string();
        self
    }
    pub fn active<T: ToString + fmt::Display>(&mut self, active: T) -> &mut Self {
        self.active = active.to_string();
        self
    }
    pub fn disabled<T: ToString + fmt::Display>(&mut self, disabled: T) -> &mut Self {
        self.disabled = disabled.to_string();
        self
    }
    pub fn state(&mut self, state: State) -> &mut Self {
        self.state = state;
        self
    }
}

impl fmt::Display for StatefulString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.state {
            State::Default => write!(f, "{}", self.default),
            State::Hovered => write!(f, "{}", self.hovered),
            State::Active => write!(f, "{}", self.active),
            State::Disabled => write!(f, "{}", self.disabled),
        }
    }
}

impl From<&str> for StatefulString {
    fn from(value: &str) -> Self {
        Self {
            default: value.to_string(),
            hovered: value.to_string(),
            active: value.to_string(),
            disabled: value.to_string(),
            state: State::Default,
        }
    }
}

impl From<String> for StatefulString {
    fn from(value: String) -> Self {
        Self {
            default: value.clone(),
            hovered: value.clone(),
            active: value.clone(),
            disabled: value.clone(),
            state: State::Default,
        }
    }
}

impl From<&mut StatefulString> for StatefulString {
    fn from(value: &mut StatefulString) -> Self {
        value.clone()
    }
}

impl<D: std::fmt::Display> From<crossterm::style::StyledContent<D>> for StatefulString {
    fn from(value: crossterm::style::StyledContent<D>) -> Self {
        Self {
            default: value.to_string(),
            hovered: value.to_string(),
            active: value.to_string(),
            disabled: value.to_string(),
            state: State::Default,
        }
    }
}

impl Component for StatefulString {
    fn display(&self) -> String {
        format!("{}", self)
    }
    fn feed(&mut self, _event: &crossterm::event::Event) {}
    fn set_state(&mut self, state: State) {
        self.state = state
    }
    fn get_state(&self) -> Option<State> {
        return Some(self.state);
    }
}
