# Quick Start Guide for Plannabot

Get your Telegram bot up and running in 5 minutes!

## Step 1: Get a Bot Token (2 minutes)

1. Open Telegram and find [@BotFather](https://t.me/botfather)
2. Send `/newbot`
3. Give your bot a name (e.g., "Plannabot")
4. Give your bot a username (must end with "bot", e.g., "my_plannabot")
5. Copy your token (looks like: `123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11`)

## Step 2: Set Up Environment (1 minute)

```bash
# Create .env file from template
cp .env.example .env

# Edit .env and paste your token:
# TELOXIDE_TOKEN=your_token_here
```

Or just export the token directly:

```bash
export TELOXIDE_TOKEN="your_token_here"
```

## Step 3: Run the Bot (2 minutes)

```bash
# Install Rust if you don't have it
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build and run
cargo run
```

You should see:
```
Starting Plannabot...
```

## Step 4: Test Your Bot (1 minute)

1. Open Telegram
2. Search for your bot username
3. Start a chat with it
4. Try these commands:
   - Send `/start` → Bot responds with greeting
   - Send `/help` → Bot shows available commands
   - Send any message → Bot responds with "Hello!"

## Troubleshooting

### "Failed to get bot"
- Check your token is correct
- Make sure `TELOXIDE_TOKEN` environment variable is set

### "error: could not compile"
- Make sure you have Rust 1.56+: `rustc --version`
- Try `cargo clean && cargo build`

### Bot doesn't respond
- Check Telegram Privacy Mode is off in @BotFather settings
- Make sure bot is running with no errors

## Next Steps

- Add more commands to the bot (see README.md)
- Explore the `telluride` markdown features
- Check out the full documentation in README.md
