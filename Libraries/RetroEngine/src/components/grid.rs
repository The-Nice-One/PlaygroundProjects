use crate::utilities::length;

use super::stateful_string::StatefulString;
use super::trait_def::{Component, State};
use super::{Button, Null, Radio, Toggle};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadioGroup {
    pub id: usize,
}

impl From<usize> for RadioGroup {
    fn from(id: usize) -> Self {
        Self { id }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GridItem {
    Null(Null),
    StatefulString(StatefulString),
    Button(Button),
    Toggle(Toggle),
    Radio(Radio, RadioGroup),
}

impl Component for GridItem {
    fn display(&self) -> String {
        match self {
            GridItem::Null(item) => item.display(),
            GridItem::StatefulString(item) => item.display(),
            GridItem::Button(item) => item.display(),
            GridItem::Toggle(item) => item.display(),
            GridItem::Radio(item, ..) => item.display(),
        }
    }
    fn feed(&mut self, event: &crossterm::event::Event) {
        match self {
            GridItem::Null(item) => item.feed(event),
            GridItem::StatefulString(item) => item.feed(event),
            GridItem::Button(item) => item.feed(event),
            GridItem::Toggle(item) => item.feed(event),
            GridItem::Radio(item, ..) => item.feed(event),
        }
    }
    fn set_state(&mut self, state: State) {
        match self {
            GridItem::Null(item) => item.set_state(state),
            GridItem::StatefulString(item) => item.set_state(state),
            GridItem::Button(item) => item.set_state(state),
            GridItem::Toggle(item) => item.set_state(state),
            GridItem::Radio(item, ..) => item.set_state(state),
        }
    }
    fn get_state(&self) -> Option<State> {
        match self {
            GridItem::Null(item) => item.get_state(),
            GridItem::StatefulString(item) => item.get_state(),
            GridItem::Button(item) => item.get_state(),
            GridItem::Toggle(item) => item.get_state(),
            GridItem::Radio(item, ..) => item.get_state(),
        }
    }
}

pub struct Grid {
    pub data: Vec<GridItem>,
    pub size: (u32, u32),
    pub top_left_spacer: Option<StatefulString>,
    pub left_spacer: StatefulString,
    pub bottom_left_spacer: Option<StatefulString>,
    pub top_right_spacer: Option<StatefulString>,
    pub right_spacer: StatefulString,
    pub bottom_right_spacer: Option<StatefulString>,
    pub horizontal_spacer: StatefulString,
    pub padded_cells: bool,
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
            top_left_spacer: None,
            left_spacer: "".into(),
            bottom_left_spacer: None,
            top_right_spacer: None,
            right_spacer: "".into(),
            bottom_right_spacer: None,
            horizontal_spacer: "".into(),
            padded_cells: false,
            hovered: (0, 0),
            state: State::Default,
        }
    }
    pub fn get_active_radio(&self, radio_group: RadioGroup) -> Option<&Radio> {
        self.data
            .iter()
            .filter_map(|item| match item {
                GridItem::Radio(item, group) => {
                    if group.id == radio_group.id && item.is_on {
                        Some(item)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .next()
    }
    pub fn update_cell_stateful_string(&mut self, index: usize, new_string: StatefulString) {
        if index < self.data.len() {
            match self.data[index] {
                GridItem::StatefulString(ref mut item) => *item = new_string,
                GridItem::Button(ref mut item) => item.text = new_string,
                _ => {}
            }
        }
    }
}

impl fmt::Display for Grid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut writer_x = 0;

        let mut cells_max_width = vec![0; self.size.0 as usize];
        if self.padded_cells {
            for index in 0..self.size.0 * self.size.1 {
                let cell_width = length(&self.data[index as usize].display());
                let x = index as usize % self.size.0 as usize;
                if cell_width > cells_max_width[x] {
                    cells_max_width[x] = cell_width;
                }
            }
        }

        for index in 0..self.size.0 * self.size.1 {
            if writer_x == 0 && index == 0 && self.top_left_spacer.is_some() {
                write!(f, "{}", self.top_left_spacer.as_ref().unwrap())?;
            } else if writer_x == 0
                && index as usize == self.data.len() - 1
                && self.bottom_left_spacer.is_some()
            {
                write!(f, "{}", self.bottom_left_spacer.as_ref().unwrap())?;
            } else if writer_x == 0 {
                write!(f, "{}", self.left_spacer)?;
            }

            let cell_data = &self.data[index as usize].display();
            write!(f, "{}", cell_data)?;
            if self.padded_cells {
                let cell_width = length(&cell_data);
                if cell_width < cells_max_width[writer_x as usize] {
                    write!(
                        f,
                        "{}",
                        " ".repeat(cells_max_width[writer_x as usize] - cell_width)
                    )?;
                }
            }

            if writer_x < self.size.0 - 1 {
                write!(f, "{}", self.horizontal_spacer)?;
            }
            writer_x += 1;

            if writer_x == self.size.0 && index == 0 && self.top_right_spacer.is_some() {
                write!(f, "{}", self.top_right_spacer.as_ref().unwrap())?;
            } else if writer_x == self.size.0
                && index as usize == self.data.len() - 1
                && self.bottom_right_spacer.is_some()
            {
                write!(f, "{}", self.bottom_right_spacer.as_ref().unwrap())?;
            } else if writer_x == self.size.0 {
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
            if key_event.kind == crossterm::event::KeyEventKind::Release {
                return;
            }

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
                let current_item = self.data[new_index].clone();
                match current_item {
                    GridItem::Radio(radio, radio_group) => {
                        if radio.is_on {
                            let related_radios =
                                self.data.iter_mut().filter_map(|item| match item {
                                    GridItem::Radio(item, group) => {
                                        if group.id == radio_group.id {
                                            Some((item, group))
                                        } else {
                                            None
                                        }
                                    }
                                    _ => None,
                                });
                            for (item, ..) in related_radios {
                                if !(item == &radio) {
                                    item.is_on = false;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    fn set_state(&mut self, state: State) {
        self.state = state;
        if let Some(spacer) = self.top_left_spacer.as_mut() {
            spacer.state = state;
        }
        if let Some(spacer) = self.bottom_left_spacer.as_mut() {
            spacer.state = state;
        }
        if let Some(spacer) = self.top_right_spacer.as_mut() {
            spacer.state = state;
        }
        if let Some(spacer) = self.bottom_right_spacer.as_mut() {
            spacer.state = state;
        }
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
