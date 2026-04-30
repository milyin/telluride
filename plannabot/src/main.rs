use telluride::markdown::{MarkdownString, MarkdownStringMessage};
use telluride::markdown_string;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Supported commands:")]
enum Command {
    #[command(description = "display help")]
    Help,
    #[command(description = "start the bot")]
    Start,
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting Plannabot...");

    let bot = Bot::from_env();

    let handler = dptree::entry()
        // Handle commands (messages starting with /)
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .endpoint(command_handler),
        )
        // Handle regular text messages (non-command text)
        .branch(
            Update::filter_message()
                .filter(|msg: Message| msg.text().is_some())
                .endpoint(text_handler),
        )
        // Fallback for unhandled updates
        .endpoint(|update: Update| async move {
            log::debug!("Unhandled update type: {:?}", update.kind);
            respond(())
        });

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

/// Handler for bot commands (messages starting with /)
async fn command_handler(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    log::info!("Received command: {:?} from {:?}", cmd, msg.chat.id);

    match cmd {
        Command::Start => {
            let text = markdown_string!("Hello\\! 👋 Welcome to Plannabot\\!");
            bot.send_markdown_message(msg.chat.id, text).await?;
        }
        Command::Help => {
            let text = markdown_string!(
                "*Available Commands:*\n\n\
                /help \\- Display this help message\n\
                /start \\- Start the bot"
            );
            bot.send_markdown_message(msg.chat.id, text).await?;
        }
    }
    Ok(())
}

/// Handler for regular text messages (non-commands)
async fn text_handler(bot: Bot, msg: Message) -> ResponseResult<()> {
    if let Some(text) = msg.text() {
        log::info!("Received text message from {:?}: {}", msg.chat.id, text);

        let reply = MarkdownString::escape("Hello!");
        bot.send_markdown_message(msg.chat.id, reply).await?;
    }
    Ok(())
}
