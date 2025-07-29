use super::trait_def::Component;
use std::fmt;

#[derive(Debug, Clone, Default)]
pub struct ProgressBar {
    pub width: u32,
    pub minimum: u32,
    pub value: u32,
    pub maximum: u32,
    pub left: String,
    pub pointer: Vec<String>,
    pub right: String,
}

impl ProgressBar {
    pub fn left<T: ToString + fmt::Display>(&mut self, left: T) -> &mut Self {
        self.left = left.to_string();
        self
    }
    pub fn pointer<T: ToString + fmt::Display>(&mut self, pointer: Vec<T>) -> &mut Self {
        self.pointer = pointer.iter().map(|x| x.to_string()).collect();
        self
    }
    pub fn right<T: ToString + fmt::Display>(&mut self, right: T) -> &mut Self {
        self.right = right.to_string();
        self
    }
}

impl fmt::Display for ProgressBar {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let range = self.maximum - self.minimum;
        let step = 100.0 / range as f64;
        let percentage = (self.value - self.minimum) as f64 * step;

        let percentage_per_character = 100.0 / self.width as f64;
        let used_characters = (percentage / percentage_per_character) as u32;

        let percentage_left = percentage - used_characters as f64 * percentage_per_character;
        let pointer_stages = self.pointer.len();
        let percentage_per_stage = percentage_per_character / pointer_stages as f64;
        let pointer_stage = (percentage_left / percentage_per_stage) as usize;

        let mut rendered_characters = 0;

        for _ in 0..used_characters {
            write!(f, "{}", self.left)?;
            rendered_characters += 1;
        }

        if rendered_characters < self.width {
            write!(f, "{}", self.pointer[pointer_stage])?;
            rendered_characters += 1;
        }

        if self.width > rendered_characters {
            for _ in 0..self.width - rendered_characters {
                write!(f, "{}", self.right)?;
            }
        }

        write!(f, "")
    }
}

impl Component for ProgressBar {
    fn display(&self) -> String {
        format!("{}", self)
    }
    fn feed(&mut self, _event: &crossterm::event::Event) {}
}
