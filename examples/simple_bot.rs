use serde::{Deserialize, Serialize};
use std::sync::Arc;
use telluride::{
    command::{InlineKeyboardButtonPackedExt, PackedValue},
    data_store::{InMemStore, UserDataStoreTrait, UserProxy},
    markdown::MarkdownStringMessage,
    markdown_format, markdown_string,
};
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
    #[command(description = "list saved messages from users")]
    Messages,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
struct MyCallbackData {
    action: String,
    value: String,
}

impl std::str::FromStr for MyCallbackData {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Fallback for simple string callbacks if needed
        Ok(MyCallbackData {
            action: s.to_string(),
            value: "0".to_string(),
        })
    }
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

    let storage = Arc::new(InMemStore::<Vec<String>>::new());
    let callback_storage = Arc::new(InMemStore::<MyCallbackData>::new());

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![me, storage, callback_storage])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

/// Helper to get user ID from message, reporting error if not present
fn get_user_id(msg: &Message) -> ResponseResult<UserId> {
    match msg.from.as_ref().map(|u| u.id) {
        Some(id) => Ok(id),
        None => {
            log::error!("Message from {:?} has no sender information", msg.chat.id);
            // We return an error to stop processing
            Err(teloxide::RequestError::Io(Arc::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "User information missing",
            ))))
        }
    }
}

/// Handler for bot commands (messages starting with /)
async fn command_handler(
    bot: Bot,
    msg: Message,
    cmd: Command,
    storage: Arc<InMemStore<Vec<String>>>,
    callback_storage: Arc<InMemStore<MyCallbackData>>,
) -> ResponseResult<()> {
    log::info!("Received command: {:?} from {:?}", cmd, msg.chat.id);
    let user_id = get_user_id(&msg)?;

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
        Command::Messages => {
            let users = storage.users().await;
            if users.is_empty() {
                bot.send_markdown_message(
                    msg.chat.id,
                    markdown_string!("No users have saved messages yet\\."),
                )
                .await?;
            } else {
                let user_proxy = UserProxy::new(callback_storage.clone(), user_id);
                let mut buttons = Vec::new();
                for uid in users {
                    let label = format!("User {}", uid);
                    buttons.push(vec![InlineKeyboardButton::callback_packed(
                        label,
                        PackedValue::pack(
                            &MyCallbackData {
                                action: "show_user".to_string(),
                                value: uid.to_string(),
                            },
                            &user_proxy,
                        )
                        .await,
                    )]);
                }
                let keyboard = InlineKeyboardMarkup::new(buttons);
                bot.send_markdown_message(
                    msg.chat.id,
                    markdown_string!("Select a user to see their messages:"),
                )
                .reply_markup(keyboard)
                .await?;
            }
        }
    }
    Ok(())
}

/// Handler for regular text messages (non-commands)
async fn text_handler(
    bot: Bot,
    msg: Message,
    me: Me,
    storage: Arc<InMemStore<Vec<String>>>,
) -> ResponseResult<()> {
    if let Some(text) = msg.text() {
        let user_id = get_user_id(&msg)?;

        // Save message to storage
        let mut messages = storage.get(user_id, "messages").await.unwrap_or_default();
        messages.push(text.to_string());
        storage.set(user_id, "messages", messages).await;

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
async fn callback_handler(
    bot: Bot,
    q: CallbackQuery,
    storage: Arc<InMemStore<Vec<String>>>,
    callback_storage: Arc<InMemStore<MyCallbackData>>,
) -> ResponseResult<()> {
    // Always answer the callback to remove the "loading" state
    bot.answer_callback_query(q.id.clone()).await?;

    if let Some(data_str) = &q.data {
        let user_id = q.from.id;
        let user_store = UserProxy::new(callback_storage.clone(), user_id);

        if let Ok(packed) = PackedValue::new(data_str) {
            if let Some(data) = packed.unpack::<MyCallbackData>(&user_store).await {
                if data.action == "show_user" {
                    if let Ok(target_uid) = data.value.parse::<u64>() {
                        let target_user_id = UserId(target_uid);
                        let messages = storage
                            .get(target_user_id, "messages")
                            .await
                            .unwrap_or_default();

                        let text = if messages.is_empty() {
                            markdown_format!(
                                "No messages saved for user {}",
                                target_user_id.0.to_string()
                            )
                        } else {
                            let list = messages
                                .iter()
                                .enumerate()
                                .map(|(i, m)| format!("{}. {}", i + 1, m))
                                .collect::<Vec<_>>()
                                .join("\n");
                            markdown_format!(
                                "Saved messages for user {}:\n{}",
                                target_user_id.0.to_string(),
                                list
                            )
                        };

                        if let Some(msg) = q.message {
                            bot.edit_markdown_message_text(msg.chat().id, msg.id(), text)
                                .await?;
                        } else if let Some(id) = q.inline_message_id {
                            bot.edit_markdown_message_text_inline(&id, text).await?;
                        }
                    }
                } else {
                    log::info!("Unhandled callback action: {}", data.action);
                }
            }
        }
    }

    Ok(())
}
