# TELLURIDE.md — Message Formatting Guide

This document describes how to format and send Telegram messages using the `telluride` library.

## Overview

All Telegram messages sent from this bot **must use the `telluride` library** for compile-time-safe MarkdownV2 formatting.

Telluride provides two main macros:
- **`markdown_string!()`** — for constant text with no user input
- **`markdown_format!()`** — for messages containing dynamic user data (automatically escaped)

## Telluride Macros

### `markdown_string!()`

Builds a `MarkdownStringMessage` from a compile-time string literal.
Used for constant text with no user input.

```rust
let text = markdown_string!("📅 No planned lessons found\\.");
```

**Escaping rule:** All MarkdownV2 special characters must be backslash-escaped:
- `\` → `\\` (literal backslash)
- `*` → `\*` (prevent bold)
- `_` → `\_` (prevent italic)
- `[` → `\[` (prevent link)
- `]` → `\]` (prevent link)
- `(` → `\(` (prevent link)
- `)` → `\)` (prevent link)
- `~` → `\~` (prevent strikethrough)
- `` ` `` → `` \` `` (prevent code)
- `` ` `` → `` ` `` (code block delimiter, no escaping needed inside blocks)
- `!` → `\!` (in specific contexts)
- `.` → `\.` (prevent unintended formatting)
- `+` → `\+` (in lists)
- `-` → `\-` (in lists)
- `=` → `\=` (in headers)
- `>` → `\>` (in quotes)
- `|` → `\|` (in tables)

### `markdown_format!()`

Builds a `MarkdownStringMessage` from a format string with interpolated values.
Used for messages containing dynamic user data.

```rust
let greeting = markdown_format!(
    "Hello, {}\\! Use /help to see available commands\\.",
    student.name.as_str()
);
```

The `{}` placeholders are filled with the arguments' display values and automatically escaped.
All compile-time string literals (including format braces and escape sequences) follow the same escaping rules as `markdown_string!()`.

## Building Complex Messages

Use `push()` to concatenate multiple `MarkdownStringMessage` values:

```rust
let mut text = markdown_string!("👋 *Welcome to Plannabot\\!*\n\n");
let greeting = markdown_format!(
    "Hello, {}\\! Use /help to see available commands\\.",
    student.name.as_str()
);
text.push(&greeting);
bot.send_markdown_message(msg.chat.id, text).await?;
```

## Sending Messages

Use `bot.send_markdown_message()` (injected by teloxide and telluride) to send a `MarkdownStringMessage`:

```rust
bot.send_markdown_message(msg.chat.id, text).await?;
```

## Key Benefits

1. **Compile-time validation** — invalid MarkdownV2 is caught at build time, not at runtime.
2. **Automatic escaping** — user-provided data in `markdown_format!()` is safe from injection; you don't have to escape it manually.
3. **Readability** — the source code visually reflects the intended formatting.

## Imports

When writing message code, import `telluride` at the top:

```rust
use telluride::markdown::MarkdownStringMessage;
use telluride::{markdown_format, markdown_string};
```

## Testing Messages

To verify a message displays correctly:

1. Build the project: `cargo build --release`
2. Run the bot: `cargo run --release`
3. Send the command from a Telegram client
4. Visually inspect the formatting in Telegram

If MarkdownV2 escaping is incorrect, Telegram will show `Bad Request: message text is empty` or `Bad Request: message text not specified` errors. Check the server logs for the full error from Telegram.

## Examples

### Constant Message

```rust
let text = markdown_string!("📅 No planned lessons found\\.");
bot.send_markdown_message(msg.chat.id, text).await?;
```

### Message with User Data

```rust
let text = markdown_format!(
    "*Hello, {}\\!*\n\nYour timezone is set to \\`{}\\`\\.",
    student.name.as_str(),
    student.timezone.as_str()
);
bot.send_markdown_message(msg.chat.id, text).await?;
```

### Multi-part Message

```rust
let mut text = markdown_string!("*Your Lessons:*\n\n");

for lesson in lessons {
    let line = markdown_format!(
        "📅 {} with {}\n",
        lesson.date.as_str(),
        lesson.teacher_name.as_str()
    );
    text.push(&line);
}

bot.send_markdown_message(msg.chat.id, text).await?;
```

## Related Files

- **[ARCHITECTURE.md](ARCHITECTURE.md)** — code organization, where messages are located
- **[CLAUDE.md](CLAUDE.md)** — checklist for message changes
- **`Cargo.toml`** — telluride dependency: `telluride = { path = ".." }`
