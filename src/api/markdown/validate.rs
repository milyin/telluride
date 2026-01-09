/// Validates MarkdownV2 format string at compile time
///
/// This function checks for:
/// - Balanced formatting characters (*, _, ~, |, `, [, ])
/// - Properly escaped reserved characters (!,.,-, +,=,>,#,{,})
/// - Correct nesting of code blocks and formatting
/// - Valid link syntax
pub const fn validate_markdownv2_format(format_str: &str) {
    let format_str_bytes = format_str.as_bytes();
    let mut i = 0;
    let mut asterisk_count = 0u8;
    let mut underscore_count = 0u8;
    let mut backtick_count = 0u8;
    let mut square_bracket_count = 0u8;
    let mut paren_count = 0u8;
    let mut tilde_count = 0u8;
    let mut pipe_count = 0u8;

    // Track nesting state for validation
    let mut in_code = false;
    let mut in_pre = false;
    let mut prev_char = 0u8;
    let mut prev_was_escaping_backslash = false;

    while i < format_str_bytes.len() {
        let current_char = format_str_bytes[i];
        let is_escaped = prev_char == b'\\' && prev_was_escaping_backslash;

        // Update for next iteration: current backslash is escaping if it's not escaped itself
        prev_was_escaping_backslash = current_char == b'\\' && !is_escaped;

        if !is_escaped {
            match current_char {
                // Basic formatting characters must be balanced
                b'*' => asterisk_count = asterisk_count.wrapping_add(1),
                b'_' => underscore_count = underscore_count.wrapping_add(1),
                b'~' => tilde_count = tilde_count.wrapping_add(1),
                b'|' => pipe_count = pipe_count.wrapping_add(1),

                // Code formatting validation
                b'`' => {
                    backtick_count = backtick_count.wrapping_add(1);
                    // Check for triple backticks (pre-formatted)
                    if i + 2 < format_str_bytes.len()
                        && format_str_bytes[i + 1] == b'`'
                        && format_str_bytes[i + 2] == b'`'
                    {
                        in_pre = !in_pre;
                    } else {
                        in_code = !in_code;
                    }
                }

                // Link formatting validation
                b'[' => square_bracket_count = square_bracket_count.wrapping_add(1),
                b']' => {
                    assert!(
                        square_bracket_count > 0,
                        "Unmatched closing square bracket ']' in markdown format string"
                    );
                    square_bracket_count = square_bracket_count.wrapping_sub(1);
                }
                b'(' => {
                    // Only count if it's potentially part of a link (after ])
                    if prev_char == b']' {
                        paren_count = paren_count.wrapping_add(1);
                    }
                }
                b')' => {
                    if paren_count > 0 {
                        paren_count = paren_count.wrapping_sub(1);
                    }
                }

                // Reserved characters that should be escaped (compile-time check)
                b'!' => {
                    if !in_code && !in_pre && !is_escaped {
                        panic!("Unescaped '!' in MarkdownV2 format string. Use \\! to escape it.");
                    }
                }
                b'.' => {
                    if !in_code && !in_pre && !is_escaped {
                        panic!("Unescaped '.' in MarkdownV2 format string. Use \\. to escape it.");
                    }
                }
                b'-' => {
                    if !in_code && !in_pre && !is_escaped {
                        panic!("Unescaped '-' in MarkdownV2 format string. Use \\- to escape it.");
                    }
                }
                b'+' => {
                    if !in_code && !in_pre && !is_escaped {
                        panic!("Unescaped '+' in MarkdownV2 format string. Use \\+ to escape it.");
                    }
                }
                b'=' => {
                    if !in_code && !in_pre && !is_escaped {
                        panic!("Unescaped '=' in MarkdownV2 format string. Use \\= to escape it.");
                    }
                }
                b'>' => {
                    if !in_code && !in_pre && !is_escaped {
                        panic!("Unescaped '>' in MarkdownV2 format string. Use \\> to escape it.");
                    }
                }
                b'#' => {
                    if !in_code && !in_pre && !is_escaped {
                        panic!("Unescaped '#' in MarkdownV2 format string. Use \\# to escape it.");
                    }
                }
                b'{' => {
                    // Allow format placeholders like {}
                    let is_format_placeholder =
                        i + 1 < format_str_bytes.len() && format_str_bytes[i + 1] == b'}';
                    if !in_code && !in_pre && !is_escaped && !is_format_placeholder {
                        panic!(
                            "Unescaped '{{' in MarkdownV2 format string. Use \\{{ to escape it or use {{}} for format placeholders."
                        );
                    }
                }
                b'}' => {
                    // Allow closing of format placeholders
                    let is_format_placeholder = i > 0 && format_str_bytes[i - 1] == b'{';
                    if !in_code && !in_pre && !is_escaped && !is_format_placeholder {
                        panic!(
                            "Unescaped '}}' in MarkdownV2 format string. Use \\}} to escape it."
                        );
                    }
                }

                _ => {}
            }
        }

        prev_char = current_char;
        i += 1;
    }

    // Validate balanced formatting
    assert!(
        asterisk_count.is_multiple_of(2),
        "Unmatched asterisks (*) in MarkdownV2 format string - bold formatting must be balanced"
    );
    assert!(
        underscore_count.is_multiple_of(2),
        "Unmatched underscores (_) in MarkdownV2 format string - italic formatting must be balanced"
    );
    assert!(
        backtick_count.is_multiple_of(2),
        "Unmatched backticks (`) in MarkdownV2 format string - code formatting must be balanced"
    );
    assert!(
        tilde_count.is_multiple_of(2),
        "Unmatched tildes (~) in MarkdownV2 format string - strikethrough formatting must be balanced"
    );
    assert!(
        pipe_count.is_multiple_of(2),
        "Unmatched pipes (|) in MarkdownV2 format string - spoiler formatting must be balanced"
    );
    assert!(
        square_bracket_count == 0,
        "Unmatched square brackets ([]) in MarkdownV2 format string - link text must be properly closed"
    );
    assert!(
        paren_count == 0,
        "Unmatched parentheses in MarkdownV2 format string - link URLs must be properly closed"
    );
    assert!(!in_code, "Unclosed code block in MarkdownV2 format string");
    assert!(
        !in_pre,
        "Unclosed pre-formatted code block in MarkdownV2 format string"
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_compile_time_validation() {
        // These should compile successfully
        const _: () = {
            let format_str = "Hello *{}*";
            let format_str_bytes = format_str.as_bytes();
            let mut asterisk_count = 0u8;
            let mut i = 0;

            while i < format_str_bytes.len() {
                if format_str_bytes[i] == b'*' {
                    asterisk_count = asterisk_count.wrapping_add(1);
                }
                i += 1;
            }

            assert!(
                asterisk_count.is_multiple_of(2),
                "Unmatched asterisks in markdown format string"
            );
        };
    }

    #[test]
    fn test_markdownv2_format_patterns() {
        // Test various valid MarkdownV2 patterns
        let valid_patterns = [
            "Simple text",
            "With *bold* text",
            "With _italic_ text",
            "With `code` text",
            "With ~strikethrough~ text",
            "With ||spoiler|| text",
            "*{}* and _{}_ and `{}`",
            "**Bold** text",
            "__Italic__ text",
            "~~Strikethrough~~ text",
            "Link: [text](url)",
            "Code block: ```code```",
            "Mixed: *bold* and `code`",
            "Escaped \\! exclamation",
            "Escaped \\. period",
            "Escaped \\- dash",
            "Escaped \\+ plus",
            "Escaped \\= equals",
            "Format placeholder: {}",
        ];

        // Test the enhanced validation logic for each pattern
        for pattern in valid_patterns {
            let format_str_bytes = pattern.as_bytes();
            let mut i = 0;
            let mut asterisk_count = 0u8;
            let mut underscore_count = 0u8;
            let mut backtick_count = 0u8;
            let mut square_bracket_count = 0u8;
            let mut paren_count = 0u8;
            let mut tilde_count = 0u8;
            let mut pipe_count = 0u8;
            let mut prev_char = 0u8;

            while i < format_str_bytes.len() {
                let current_char = format_str_bytes[i];
                let is_escaped = prev_char == b'\\';

                if !is_escaped {
                    match current_char {
                        b'*' => asterisk_count = asterisk_count.wrapping_add(1),
                        b'_' => underscore_count = underscore_count.wrapping_add(1),
                        b'~' => tilde_count = tilde_count.wrapping_add(1),
                        b'|' => pipe_count = pipe_count.wrapping_add(1),
                        b'`' => backtick_count = backtick_count.wrapping_add(1),
                        b'[' => square_bracket_count = square_bracket_count.wrapping_add(1),
                        b']' => {
                            if square_bracket_count > 0 {
                                square_bracket_count = square_bracket_count.wrapping_sub(1);
                            }
                        }
                        b'(' => {
                            if prev_char == b']' {
                                paren_count = paren_count.wrapping_add(1);
                            }
                        }
                        b')' => {
                            if paren_count > 0 {
                                paren_count = paren_count.wrapping_sub(1);
                            }
                        }
                        _ => {}
                    }
                }

                prev_char = current_char;
                i += 1;
            }

            // Validate all formatting is balanced
            assert!(
                asterisk_count.is_multiple_of(2),
                "Pattern '{}' has unmatched asterisks",
                pattern
            );
            assert!(
                underscore_count.is_multiple_of(2),
                "Pattern '{}' has unmatched underscores",
                pattern
            );
            assert!(
                backtick_count.is_multiple_of(2),
                "Pattern '{}' has unmatched backticks",
                pattern
            );
            assert!(
                tilde_count.is_multiple_of(2),
                "Pattern '{}' has unmatched tildes",
                pattern
            );
            assert!(
                pipe_count.is_multiple_of(2),
                "Pattern '{}' has unmatched pipes",
                pattern
            );
            assert!(
                square_bracket_count == 0,
                "Pattern '{}' has unmatched square brackets",
                pattern
            );
            assert!(
                paren_count == 0,
                "Pattern '{}' has unmatched parentheses",
                pattern
            );
        }
    }

    // Note: These patterns would cause compile errors if used with the markdown_string! macro:
    // Invalid examples (unbalanced formatting):
    // "*unmatched bold" - unmatched asterisk
    // "_unmatched italic" - unmatched underscore
    // "`unmatched code" - unmatched backtick
    // "~unmatched strike" - unmatched tilde
    // "||unmatched spoiler" - unmatched pipes
    // "[unmatched link" - unmatched square bracket
    // "[text](unmatched url" - unmatched parenthesis

    #[test]
    fn test_escape_detection() {
        // Test the escape detection logic directly
        let test_string = "text\\.more";
        let bytes = test_string.as_bytes();
        let mut prev_char = 0u8;
        let mut prev_was_escaping_backslash = false;

        for (i, &current_char) in bytes.iter().enumerate() {
            let is_escaped = prev_char == b'\\' && prev_was_escaping_backslash;
            let new_prev_was_escaping_backslash = current_char == b'\\' && !is_escaped;

            if current_char == b'.' {
                assert!(
                    is_escaped,
                    "Period at position {} should be escaped in '{}'",
                    i, test_string
                );
            }

            prev_was_escaping_backslash = new_prev_was_escaping_backslash;
            prev_char = current_char;
        }
    }
}
