use super::trait_def::Component;
use std::fmt;

#[derive(Debug, Clone, Default)]
pub struct VerticalLine {
    height: u32,
    middle: String,
    start: Option<String>,
    end: Option<String>,
}

impl VerticalLine {
    pub fn new() -> Self {
        Self {
            height: 0,
            middle: String::new(),
            start: None,
            end: None,
        }
    }
    pub fn height(&mut self, height: u32) -> &mut Self {
        self.height = height;
        self
    }
    pub fn middle<T: ToString + fmt::Display>(&mut self, middle: T) -> &mut Self {
        self.middle = middle.to_string();
        self
    }
    pub fn start<T: ToString + fmt::Display>(&mut self, start: T) -> &mut Self {
        self.start = Some(start.to_string());
        self
    }
    pub fn end<T: ToString + fmt::Display>(&mut self, end: T) -> &mut Self {
        self.end = Some(end.to_string());
        self
    }
}

impl fmt::Display for VerticalLine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for index in 1..self.height {
            if index == 1 && self.height >= 2 && self.start.is_some() {
                writeln!(f, "{}", self.start.as_ref().unwrap())?;
            } else {
                writeln!(f, "{}", self.middle)?;
            }
        }

        if self.height >= 2 && self.end.is_some() {
            write!(f, "{}", self.end.as_ref().unwrap())
        } else {
            write!(f, "{}", self.middle)
        }
    }
}

impl Component for VerticalLine {
    fn display(&self) -> String {
        format!("{}", self)
    }
    fn feed(&mut self, _event: &crossterm::event::Event) {}
}
