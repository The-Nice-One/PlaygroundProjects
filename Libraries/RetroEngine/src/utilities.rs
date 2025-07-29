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
