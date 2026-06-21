use crate::logger::EntryId;
use crate::theme;
use crate::{Entry, EntryStatus, Log};
use chrono::prelude::*;
use chrono_humanize::HumanTime;
use retro_engine::{Stylize, components::*};
use std::sync::{Arc, Mutex};

pub struct LogDisplay<'a> {
    title: Option<StatefulString>,
    logger: &'a Arc<Mutex<Log<EntryId>>>,
    state: State,
}

impl<'a> LogDisplay<'a> {
    pub fn from_logger(logger: &'a Arc<Mutex<Log<EntryId>>>) -> Self {
        Self {
            title: None,
            logger,
            state: State::default(),
        }
    }
}

fn format_entry(entry: &Entry) -> String {
    let mut entry_display = match entry.status {
        EntryStatus::Incomplete => String::from("☐ ").with(theme!().primary).to_string(),
        EntryStatus::Complete => String::from("☑ ").with(theme!().accent).to_string(),
        EntryStatus::Failed => String::from("☒ ").with(theme!().secondary).to_string(),
    };
    entry_display.push_str(&entry.description);
    let delta_time = entry.timestamp.signed_duration_since(Local::now());
    let human_time = HumanTime::from(delta_time);
    entry_display.push_str(&format!(" ({})", human_time));

    entry_display
}

impl std::fmt::Display for LogDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let title = if let Some(title) = &self.title {
            title.display() + "\n"
        } else {
            String::new()
        };
        write!(f, "{}", title)?;

        for (_, entry) in self.logger.lock().unwrap().entries.iter() {
            write!(f, "{}\n", format_entry(&entry))?;
        }
        write!(f, "")
    }
}

impl Component for LogDisplay<'_> {
    fn display(&self) -> String {
        format!("{}", self)
    }
    fn feed(&mut self, _event: &retro_engine::Event) {}

    fn get_state(&self) -> Option<State> {
        Some(self.state)
    }
    fn set_state(&mut self, state: State) {
        self.state = state;
        if let Some(title) = &mut self.title {
            title.state(state);
        }
    }
}
