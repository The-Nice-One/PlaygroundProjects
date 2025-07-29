use crate::components::trait_def::Component;

pub trait Feeder {
    fn feed(&mut self, event: &crossterm::event::Event, components: Vec<Box<&mut dyn Component>>);
}
