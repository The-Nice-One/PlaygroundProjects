use super::{trait_def::Component, StatefulString};
use crate::utilities::{length, take};
use std::fmt;

pub struct Text {
    pub text: StatefulString,
    pub min_width: Option<u16>,
    pub max_width: Option<u16>,
    pub offset: u16,
    pub looping: bool,
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut offset = self.offset;
        let text_length = length(&self.text.display()) as u16;
        let min_width = self.min_width.unwrap_or(0);
        let max_width = self.max_width.unwrap_or(text_length);

        if offset > text_length {
            offset = text_length;
        }

        if text_length >= min_width {
            let mut current_length = 0;

            let end = offset as usize + max_width as usize - 1;
            let content = take(&self.text.display(), offset as usize, end);
            write!(f, "{}", content)?;
            current_length += length(&content);
            if current_length < min_width as usize {
                let padding = min_width as usize - current_length;
                if self.looping {
                    let padding = take(&self.text.display(), 0, padding as usize - 1);
                    write!(f, "{}", padding)?;
                } else {
                    let padding = " ".repeat(padding as usize);
                    write!(f, "{}", padding)?;
                }
            }
            write!(f, "")
        } else {
            let difference = min_width - text_length;
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
    pub fn new<T: Into<StatefulString>>(
        text: T,
        min_width: Option<u16>,
        max_width: Option<u16>,
        offset: u16,
        looping: bool,
    ) -> Self {
        Self {
            text: text.into(),
            min_width,
            max_width,
            offset,
            looping,
        }
    }
}
