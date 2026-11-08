use crate::theme;
use retro_engine::components::*;
use retro_engine::{Event, KeyCode, Stylize};

pub struct SearchBox {
    pub state: State,
    pub prefix: StatefulString,
    pub query: String,
    pub postfix: String,
    pub left: StatefulString,
    pub right: StatefulString,
    pub submitted: bool,
}

impl SearchBox {
    pub fn new() -> Self {
        Self {
            state: State::Default,
            prefix: StatefulString::from("⌕ Search for ".with(theme!().primary))
                .hovered("⌕ Search for ".with(theme!().accent))
                .active("⌕ Search for ".with(theme!().accent).bold())
                .into(),
            query: String::new(),
            postfix: String::from(" "),
            left: StatefulString::from("[  ".with(theme!().primary))
                .hovered("[  ".with(theme!().accent))
                .active("[  ".with(theme!().accent).bold())
                .into(),
            right: StatefulString::from("  ]".with(theme!().primary))
                .hovered("  ]".with(theme!().accent))
                .active("  ]".with(theme!().accent).bold())
                .into(),
            submitted: false,
        }
    }

    pub fn take_submitted_query(&mut self) -> Option<String> {
        if self.submitted {
            self.submitted = false;
            Some(self.query.trim().to_string())
        } else {
            None
        }
    }
}

impl std::fmt::Display for SearchBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}{}",
            self.left.display(),
            self.prefix.display(),
            format!("{}{}", self.query, self.postfix).with(theme!().primary),
            self.right.display()
        )
    }
}

impl Component for SearchBox {
    fn display(&self) -> String {
        format!("{}", self)
    }

    fn feed(&mut self, event: &retro_engine::Event) {
        if self.state != State::Active {
            return;
        }

        if let Event::Key(event) = event {
            if let Some(character) = event.code.as_char() {
                self.query.push(character);
            }
            if event.code == KeyCode::Backspace {
                self.query.pop();
            }

            if event.code == KeyCode::Enter {
                if !self.query.trim().is_empty() {
                    self.submitted = true;
                }
                self.set_state(State::Hovered);
            }
        }
    }

    fn get_state(&self) -> Option<State> {
        Some(self.state)
    }

    fn set_state(&mut self, state: State) {
        self.state = state;
        if self.state == State::Active {
            self.prefix.state = State::Hovered;
        } else {
            self.prefix.state = state;
        }
        self.left.state = state;
        self.right.state = state;
    }
}
