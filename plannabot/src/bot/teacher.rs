use super::{Action, CommonCommand, TeacherCommand};
use crate::api;
use crate::models::Teacher;
use crate::state::BotState;
use anyhow::Result;
use std::sync::Arc;
use telluride::command::{CallbackKey, InlineKeyboardButtonPackedExt};
use telluride::data_store::{InMemStore, UserProxy};
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, UserId};

/// Route common commands for a teacher to the appropriate API functions.
pub async fn handle_command(
    bot: &Bot,
    msg: &Message,
    cmd: &CommonCommand,
    teacher: &Teacher,
    state: &Arc<BotState>,
) -> Result<()> {
    match cmd {
        CommonCommand::Start => api::teacher::start(bot, msg.chat.id, teacher, state).await,
        CommonCommand::Help => api::teacher::help(bot, msg.chat.id).await,
        CommonCommand::Schedule => api::teacher::schedule(bot, msg.chat.id, teacher, state).await,
    }
}

/// Route teacher-specific commands to the appropriate API functions.
///
/// `user_id` is the Telegram user ID of the teacher, used to namespace the
/// per-user callback storage so that each teacher's button actions are
/// isolated from other teachers' pending selections.
pub async fn handle_teacher_command(
    bot: &Bot,
    msg: &Message,
    cmd: &TeacherCommand,
    teacher: &Teacher,
    state: &Arc<BotState>,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Action>>,
) -> Result<()> {
    match cmd {
        TeacherCommand::Impersonate => {
            show_student_selection(bot, msg.chat.id, user_id, callback_storage, state).await
        }
        TeacherCommand::Quit => api::teacher::quit(bot, msg.chat.id, teacher, state).await,
    }
}

/// Send an inline keyboard listing every registered student.
///
/// Each button is labelled `@<telegram_name>` and carries a packed
/// [`Action::ImpersonateStudent`] callback.  When the teacher presses a
/// button, [`crate::bot::callback_action_handler`] unpacks the action and
/// calls [`crate::api::teacher::impersonate`] with the chosen student name.
async fn show_student_selection(
    bot: &Bot,
    chat_id: ChatId,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Action>>,
    state: &Arc<BotState>,
) -> Result<()> {
    let student_names = state.get_student_names().await;

    if student_names.is_empty() {
        bot.send_message(
            chat_id,
            "No students are registered in the spreadsheet yet.",
        )
        .await?;
        return Ok(());
    }

    // Each callback key is scoped to this teacher's user_id so that multiple
    // teachers using the bot simultaneously don't share each other's keys.
    let user_proxy = UserProxy::new(callback_storage, user_id);

    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    for name in student_names {
        let label = format!("@{}", name);
        let key = CallbackKey::pack(Action::ImpersonateStudent(name), &user_proxy).await;
        let button = InlineKeyboardButton::callback_key(label, &key);
        buttons.push(vec![button]);
    }

    let keyboard = InlineKeyboardMarkup::new(buttons);
    bot.send_message(chat_id, "Select the student you want to impersonate:")
        .reply_markup(keyboard)
        .await?;

    Ok(())
}
