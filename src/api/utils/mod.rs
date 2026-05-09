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

pub struct ParamParser<'a> {
    parts: &'a [String],
    pos: usize,
}

impl<'a> ParamParser<'a> {
    pub fn new(parts: &'a [String], start: usize) -> Self {
        Self { parts, pos: start }
    }
    pub fn is_empty(&self) -> bool {
        self.pos >= self.parts.len()
    }
    pub fn peek(&self) -> Option<&str> {
        self.parts.get(self.pos).map(|s| s.as_str())
    }
    pub fn next(&mut self, name: &str) -> anyhow::Result<&str> {
        self.parts
            .get(self.pos)
            .map(|s| { self.pos += 1; s.as_str() })
            .ok_or_else(|| anyhow::anyhow!("missing parameter: {}", name))
    }
    pub fn next_opt(&mut self) -> Option<&str> {
        self.parts.get(self.pos).map(|s| { self.pos += 1; s.as_str() })
    }
    pub fn rest(&self) -> &[String] {
        &self.parts[self.pos..]
    }
    pub fn finish(&self) -> anyhow::Result<()> {
        match self.parts.get(self.pos) {
            None => Ok(()),
            Some(extra) => Err(anyhow::anyhow!("extra parameter: {}", extra)),
        }
    }
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
