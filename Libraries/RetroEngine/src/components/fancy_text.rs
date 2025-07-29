use super::trait_def::Component;
use std::fmt;
use text2art::BasicFonts;
use text2art::Font;
use text2art::Printer;

pub struct FancyText {
    text: &'static str,
    printer: Printer,
}

impl FancyText {
    pub fn new(text: &'static str) -> Self {
        let font = match Font::from_basic(BasicFonts::Big) {
            Ok(font) => font,
            Err(_) => panic!("something wrong with font"),
        };
        let prntr = Printer::with_font(font);
        Self {
            text,
            printer: prntr,
        }
    }
    pub fn parse(text: String) -> String {
        let mut iter = text.chars();
        iter.next_back();
        iter.next_back();
        iter.as_str().to_owned()
    }
}

impl fmt::Display for FancyText {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            FancyText::parse(self.printer.render_text(self.text).unwrap())
        )
    }
}

impl Component for FancyText {
    fn display(&self) -> String {
        format!("{}", self)
    }
    fn feed(&mut self, _event: &crossterm::event::Event) {}
}
