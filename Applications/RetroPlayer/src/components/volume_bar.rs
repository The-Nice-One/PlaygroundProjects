use retro_engine::components::*;
use retro_engine::{Event, KeyCode, Stylize};

pub struct VolumeBar {
    pub value: u32,
    pub db: f32,
    pub state: State,

    pub left: StatefulString,
    pub right: StatefulString,
    pub progress_bar: ProgressBar,
}

impl VolumeBar {
    pub fn new() -> Self {
        let mut volume = ProgressBar::default();

        volume.left("█".blue());
        volume.pointer(vec![
            "▁".blue(),
            "▂".blue(),
            "▃".blue(),
            "▄".blue(),
            "▅".blue(),
            "▆".blue(),
            "▇".blue(),
            "█".blue(),
        ]);
        volume.minimum = 0;
        volume.maximum = 100;
        volume.value = 100;
        volume.width = 1;

        Self {
            value: 100,
            db: Self::volume_to_db(100),
            state: State::Default,
            left: StatefulString::from("[  ")
                .hovered("[  ".blue())
                .active("[  ".blue().bold())
                .into(),
            right: StatefulString::from("  ]")
                .hovered("  ]".blue())
                .active("  ]".blue().bold())
                .into(),
            progress_bar: volume,
        }
    }
    pub fn volume_to_db(value: u32) -> f32 {
        if value == 0 {
            f32::NEG_INFINITY
        } else {
            let value = value as f32;
            let normalized = value / 100.0;
            20.0 * (normalized * normalized).log10()
        }
    }
}

impl std::fmt::Display for VolumeBar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}% {}{}",
            self.left.display(),
            self.value,
            self.progress_bar.display(),
            self.right
        )
    }
}

impl Component for VolumeBar {
    fn display(&self) -> String {
        format!("{}", self)
    }
    fn feed(&mut self, event: &retro_engine::Event) {
        if self.state == State::Active {
            if let Event::Key(event) = event {
                if event.code == KeyCode::Up && self.value < 100 {
                    self.value += 1;
                    self.progress_bar.value = self.value;
                    self.db = Self::volume_to_db(self.value);
                }
                if event.code == KeyCode::Down && self.value > 0 {
                    self.value -= 1;
                    self.progress_bar.value = self.value;
                    self.db = Self::volume_to_db(self.value);
                }
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
