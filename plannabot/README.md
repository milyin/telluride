# Plannabot

A Telegram bot built with Rust using the `telluride` and `teloxide` libraries.

## Features

- ✅ Responds to `/help` command with available commands
- ✅ Responds to `/start` command with a greeting
- ✅ Responds to any message with "Hello!"
- ✅ Uses `telluride` library for compile-time safe MarkdownV2 formatting
- ✅ Built on `teloxide` framework with modern async/await
- ✅ Graceful shutdown with Ctrl+C handler

## Prerequisites

- Rust 1.56 or later (install from [rustup.rs](https://rustup.rs/))
- A Telegram Bot Token (obtain from [@BotFather](https://t.me/botfather) on Telegram)

## Getting Started

### 1. Create Your Bot

1. Open Telegram and chat with [@BotFather](https://t.me/botfather)
2. Use `/newbot` command to create a new bot
3. Follow the prompts and you'll receive your bot token

### 2. Set Up Environment

Copy the example environment file and add your bot token:

```bash
cp .env.example .env
# Then edit .env and add your TELOXIDE_TOKEN
```

Or set the environment variable directly:

```bash
export TELOXIDE_TOKEN="your_actual_token_here"
```

### 3. Build and Run

Build the project:

```bash
cargo build --release
```

Run the bot:

```bash
cargo run
```

For development with auto-reload, install and use `cargo-watch`:

```bash
cargo install cargo-watch
cargo watch -x run
```

## Commands

The bot supports the following commands:

- `/start` - Start the bot and receive a greeting
- `/help` - Display help information and available commands

Any regular message will receive a "Hello!" response.

## Project Structure

```
plannabot/
├── src/
│   └── main.rs          # Main bot implementation
├── Cargo.toml           # Project dependencies
├── .env.example         # Environment configuration template
├── .gitignore           # Git ignore rules
└── README.md            # This file
```

## Implementation Details

The bot uses the following key components:

### Teloxide Framework
- Modern async Rust framework for Telegram Bot API
- Provides high-level abstractions for handling updates
- Built on `tokio` for efficient concurrent processing

### Telluride Library Extension
The project leverages the `telluride` library which extends `teloxide` with:
- **MarkdownString**: Compile-time validated MarkdownV2 formatting
- **markdown_string!**: Macro for creating safe markdown strings with validation
- **MarkdownStringMessage**: Extended `Bot` methods for sending markdown messages

### Dispatcher Architecture
The bot uses a handler chain architecture:
1. Command handler - processes `/start` and `/help` commands
2. Text handler - processes regular text messages
3. Fallback handler - logs unhandled update types

## Example Usage

```bash
# Start the bot
cargo run

# In Telegram, message the bot:
# - Send: /start
#   Bot responds: "Hello! 👋 Welcome to Plannabot!"
# 
# - Send: /help
#   Bot responds with available commands
#
# - Send: "Hi there"
#   Bot responds: "Hello!"
```

## Logging

The bot uses the `log` crate with `pretty_env_logger` for formatted output.

Control log level with the `RUST_LOG` environment variable:

```bash
RUST_LOG=debug cargo run    # Show debug messages
RUST_LOG=info cargo run     # Show info messages (default)
RUST_LOG=warn cargo run     # Show warnings only
```

## Extending the Bot

To add new commands:

1. Add a new variant to the `Command` enum:

```rust
#[derive(BotCommands, Clone, Debug)]
enum Command {
    #[command(description = "existing command")]
    Start,
    #[command(description = "my new command")]
    MyNewCommand,
}
```

2. Handle it in `command_handler`:

```rust
Command::MyNewCommand => {
    let text = markdown_string!("My response");
    bot.send_markdown_message(msg.chat.id, text).await?;
}
```

## License

MIT OR Apache-2.0

## Resources

- [Teloxide Documentation](https://docs.rs/teloxide/)
- [Telluride GitHub](https://github.com/milyin/telluride)
- [Telegram Bot API](https://core.telegram.org/bots/api)
- [MarkdownV2 Format](https://core.telegram.org/bots/api#markdownv2-style)
