use crate::components::{LogDisplay, PlaylistDropdown, VolumeBar};
use crate::streaming::StreamingSession;
use crate::PlayerSession;
use kira::sound::static_sound::StaticSoundHandle;
use retro_engine::components::*;
use retro_engine::core::Terminal;
use retro_engine::feeders::GridFeeder;

pub mod control;
pub mod log;
pub mod streaming;

pub type AppView = dyn for<'a> View<ViewState<'a>>;

pub struct ViewState<'a> {
    // Shared components
    pub header: &'a mut Grid,
    pub control_panel: &'a mut Text,
    pub side_bar: &'a VerticalLine,
    pub song_list: &'a mut Grid,
    pub playlist_dropdown: &'a mut PlaylistDropdown,
    pub terminal: &'a Terminal,
    pub running: &'a mut bool,
    pub null_component: &'a mut Null,

    // Control view specific
    pub player_bar: &'a mut ProgressBar,
    pub audio_controls: &'a mut Grid,
    pub volume_bar: &'a mut VolumeBar,
    pub sound: &'a mut Option<StaticSoundHandle>,
    pub player: &'a mut PlayerSession,

    // Log view specific
    pub log: &'a LogDisplay<'a>,

    // Streaming view specific
    pub recommendations: &'a mut Grid,
    pub streaming: &'a mut StreamingSession,
}

pub trait View<T> {
    fn controller(&mut self) -> &mut GridFeeder;
    fn components<'a>(&self, state: &'a mut T) -> Vec<Box<&'a mut dyn Component>>;
    fn handle_event(&mut self, event: &retro_engine::Event, state: &mut T);
    fn render(&self, state: &mut T) -> String;
}

pub struct ViewRegistry<V: ?Sized> {
    pub views: Vec<Box<V>>,
    pub current_view_id: usize,
}

impl<V: ?Sized> ViewRegistry<V> {
    pub fn new() -> Self {
        Self {
            views: vec![],
            current_view_id: 0,
        }
    }

    pub fn current_view(&mut self) -> Option<&mut Box<V>> {
        self.views.get_mut(self.current_view_id)
    }

    pub fn register(&mut self, view: Box<V>) {
        self.views.push(view);
    }

    pub fn switch_to(&mut self, view_index: usize) {
        if view_index < self.views.len() {
            self.current_view_id = view_index;
        }
    }
}
