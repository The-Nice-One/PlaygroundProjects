use super::stateful_string::StatefulString;
use super::trait_def::{Component, State};
use super::{Button, Null, Toggle};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum GridItem {
    Null(Null),
    StatefulString(StatefulString),
    Button(Button),
    Toggle(Toggle),
}

impl Component for GridItem {
    fn display(&self) -> String {
        match self {
            GridItem::Null(item) => item.display(),
            GridItem::StatefulString(item) => item.display(),
            GridItem::Button(item) => item.display(),
            GridItem::Toggle(item) => item.display(),
        }
    }
    fn feed(&mut self, event: &crossterm::event::Event) {
        match self {
            GridItem::Null(item) => item.feed(event),
            GridItem::StatefulString(item) => item.feed(event),
            GridItem::Button(item) => item.feed(event),
            GridItem::Toggle(item) => item.feed(event),
        }
    }
    fn set_state(&mut self, state: State) {
        match self {
            GridItem::Null(item) => item.set_state(state),
            GridItem::StatefulString(item) => item.set_state(state),
            GridItem::Button(item) => item.set_state(state),
            GridItem::Toggle(item) => item.set_state(state),
        }
    }
    fn get_state(&self) -> Option<State> {
        match self {
            GridItem::Null(item) => item.get_state(),
            GridItem::StatefulString(item) => item.get_state(),
            GridItem::Button(item) => item.get_state(),
            GridItem::Toggle(item) => item.get_state(),
        }
    }
}

pub struct Grid {
    pub data: Vec<GridItem>,
    pub size: (u32, u32),
    pub left_spacer: StatefulString,
    pub right_spacer: StatefulString,
    pub horizontal_spacer: StatefulString,
    pub hovered: (u32, u32),
    pub state: State,
}

impl Grid {
    pub fn new(size: (u32, u32)) -> Self {
        let mut data: Vec<GridItem> = Vec::with_capacity((size.0 * size.1) as usize);
        for _ in 0..size.0 * size.1 {
            data.push(GridItem::Null(Null::default()));
        }
        Self {
            data,
            size,
            left_spacer: "".into(),
            right_spacer: "".into(),
            horizontal_spacer: "".into(),
            hovered: (0, 0),
            state: State::Default,
        }
    }
}

impl fmt::Display for Grid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut writer_x = 0;
        for index in 0..self.size.0 * self.size.1 {
            if writer_x == 0 {
                write!(f, "{}", self.left_spacer)?;
            }

            write!(f, "{}", self.data[index as usize].display())?;
            if writer_x < self.size.0 - 1 {
                write!(f, "{}", self.horizontal_spacer)?;
            }
            writer_x += 1;

            if writer_x == self.size.0 {
                write!(f, "{}", self.right_spacer)?;
            }

            if writer_x > self.size.0 - 1 && index as usize != self.data.len() - 1 {
                writer_x = 0;
                writeln!(f)?;
            }
        }
        write!(f, "")
    }
}

impl Component for Grid {
    fn display(&self) -> String {
        format!("{}", self)
    }
    fn feed(&mut self, event: &crossterm::event::Event) {
        if let crossterm::event::Event::Key(key_event) = event {
            let old_index = (self.hovered.0 + self.hovered.1 * self.size.0) as usize;
            let old_hovered = self.hovered;

            let mut changed_selection = false;

            if key_event.code == crossterm::event::KeyCode::Left && self.hovered.0 > 0 {
                self.hovered.0 -= 1;
                changed_selection = true;
            }
            if key_event.code == crossterm::event::KeyCode::Right
                && self.hovered.0 < self.size.0 - 1
            {
                self.hovered.0 += 1;
                changed_selection = true;
            }
            if key_event.code == crossterm::event::KeyCode::Up && self.hovered.1 > 0 {
                self.hovered.1 -= 1;
                changed_selection = true;
            }
            if key_event.code == crossterm::event::KeyCode::Down && self.hovered.1 < self.size.1 - 1
            {
                self.hovered.1 += 1;
                changed_selection = true;
            }

            let mut new_index = (self.hovered.0 + self.hovered.1 * self.size.0) as usize;
            if changed_selection && self.data[new_index].get_state() == Some(State::Disabled) {
                self.hovered = old_hovered;
                new_index = old_index;
            }
            if changed_selection {
                if old_index != new_index {
                    self.data[old_index].set_state(State::Default);
                }
                self.data[new_index].set_state(State::Hovered);
            } else {
                self.data[new_index].feed(event);
            }
        }
    }
    fn set_state(&mut self, state: State) {
        self.state = state;
        self.left_spacer.state = state;
        self.right_spacer.state = state;
        self.horizontal_spacer.state = state;

        let current_index = (self.hovered.0 + self.hovered.1 * self.size.0) as usize;
        if !(self.data[current_index].get_state() == Some(State::Disabled))
            && state == State::Active
        {
            self.data[current_index].set_state(State::Hovered);
        }
    }
    fn get_state(&self) -> Option<State> {
        Some(self.state)
    }
}
