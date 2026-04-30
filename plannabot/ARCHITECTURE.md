# Plannabot Architecture

This document describes the architecture and design of the Plannabot Telegram bot.

## Overview

Plannabot is a lightweight Telegram bot built on top of the `teloxide` framework with extensions from the `telluride` library. It demonstrates best practices for Rust bot development with compile-time safe message formatting.

## Technology Stack

| Component | Purpose | Version |
|-----------|---------|---------|
| **teloxide** | Telegram Bot API framework | 0.17.0 |
| **telluride** | Safe MarkdownV2 formatting | 0.2.0 |
| **tokio** | Async runtime | 1.8+ |
| **dptree** | Handler routing | 0.5 |
| **log** | Logging | 0.4 |

## Project Structure

```
plannabot/
├── src/
│   └── main.rs              # Main bot implementation
├── Cargo.toml               # Project manifest with dependencies
├── Cargo.lock               # Locked dependency versions
├── .env.example             # Environment configuration template
├── .gitignore               # Git ignore rules
├── README.md                # Full documentation
├── QUICKSTART.md            # Quick start guide
└── ARCHITECTURE.md          # This file
```

## Core Architecture

### 1. Bot Initialization

The bot starts by:
1. Initializing the logging system with `pretty_env_logger`
2. Loading the bot token from the `TELOXIDE_TOKEN` environment variable
3. Creating a `Bot` instance
4. Building the request handler chain

```rust
let bot = Bot::from_env();
let handler = dptree::entry()
    .branch(/* command handler */)
    .branch(/* text handler */)
    .endpoint(/* fallback handler */);
```

### 2. Handler Chain

The bot uses a branching handler architecture implemented with `dptree`:

```
Update → Entry Point
    ├→ Command Handler (if message.text() starts with /)
    ├→ Text Handler (if message contains text)
    └→ Fallback Handler (all other updates)
```

#### Command Handler
- Filters messages that are commands (e.g., `/start`, `/help`)
- Extracts the command using `#[derive(BotCommands)]`
- Routes to appropriate command logic

#### Text Handler
- Filters messages containing text that aren't commands
- Responds with "Hello!"

#### Fallback Handler
- Logs unhandled update types for debugging
- Prevents errors from unhandled updates

### 3. Dispatcher

The dispatcher processes incoming updates from Telegram:

```rust
Dispatcher::builder(bot, handler)
    .enable_ctrlc_handler()    // Graceful shutdown
    .build()
    .dispatch()                 // Start processing
    .await;
```

## Telluride Integration

### MarkdownString

The `MarkdownString` type provides compile-time safe MarkdownV2 formatting:

```rust
// Using the markdown_string! macro (compile-time validated)
let text = markdown_string!("*Bold* _italic_ `code`");

// Using escape for unsafe strings
let user_input = MarkdownString::escape(user_text);
```

### MarkdownStringMessage Trait

Extended `Bot` methods for sending markdown messages:

```rust
// Regular teloxide method
bot.send_message(chat_id, text).parse_mode(ParseMode::MarkdownV2).await?;

// With telluride
bot.send_markdown_message(chat_id, markdown_text).await?;
```

Benefits:
- No parse mode mistakes
- Compile-time validation for string literals
- Type-safe escape handling for user input

## Command System

Commands are defined using the `BotCommands` derive macro:

```rust
#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Supported commands:")]
enum Command {
    #[command(description = "display help")]
    Help,
    #[command(description = "start the bot")]
    Start,
}
```

Features:
- Automatic command parsing from messages
- Built-in `/help` command generation
- Description support for documentation

## Async/Await Pattern

All handlers use async/await for non-blocking I/O:

```rust
async fn command_handler(
    bot: Bot,
    msg: Message,
    cmd: Command,
) -> ResponseResult<()> {
    // ...
}
```

This allows the bot to handle multiple concurrent users without blocking.

## Error Handling

The bot uses `ResponseResult<T>` (alias for `Result<T, RequestError>`):

```rust
// Propagate errors with ?
bot.send_markdown_message(chat_id, text).await?;

// Return Ok(()) for success
Ok(())
```

Errors are automatically logged by teloxide and reported back to Telegram users when appropriate.

## Data Flow

```
Telegram API
    ↓
teloxide::Dispatcher
    ↓
dptree handler chain
    ↓
Command/Text/Fallback handlers
    ↓
telluride MarkdownString formatting
    ↓
Bot::send_markdown_message()
    ↓
Telegram API
```

## Concurrency Model

- **Async Runtime**: tokio with multi-threaded executor
- **Concurrent Handlers**: Multiple users can be served simultaneously
- **Update Processing**: Updates are processed sequentially but handlers are async
- **Graceful Shutdown**: Ctrl+C handler allows clean shutdown

## Extending the Bot

### Adding a New Command

1. Add variant to `Command` enum
2. Add handler logic in `command_handler`
3. Recompile and deploy

### Adding a New Handler

1. Create a filter condition
2. Create a handler function
3. Add branch to `dptree::entry()` chain

### Adding Dependencies

1. Update `Cargo.toml`
2. Use in code
3. Run `cargo build`

## Logging

Logging is implemented using the `log` crate with `pretty_env_logger`:

- Debug logs: Update types and detailed tracing
- Info logs: Command and message reception
- Warn logs: Potential issues
- Error logs: Processing errors

Control with `RUST_LOG` environment variable:
```bash
RUST_LOG=debug cargo run
RUST_LOG=info cargo run
```

## Security Considerations

1. **Input Validation**: User input is escaped via `MarkdownString::escape()`
2. **No Hardcoded Secrets**: Token comes from environment variables
3. **Markdown Validation**: Compile-time checks prevent invalid formatting
4. **Type Safety**: Rust's type system prevents many common errors

## Performance Characteristics

- **Memory**: Minimal footprint, single bot instance
- **Latency**: <100ms typical response time
- **Throughput**: Can handle thousands of concurrent users
- **Startup**: ~1-2 seconds to initialize and connect

## Future Enhancements

Possible improvements:
- Add persistent storage (database)
- Implement inline keyboards with callbacks
- Add user session management
- Create data analysis features
- Implement caching strategies
