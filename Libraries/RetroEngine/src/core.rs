use core::panic;
use std::io::stdout;

use std::io::Write;
use std::time::Duration;

use crossterm::event::{poll, read, Event};

use crossterm::QueueableCommand;

use crate::utilities::{length, take};

pub struct Terminal {
    pub screen: Screen,
    pub configuration: Configuration,
    pub event: Option<Event>,
    pub polls: u64,
    pub cache: Cache,
}

pub struct Configuration {
    pub overwrite_lines: bool,
}

pub struct Cache {
    pub max_height: u16,
}

pub struct Screen {
    pub width: u16,
    pub height: u16,
}

impl Terminal {
    pub fn init() -> Self {
        crossterm::terminal::enable_raw_mode().unwrap();
        crossterm::execute!(stdout(), crossterm::event::EnableFocusChange).unwrap();
        crossterm::execute!(
            stdout(),
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
        )
        .unwrap();

        Self {
            screen: Screen {
                width: crossterm::terminal::size().unwrap().0,
                height: crossterm::terminal::size().unwrap().1,
            },
            configuration: Configuration {
                overwrite_lines: false,
            },
            event: None,
            polls: 0,
            cache: Cache { max_height: 0 },
        }
    }
    pub fn hide_cursor(&self) {
        crossterm::execute!(stdout(), crossterm::cursor::Hide).unwrap();
    }
    pub fn show_cursor(&self) {
        crossterm::execute!(stdout(), crossterm::cursor::Show).unwrap();
    }
    pub fn deinit(&self) {
        crossterm::terminal::disable_raw_mode().unwrap();
        crossterm::execute!(stdout(), crossterm::event::DisableFocusChange).unwrap();
    }
    pub fn print(&mut self, string: &str) -> String {
        let mut strings: Vec<String> = string.split("\n").map(|s| s.to_string()).collect();

        let frame_height = strings.len() as u16;
        if self.configuration.overwrite_lines {
            if frame_height < self.cache.max_height {
                for _ in 0..(self.cache.max_height - frame_height) {
                    strings.push(String::new());
                }
            }
        }

        for (row_index, string) in strings.iter_mut().enumerate() {
            let row_index = row_index as u16;

            let current_length = length(&string);

            if current_length < self.screen.width as usize {
                *string += &" ".repeat(self.screen.width as usize - current_length);
            } else if current_length > self.screen.width as usize && self.screen.width > 0 {
                *string = take(&string, 0, self.screen.width as usize - 1);
            }

            stdout()
                .queue(crossterm::cursor::MoveTo(0, row_index))
                .unwrap();
            stdout().queue(crossterm::style::Print(string)).unwrap();
        }
        stdout().flush().unwrap();
        if strings.len() > self.cache.max_height as usize {
            self.cache.max_height = strings.len() as u16;
        }
        strings.join("\n")
    }
    pub fn poll(&mut self, timeout: u64) {
        self.screen.width = crossterm::terminal::size().unwrap().0;
        self.screen.height = crossterm::terminal::size().unwrap().1;
        self.polls += 1;
        if poll(Duration::from_millis(timeout)).unwrap() {
            self.event = Some(read().unwrap());
            match self.event.as_ref().unwrap() {
                crossterm::event::Event::Key(event) => {
                    if event
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL)
                        && event.code == crossterm::event::KeyCode::Char('c')
                    {
                        self.deinit();
                        panic!();
                    }
                }
                _ => (),
            }
        } else {
            self.event = None;
        }
    }
    pub fn goto_y(&self, y: u16) {
        stdout().queue(crossterm::cursor::MoveTo(0, y)).unwrap();
    }
    pub fn top(&self) {
        // stdout()
        //     .queue(crossterm::terminal::SetSize(self.screen.width, 3))
        //     .unwrap();
        stdout().queue(crossterm::cursor::MoveTo(0, 0)).unwrap();
        // stdout()
        //     .queue(crossterm::terminal::Clear(
        //         crossterm::terminal::ClearType::FromCursorDown,
        //     ))
        //     .unwrap();

        // stdout()
        //     .queue(crossterm::terminal::Clear(
        //         crossterm::terminal::ClearType::FromCursorDown,
        //     ))
        //     .unwrap();

        stdout().flush().unwrap();
    }
}
