use std::sync::Arc;
use telluride::{
    command::CallbackKey,
    data_store::{CommonProxy, DataStoreTrait, InMemStore, UserDataStoreTrait, UserProxy},
    markdown::MarkdownStringMessage,
    markdown_format, markdown_string,
};
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, Me, User},
    utils::command::BotCommands,
};
use serde::{Deserialize, Serialize};

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

/// Application-specific Action enum for callback handling.
/// Uses CallbackKey::pack/unpack for smart serialization.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
enum Action {
    ShowUser(u64),
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
        .branch(
            Update::filter_callback_query()
                .filter(|q: teloxide::types::CallbackQuery| {
                    q.data.as_ref().map_or(false, |data| 
                        CallbackKey::is_packed_data(data)
                    )
                })
                .endpoint(action_handler),
        )
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

    let storage = Arc::new(InMemStore::<String, Vec<String>>::new());
    // Use in-memory storage for global user registry
    // It will use UserId(0) as namespace in InMemStore
    let global_user_storage: Arc<dyn DataStoreTrait<UserId, User>> =
        Arc::new(CommonProxy::new(InMemStore::<UserId, User>::new()));

    // Per-user callback action storage (uses PackedValue keys)
    let callback_storage = Arc::new(InMemStore::<CallbackKey, Action>::new());

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![
            me,
            storage,
            callback_storage,
            global_user_storage
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

/// Helper to update user info in global registry
async fn update_user(user: &User, user_registry: &Arc<dyn DataStoreTrait<UserId, User>>) {
    // Update user registry using UserId directly as key
    user_registry.set(&user.id, user.clone()).await;
}

/// Helper to get user from message and update registry
async fn get_user_from_message(
    msg: &Message,
    user_registry: &Arc<dyn DataStoreTrait<UserId, User>>,
) -> ResponseResult<User> {
    match msg.from.as_ref() {
        Some(user) => {
            update_user(user, user_registry).await;
            Ok(user.clone())
        }
        None => {
            log::error!("Message from {:?} has no sender information", msg.chat.id);
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
    callback_storage: Arc<InMemStore<CallbackKey, Action>>,
    user_registry: Arc<dyn DataStoreTrait<UserId, User>>,
) -> ResponseResult<()> {
    log::info!("Received command: {:?} from {:?}", cmd, msg.chat.id);
    let user = get_user_from_message(&msg, &user_registry).await?;
    let user_id = user.id;

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
            let user_ids = user_registry.keys().await;
            if user_ids.is_empty() {
                bot.send_markdown_message(
                    msg.chat.id,
                    markdown_string!("No users have registered yet\\."),
                )
                .await?;
            } else {
                let user_proxy = UserProxy::new(callback_storage.clone(), user_id);
                let mut buttons = Vec::new();
                for uid in user_ids {
                    if let Some(u) = user_registry.get(&uid).await {
                        let label = format!("User {}", u.full_name());
                        // Use CallbackKey::pack for smart inline/storage routing
                        let key = CallbackKey::pack(&Action::ShowUser(uid.0), &user_proxy).await;
                        buttons.push(vec![
                            InlineKeyboardButton::callback(label, key.to_string()),
                        ]);
                    }
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
    storage: Arc<InMemStore<String, Vec<String>>>,
    user_registry: Arc<dyn DataStoreTrait<UserId, User>>,
) -> ResponseResult<()> {
    if let Some(text) = msg.text() {
        let user = get_user_from_message(&msg, &user_registry).await?;
        let user_id = user.id;

        // Save message to storage
        let mut messages = storage
            .get(user_id, &"messages".to_string())
            .await
            .unwrap_or_default();
        messages.push(text.to_string());
        storage
            .set(user_id, &"messages".to_string(), messages)
            .await;

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

/// Handler for actions (inline keyboard button presses)
/// Uses CallbackKey::unpack for smart action extraction
async fn action_handler(
    bot: Bot,
    q: CallbackQuery,
    callback_storage: Arc<InMemStore<CallbackKey, Action>>,
    storage: Arc<InMemStore<String, Vec<String>>>,
    user_registry: Arc<dyn DataStoreTrait<UserId, User>>,
) -> ResponseResult<()> {
    // Update user info
    update_user(&q.from, &user_registry).await;

    // Always answer the callback to remove the "loading" state
    bot.answer_callback_query(q.id.clone()).await?;

    // Extract action using CallbackKey::unpack - handles both inline and storage-backed actions
    let callback_data = match &q.data {
        Some(data) => data,
        None => {
            log::warn!("Callback query has no data");
            return Ok(());
        }
    };

    let user_proxy = UserProxy::new(callback_storage.clone(), q.from.id);
    let action: Action = match CallbackKey::unpack(callback_data, &user_proxy).await {
        Ok(action) => action,
        Err(e) => {
            log::warn!("Failed to unpack action from callback: {}", e);
            return Ok(());
        }
    };

    match action {
        Action::ShowUser(target_uid) => {
            let target_user_id = UserId(target_uid);
            let messages = storage
                .get(target_user_id, &"messages".to_string())
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
    }

    Ok(())
}
