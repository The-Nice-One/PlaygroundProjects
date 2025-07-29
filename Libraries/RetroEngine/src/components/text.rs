use crate::utilities::length;

use super::{trait_def::Component, StatefulString};
use std::fmt;

pub struct Text {
    pub text: StatefulString,
    pub min_width: Option<u16>,
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text_length = length(&self.text.display()) as u16;
        if text_length >= self.min_width.unwrap_or_else(|| 0) {
            write!(f, "{}", self.text)
        } else {
            let difference = self.min_width.unwrap() - text_length;
            let addtion = (difference % 2 == 1) as u16;
            let padding_left = difference / 2 + addtion;
            let padding_right = difference / 2;
            write!(
                f,
                "{}{}{}",
                " ".repeat(padding_left as usize),
                self.text,
                " ".repeat(padding_right as usize)
            )
        }
    }
}

impl Component for Text {
    fn display(&self) -> String {
        format!("{}", self)
    }
    fn feed(&mut self, _event: &crossterm::event::Event) {}
}

impl Text {
    pub fn new<T: Into<StatefulString>>(text: T, min_width: Option<u16>) -> Self {
        Self {
            text: text.into(),
            min_width,
        }
    }
}
