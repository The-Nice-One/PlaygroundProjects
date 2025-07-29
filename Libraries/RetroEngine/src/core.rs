use core::panic;
use std::io::stdout;

use std::io::Write;
use std::time::Duration;

use crossterm::event::{poll, read, Event};

use crossterm::QueueableCommand;
use unicode_segmentation::UnicodeSegmentation;

use strip_ansi_escapes;

pub struct Terminal {
    pub screen: Screen,
    pub event: Option<Event>,
}

pub struct Screen {
    pub width: u16,
    pub height: u16,
}

impl Terminal {
    pub fn init() -> Self {
        crossterm::terminal::enable_raw_mode().unwrap();
        crossterm::execute!(stdout(), crossterm::event::EnableFocusChange).unwrap();

        Self {
            screen: Screen {
                width: crossterm::terminal::size().unwrap().0,
                height: crossterm::terminal::size().unwrap().1,
            },
            event: None,
        }
    }
    pub fn print(&self, string: &String) -> String {
        let mut strings: Vec<String> = string.split("\n").map(|s| s.to_string()).collect();
        for string in strings.iter_mut() {
            let dif = self.screen.width
                - String::from_utf8(strip_ansi_escapes::strip(string.clone()))
                    .unwrap()
                    .graphemes(true)
                    .count() as u16;
            *string += String::from(" ").repeat(dif as usize).as_str();
            //stdout().queue(crossterm::cursor::MoveToColumn(0));
            stdout().queue(crossterm::style::Print(string)).unwrap();
            //stdout().queue(crossterm::cursor::MoveToNextLine(1));
            // stdout().queue(crossterm::cursor::MoveDown(1));
            // stdout().queue(crossterm::cursor::MoveToColumn(0));
        }
        stdout().flush().unwrap();
        strings.join("")
    }
    pub fn poll(&mut self, timeout: u64) {
        if poll(Duration::from_millis(timeout)).unwrap() {
            self.event = Some(read().unwrap());
            match self.event.as_ref().unwrap() {
                crossterm::event::Event::Key(event) => {
                    if event
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL)
                        && event.code == crossterm::event::KeyCode::Char('c')
                    {
                        crossterm::terminal::disable_raw_mode().unwrap();
                        panic!();
                    }
                }
                _ => (),
            }
        } else {
            self.event = None;
        }
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
