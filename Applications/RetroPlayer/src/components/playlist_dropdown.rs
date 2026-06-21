use crate::theme;
use retro_engine::components::*;
use retro_engine::{Event, KeyCode, Stylize};
use std::path::Path;

pub struct PlaylistDropdown {
    pub state: State,
    pub playlists: Vec<String>,
    pub last_selected: usize,
    pub selected: usize,
    changed: bool,
    pub left: StatefulString,
    pub right: StatefulString,
}

impl PlaylistDropdown {
    pub fn new(playlists: Vec<String>, selected: usize) -> Self {
        let bounded_selected = if playlists.is_empty() {
            0
        } else {
            selected.min(playlists.len() - 1)
        };

        Self {
            state: State::Default,
            playlists,
            last_selected: usize::MAX,
            selected: bounded_selected,
            changed: false,
            left: StatefulString::from("[  ".with(theme!().primary))
                .hovered("[  ".with(theme!().accent))
                .active("[  ".with(theme!().accent).bold())
                .into(),
            right: StatefulString::from("  ]".with(theme!().primary))
                .hovered("  ]".with(theme!().accent))
                .active("  ]".with(theme!().accent).bold())
                .into(),
        }
    }

    pub fn current_path(&self) -> Option<&str> {
        self.playlists.get(self.selected).map(|s| s.as_str())
    }

    pub fn take_changed(&mut self) -> Option<usize> {
        if self.changed {
            self.changed = false;
            Some(self.selected)
        } else {
            None
        }
    }

    fn current_label(&self) -> String {
        let Some(path) = self.current_path() else {
            return "No playlists configured".to_string();
        };

        Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_string())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| path.to_string())
    }
}

impl std::fmt::Display for PlaylistDropdown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}",
            self.left.display(),
            format!("↕ Streaming {}", self.current_label()).with(theme!().primary),
            self.right.display()
        )
    }
}

impl Component for PlaylistDropdown {
    fn display(&self) -> String {
        format!("{}", self)
    }

    fn feed(&mut self, event: &retro_engine::Event) {
        if self.playlists.len() <= 1 || self.state != State::Active {
            return;
        }

        if let Event::Key(event) = event {
            if event.code == KeyCode::Up {
                self.selected = if self.selected == 0 {
                    self.playlists.len() - 1
                } else {
                    self.selected - 1
                };
            }

            if event.code == KeyCode::Down {
                self.selected = (self.selected + 1) % self.playlists.len();
            }

            if event.code == KeyCode::Enter && self.selected != self.last_selected {
                self.last_selected = self.selected;
                self.changed = true;
            }
        }
    }

    fn get_state(&self) -> Option<State> {
        Some(self.state)
    }

    fn set_state(&mut self, state: State) {
        self.state = state;
        self.left.state = state;
        self.right.state = state;
    }
}
