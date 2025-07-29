use crate::components::trait_def::{Component, State};
use crate::feeders::trait_def::Feeder;

#[derive(Default)]
pub struct GridFeeder {
    pub size: (u32, u32),
    pub hovered: (u32, u32),
}

impl GridFeeder {
    pub fn new(size: (u32, u32)) -> Self {
        Self {
            size,
            hovered: (0, 0),
        }
    }
}

impl Feeder for GridFeeder {
    fn feed(
        &mut self,
        event: &crossterm::event::Event,
        mut components: Vec<Box<&mut dyn Component>>,
    ) {
        let mut pressed_escape = false;
        if let crossterm::event::Event::Key(key_event) = event {
            if key_event.code == crossterm::event::KeyCode::Esc {
                pressed_escape = true;
            }
        }

        let index = (self.hovered.0 + self.hovered.1 * self.size.0) as usize;

        if components[index].get_state() == Some(State::Active) && !pressed_escape {
            components[index].feed(&event);
        } else if let crossterm::event::Event::Key(key_event) = event {
            let old_index = (self.hovered.0 + self.hovered.1 * self.size.0) as usize;
            if pressed_escape {
                components[old_index].set_state(State::Hovered);
            }

            if key_event.code == crossterm::event::KeyCode::Enter {
                components[old_index].set_state(State::Active);
            }

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
            if changed_selection && components[new_index].get_state() == Some(State::Disabled) {
                self.hovered = old_hovered;
                new_index = old_index;
            }
            if changed_selection {
                if old_index != new_index {
                    components[old_index].set_state(State::Default);
                }
                components[new_index].set_state(State::Hovered);
            }
        }
    }
}
