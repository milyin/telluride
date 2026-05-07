pub fn split_with_screened_spaces(arg: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut chars = arg.lines().next().unwrap_or("").chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(&next_c) = chars.peek() {
                    if next_c == '\\' {
                        current.push('\\');
                        chars.next();
                    } else if next_c == ' ' {
                        current.push(' ');
                        chars.next();
                    } else {
                        current.push('\\');
                    }
                } else {
                    current.push('\\');
                }
            }
            ' ' => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

pub fn screen_spaces(s: &str) -> String {
    s.replace('\\', "\\\\").replace(' ', "\\ ")
}

/// Formats multiple `Display` values by wrapping each with [`screen_spaces`] and joining with spaces.
#[macro_export]
macro_rules! format_screen_spaces {
    () => { String::new() };
    ($($arg:expr),+) => {{
        let parts: Vec<String> = vec![
            $( $crate::utils::screen_spaces(&$arg.to_string()) ),+
        ];
        parts.join(" ")
    }};
}
