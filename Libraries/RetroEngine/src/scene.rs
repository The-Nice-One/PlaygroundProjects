use unicode_segmentation::UnicodeSegmentation;

pub fn align_horizontally(base: String, join: String, spacer: String) -> String {
    let mut base_vec: Vec<&str> = base.split("\n").collect();
    let mut join_vec: Vec<&str> = join.split("\n").collect();

    let mut cloned_vec = base_vec.clone();

    cloned_vec.sort_by(|a, b| {
        String::from_utf8(strip_ansi_escapes::strip(b))
            .unwrap()
            .graphemes(true)
            .count()
            .cmp(
                &String::from_utf8(strip_ansi_escapes::strip(a))
                    .unwrap()
                    .graphemes(true)
                    .count(),
            )
    });

    let longest_line = String::from_utf8(strip_ansi_escapes::strip(cloned_vec[0]))
        .unwrap()
        .graphemes(true)
        .count() as u32;

    let mut returned = String::from("");

    let adder = String::from(" ").repeat(longest_line as usize);
    let mut index = 0;

    if base_vec.len() > join_vec.len() {
        for _ in 0..base_vec.len() - join_vec.len() {
            join_vec.push("");
        }
    } else if join_vec.len() > base_vec.len() {
        for _ in 0..join_vec.len() - base_vec.len() {
            base_vec.push(&adder);
        }
    }

    for line in base_vec.iter() {
        let lenght: u32 = String::from_utf8(strip_ansi_escapes::strip(line))
            .unwrap()
            .graphemes(true)
            .count() as u32;
        returned.push_str(line);

        if lenght < longest_line {
            for _ in 0..longest_line - lenght {
                returned.push(' ');
            }
        }
        returned.push_str(&spacer);

        returned.push_str(join_vec[index]);
        if index != base_vec.len() - 1 {
            returned.push('\n');
        }
        index += 1;
    }
    returned
}

pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

pub fn align_vertically(base: String, join: String, alignment: HorizontalAlignment) -> String {
    let mut main_vec: Vec<&str> = base.split("\n").collect();
    let mut join_vec: Vec<&str> = join.split("\n").collect();
    main_vec.append(&mut join_vec);

    let mut cloned_vec = main_vec.clone();
    cloned_vec.sort_by(|a, b| {
        String::from_utf8(strip_ansi_escapes::strip(b))
            .unwrap()
            .graphemes(true)
            .count()
            .cmp(
                &String::from_utf8(strip_ansi_escapes::strip(a))
                    .unwrap()
                    .graphemes(true)
                    .count(),
            )
    });

    let longest_line = String::from_utf8(strip_ansi_escapes::strip(cloned_vec[0]))
        .unwrap()
        .graphemes(true)
        .count() as u32;

    let mut returned = String::from("");
    match alignment {
        HorizontalAlignment::Left => {
            for (index, line) in main_vec.iter().enumerate() {
                returned.push_str(line);
                if index != main_vec.len() - 1 {
                    returned.push('\n');
                }
            }
        }
        HorizontalAlignment::Center => {}
        HorizontalAlignment::Right => {
            for (index, line) in main_vec.iter().enumerate() {
                let lenght: u32 = String::from_utf8(strip_ansi_escapes::strip(line))
                    .unwrap()
                    .graphemes(true)
                    .count() as u32;

                if lenght < longest_line {
                    for _ in 0..longest_line - lenght {
                        returned.push(' ');
                    }
                }

                returned.push_str(line);
                if index != main_vec.len() - 1 {
                    returned.push('\n');
                }
            }
        }
    }

    returned
}
