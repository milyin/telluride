When adding or modifying Telegram messages make sure that it's done accordingly to [TELLURIDE.md](TELLURIDE.md)
Keep [ARCHITECTURE.md](ARCHITECTURE.md) and [README.md](README.md) up to date

## Message formatting rule

ALL text sent to Telegram must be created with `markdown_string!` or `markdown_format!` macros — never pass a plain `&str` or `String` to `send_message` / `edit_message_text`. Use `send_markdown_message` / `edit_markdown_message_text` instead. This catches MarkdownV2 escaping errors at compile time.

Function parameters that carry message text must be typed as `MarkdownString`, not `&str` or `String`.
