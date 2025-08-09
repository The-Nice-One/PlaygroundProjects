use crate::Stylize;
use unicode_segmentation::UnicodeSegmentation;

pub fn max_length<T: ToString>(strings: &[T]) -> usize {
    let mut max_length = 0;
    for string in strings {
        let current_length = String::from_utf8(strip_ansi_escapes::strip(string.to_string()))
            .unwrap()
            .graphemes(true)
            .count();
        if current_length > max_length {
            max_length = current_length
        }
    }
    max_length
}

pub fn length<T: ToString>(string: &T) -> usize {
    String::from_utf8(strip_ansi_escapes::strip(string.to_string()))
        .unwrap()
        .graphemes(true)
        .count()
}

pub fn take<T: ToString>(string: &T, start: usize, end: usize) -> String {
    let string = string.to_string();
    let mut result = String::new();
    let mut grapheme_count = 0;
    let mut chars = string.chars().peekable();
    while let Some(char) = chars.next() {
        if char == '\x1b' && chars.peek() == Some(&'[') && grapheme_count >= start {
            result.push(char);
            result.push(chars.next().unwrap());
            while let Some(char) = chars.next() {
                result.push(char);
                if char.is_alphabetic() {
                    break;
                }
            }
        } else {
            if grapheme_count > end {
                break;
            }

            let remianing_characters: String = std::iter::once(char).chain(chars.clone()).collect();
            let mut ramianing_graphemes = remianing_characters.graphemes(true);
            if let Some(grapheme) = ramianing_graphemes.next() {
                if grapheme_count >= start {
                    result.push_str(grapheme);
                }
                grapheme_count += 1;

                for _ in 1..grapheme.chars().count() {
                    chars.next();
                }
            }
        }
    }
    result + &"".reset().to_string()
}
