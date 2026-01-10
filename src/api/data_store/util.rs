/// Encode a key to make it safe for use as a filename
///
/// This function encodes characters that are forbidden or problematic in filenames:
/// - `/` (path separator) -> `%2F`
/// - `\` (Windows path separator) -> `%5C`
/// - `:` (forbidden on Windows) -> `%3A`
/// - `*` (wildcard) -> `%2A`
/// - `?` (wildcard) -> `%3F`
/// - `"` (forbidden on Windows) -> `%22`
/// - `<` (forbidden on Windows) -> `%3C`
/// - `>` (forbidden on Windows) -> `%3E`
/// - `|` (forbidden on Windows) -> `%7C`
/// - `%` (our escape character) -> `%25`
/// - `.` at start (hidden files on Unix) -> `%2E`
/// - ` ` (space, can be problematic) -> `%20`
///
/// # Examples
/// ```
/// # use yoroolbot::storage::encode_key_to_filename;
/// assert_eq!(encode_key_to_filename("simple"), "simple");
/// assert_eq!(encode_key_to_filename("path/to/key"), "path%2Fto%2Fkey");
/// assert_eq!(encode_key_to_filename(".hidden"), "%2Ehidden");
/// assert_eq!(encode_key_to_filename("key:value"), "key%3Avalue");
/// ```
pub fn encode_key_to_filename(key: &str) -> String {
    let mut result = String::with_capacity(key.len());

    for (i, ch) in key.chars().enumerate() {
        match ch {
            '/' => result.push_str("%2F"),
            '\\' => result.push_str("%5C"),
            ':' => result.push_str("%3A"),
            '*' => result.push_str("%2A"),
            '?' => result.push_str("%3F"),
            '"' => result.push_str("%22"),
            '<' => result.push_str("%3C"),
            '>' => result.push_str("%3E"),
            '|' => result.push_str("%7C"),
            '%' => result.push_str("%25"),
            ' ' => result.push_str("%20"),
            '.' if i == 0 => result.push_str("%2E"), // Only encode leading dot
            _ => result.push(ch),
        }
    }

    result
}

/// Decode a filename back to the original key
///
/// This function reverses the encoding done by `encode_key_to_filename`.
///
/// # Examples
/// ```
/// # use yoroolbot::storage::{encode_key_to_filename, decode_filename_to_key};
/// assert_eq!(decode_filename_to_key("simple"), "simple");
/// assert_eq!(decode_filename_to_key("path%2Fto%2Fkey"), "path/to/key");
/// assert_eq!(decode_filename_to_key("%2Ehidden"), ".hidden");
/// assert_eq!(decode_filename_to_key("key%3Avalue"), "key:value");
///
/// // Round-trip test
/// let original = "path/to:key*.txt";
/// let encoded = encode_key_to_filename(original);
/// let decoded = decode_filename_to_key(&encoded);
/// assert_eq!(decoded, original);
/// ```
pub fn decode_filename_to_key(filename: &str) -> String {
    let mut result = String::with_capacity(filename.len());
    let mut chars = filename.chars();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            // Read next two characters as hex digits
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                } else {
                    // Invalid hex, keep the % and hex chars
                    result.push('%');
                    result.push_str(&hex);
                }
            } else {
                // Not enough characters after %, keep the %
                result.push('%');
                result.push_str(&hex);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for filename encoding/decoding functions

    #[test]
    fn test_encode_simple_key() {
        assert_eq!(encode_key_to_filename("simple"), "simple");
        assert_eq!(encode_key_to_filename("key123"), "key123");
        assert_eq!(encode_key_to_filename("my_key"), "my_key");
    }

    #[test]
    fn test_encode_forbidden_chars() {
        // Path separators
        assert_eq!(encode_key_to_filename("path/to/key"), "path%2Fto%2Fkey");
        assert_eq!(encode_key_to_filename("path\\to\\key"), "path%5Cto%5Ckey");

        // Windows forbidden characters
        assert_eq!(encode_key_to_filename("key:value"), "key%3Avalue");
        assert_eq!(encode_key_to_filename("file*.txt"), "file%2A.txt");
        assert_eq!(encode_key_to_filename("what?.txt"), "what%3F.txt");
        assert_eq!(encode_key_to_filename("name\"quoted\""), "name%22quoted%22");
        assert_eq!(encode_key_to_filename("left<right>"), "left%3Cright%3E");
        assert_eq!(encode_key_to_filename("cmd|pipe"), "cmd%7Cpipe");

        // Special characters
        assert_eq!(encode_key_to_filename("percent%sign"), "percent%25sign");
        assert_eq!(encode_key_to_filename("space key"), "space%20key");
    }

    #[test]
    fn test_encode_leading_dot() {
        assert_eq!(encode_key_to_filename(".hidden"), "%2Ehidden");
        assert_eq!(encode_key_to_filename(".gitignore"), "%2Egitignore");
        // Dots not at the start should not be encoded
        assert_eq!(encode_key_to_filename("file.txt"), "file.txt");
        assert_eq!(encode_key_to_filename("my.file.name"), "my.file.name");
    }

    #[test]
    fn test_encode_combined_forbidden_chars() {
        assert_eq!(
            encode_key_to_filename("path/to:key*.txt"),
            "path%2Fto%3Akey%2A.txt"
        );
        assert_eq!(
            encode_key_to_filename(".hidden/dir/file name.txt"),
            "%2Ehidden%2Fdir%2Ffile%20name.txt"
        );
    }

    #[test]
    fn test_decode_simple_key() {
        assert_eq!(decode_filename_to_key("simple"), "simple");
        assert_eq!(decode_filename_to_key("key123"), "key123");
        assert_eq!(decode_filename_to_key("my_key"), "my_key");
    }

    #[test]
    fn test_decode_forbidden_chars() {
        assert_eq!(decode_filename_to_key("path%2Fto%2Fkey"), "path/to/key");
        assert_eq!(decode_filename_to_key("path%5Cto%5Ckey"), "path\\to\\key");
        assert_eq!(decode_filename_to_key("key%3Avalue"), "key:value");
        assert_eq!(decode_filename_to_key("file%2A.txt"), "file*.txt");
        assert_eq!(decode_filename_to_key("what%3F.txt"), "what?.txt");
        assert_eq!(decode_filename_to_key("name%22quoted%22"), "name\"quoted\"");
        assert_eq!(decode_filename_to_key("left%3Cright%3E"), "left<right>");
        assert_eq!(decode_filename_to_key("cmd%7Cpipe"), "cmd|pipe");
        assert_eq!(decode_filename_to_key("percent%25sign"), "percent%sign");
        assert_eq!(decode_filename_to_key("space%20key"), "space key");
    }

    #[test]
    fn test_decode_leading_dot() {
        assert_eq!(decode_filename_to_key("%2Ehidden"), ".hidden");
        assert_eq!(decode_filename_to_key("%2Egitignore"), ".gitignore");
        assert_eq!(decode_filename_to_key("file.txt"), "file.txt");
    }

    #[test]
    fn test_decode_invalid_encoding() {
        // Invalid hex - should keep the % and chars
        assert_eq!(decode_filename_to_key("key%ZZvalue"), "key%ZZvalue");
        // Incomplete encoding at end - should keep the % and chars
        assert_eq!(decode_filename_to_key("key%2"), "key%2");
        assert_eq!(decode_filename_to_key("key%"), "key%");
    }

    #[test]
    fn test_round_trip_encoding() {
        let test_cases = vec![
            "simple",
            "path/to/key",
            "path\\to\\key",
            "key:value",
            "file*.txt",
            "what?.txt",
            "name\"quoted\"",
            "left<right>",
            "cmd|pipe",
            "percent%sign",
            "space key",
            ".hidden",
            ".hidden/path/to:file*.txt",
            "complex/path\\with:many*forbidden?chars\"<>|%and spaces",
        ];

        for original in test_cases {
            let encoded = encode_key_to_filename(original);
            let decoded = decode_filename_to_key(&encoded);
            assert_eq!(
                decoded, original,
                "Round-trip failed for '{}': encoded='{}', decoded='{}'",
                original, encoded, decoded
            );
        }
    }

    #[test]
    fn test_chat_id_encoding() {
        // Test realistic ChatId values
        let chat_ids = vec!["123456789", "-123456789", "0"];

        for chat_id in chat_ids {
            let encoded = encode_key_to_filename(chat_id);
            let decoded = decode_filename_to_key(&encoded);
            assert_eq!(decoded, chat_id);
            // Chat IDs should not require encoding (only digits and minus)
            assert_eq!(encoded, chat_id);
        }
    }

    #[test]
    fn test_encode_percent_sequences() {
        // Test that keys containing percent-encoded sequences are handled correctly
        // A key with literal "%25" should encode to "%2525"
        assert_eq!(encode_key_to_filename("%25"), "%2525");
        assert_eq!(encode_key_to_filename("test%25"), "test%2525");
        assert_eq!(encode_key_to_filename("%25test"), "%2525test");

        // Test round-trip
        let key_with_percent = "value%25";
        let encoded = encode_key_to_filename(key_with_percent);
        assert_eq!(encoded, "value%2525");
        let decoded = decode_filename_to_key(&encoded);
        assert_eq!(decoded, key_with_percent);

        // Test key with "%2F" literal string
        let key_with_encoded_slash = "path%2Fto";
        let encoded = encode_key_to_filename(key_with_encoded_slash);
        assert_eq!(encoded, "path%252Fto");
        let decoded = decode_filename_to_key(&encoded);
        assert_eq!(decoded, key_with_encoded_slash);
    }
}