use telluride::{markdown::MarkdownStringMessage, markdown_format, markdown_string};
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, Me},
    utils::command::BotCommands,
};

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Supported commands:")]
enum Command {
    #[command(description = "start the bot")]
    Start,
    #[command(description = "display help")]
    Help,
    #[command(description = "show inline keyboard")]
    Menu,
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting simple_bot...");

    let bot = Bot::from_env();
    let me = bot.get_me().await.unwrap();

    let handler = dptree::entry()
        // Global update logger - see everything coming in
        .inspect(|update: Update| {
            log::debug!("Received update: {:?}", update.id);
        })
        // Handle commands (messages starting with /)
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .endpoint(command_handler),
        )
        // Handle callback queries (inline keyboard button presses)
        .branch(Update::filter_callback_query().endpoint(callback_handler))
        // Handle new chat members (when bot is added to a group)
        .branch(
            Update::filter_message()
                .filter(|msg: Message| {
                    msg.new_chat_members()
                        .map(|m| !m.is_empty())
                        .unwrap_or(false)
                })
                .endpoint(new_chat_members_handler),
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

    // Other Telegram update/message types that can be handled:
    //
    // Message content types (via Message methods):
    // - msg.photo()         - Photo messages
    // - msg.video()         - Video messages
    // - msg.audio()         - Audio files
    // - msg.voice()         - Voice messages
    // - msg.video_note()    - Video notes (round videos)
    // - msg.document()      - Document/file messages
    // - msg.animation()     - GIF animations
    // - msg.sticker()       - Stickers
    // - msg.contact()       - Shared contacts
    // - msg.location()      - Location messages
    // - msg.venue()         - Venue messages
    // - msg.poll()          - Polls
    // - msg.dice()          - Dice/random value messages
    // - msg.game()          - Games
    // - msg.invoice()       - Payment invoices
    // - msg.successful_payment() - Successful payment notifications
    //
    // Update types (via Update::filter_*):
    // - Update::filter_message()           - Regular messages
    // - Update::filter_edited_message()    - Edited messages
    // - Update::filter_channel_post()      - Channel posts
    // - Update::filter_edited_channel_post() - Edited channel posts
    // - Update::filter_callback_query()    - Inline keyboard callbacks
    // - Update::filter_inline_query()      - Inline mode queries
    // - Update::filter_chosen_inline_result() - Chosen inline results
    // - Update::filter_shipping_query()    - Shipping queries (payments)
    // - Update::filter_pre_checkout_query() - Pre-checkout queries (payments)
    // - Update::filter_poll()              - Poll state updates
    // - Update::filter_poll_answer()       - Poll answer updates
    // - Update::filter_my_chat_member()    - Bot's chat member status changes
    // - Update::filter_chat_member()       - Other chat member status changes
    // - Update::filter_chat_join_request() - Join request updates

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![me])
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
            bot.send_markdown_message(
                msg.chat.id,
                markdown_string!("Welcome\\! Use /help to see available commands\\."),
            )
            .await?;
        }
        Command::Help => {
            bot.send_markdown_message(
                msg.chat.id,
                markdown_format!("{}", Command::descriptions().to_string()),
            )
            .await?;
        }
        Command::Menu => {
            let keyboard = make_keyboard();
            bot.send_markdown_message(msg.chat.id, markdown_string!("Choose an option:"))
                .reply_markup(keyboard)
                .await?;
        }
    }
    Ok(())
}

/// Handler for regular text messages (non-commands)
async fn text_handler(bot: Bot, msg: Message, me: Me) -> ResponseResult<()> {
    if let Some(text) = msg.text() {
        if !msg.chat.is_private() {
            let text_lower = text.to_lowercase();
            let username_lower = me.username.as_ref().unwrap().to_lowercase();
            let firstname_lower = me.first_name.to_lowercase();

            if text_lower.contains(&username_lower) || text_lower.contains(&firstname_lower) {
                log::info!("Responding to mention in group {:?}: {}", msg.chat.id, text);
                let mut req =
                    bot.send_markdown_message(msg.chat.id, markdown_format!("You said: {}", text));
                if let Some(thread_id) = msg.thread_id {
                    req = req.message_thread_id(thread_id);
                }
                req.await?;
            } else {
                log::info!(
                    "Ignoring message in group {:?} not mentioning bot. Text: \"{}\". (Note: Bot Privacy Mode might hide non-mentions/non-commands)",
                    msg.chat.id,
                    text
                );
            }
        } else {
            log::info!(
                "Responding to private message from {:?}: {}",
                msg.chat.id,
                text
            );
            let mut req =
                bot.send_markdown_message(msg.chat.id, markdown_format!("You said: {}", text));
            if let Some(thread_id) = msg.thread_id {
                req = req.message_thread_id(thread_id);
            }
            req.await?;
        }
    }
    Ok(())
}

/// Handler for new chat members
async fn new_chat_members_handler(bot: Bot, msg: Message, me: Me) -> ResponseResult<()> {
    if let Some(members) = msg.new_chat_members() {
        for member in members {
            if member.id == me.id {
                log::info!("Bot added to group {:?}", msg.chat.id);
                bot.send_markdown_message(
                    msg.chat.id,
                    markdown_string!("Hello\\! Thanks for adding me\\."),
                )
                .await?;
                return Ok(());
            }
        }
    }
    Ok(())
}

/// Handler for callback queries (inline keyboard button presses)
async fn callback_handler(bot: Bot, q: CallbackQuery) -> ResponseResult<()> {
    // Always answer the callback to remove the "loading" state
    bot.answer_callback_query(q.id.clone()).await?;

    if let Some(data) = &q.data {
        log::info!("Received callback query from {:?}: {}", q.from.id, data);
        let text = markdown_format!("You pressed: {}", data);

        // Send response - either edit the original message or send a new one
        if let Some(msg) = q.message {
            bot.edit_markdown_message_text(msg.chat().id, msg.id(), text)
                .await?;
        } else if let Some(id) = q.inline_message_id {
            bot.edit_markdown_message_text_inline(&id, text).await?;
        }
    }

    Ok(())
}

/// Creates an inline keyboard with sample buttons
fn make_keyboard() -> InlineKeyboardMarkup {
    let buttons = vec![
        vec![
            InlineKeyboardButton::callback("Option 1", "option_1"),
            InlineKeyboardButton::callback("Option 2", "option_2"),
        ],
        vec![InlineKeyboardButton::callback("Option 3", "option_3")],
    ];
    InlineKeyboardMarkup::new(buttons)
}
