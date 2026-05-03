use crate::models::UserRole;
use crate::state::BotState;
use anyhow::Result;
use std::sync::Arc;
use telluride::command::{CallbackKey, InlineKeyboardButtonPackedExt};
use telluride::data_store::{InMemStore, UserProxy};
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, UserId};

use crate::bot::Action;

/// Handle the /impersonate command for a teacher.
///
/// If `student_name` is `None`, shows a selection UI with all available students.
/// If `student_name` is `Some`, attempts to impersonate that specific student.
///
/// `user_id` is the Telegram user ID of the teacher, used to namespace the
/// per-user callback storage so that each teacher's button actions are
/// isolated from other teachers' pending selections.
pub async fn impersonate(
    bot: &Bot,
    chat_id: ChatId,
    student_name: Option<&str>,
    state: &Arc<BotState>,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Action>>,
) -> Result<()> {
    if state.get_impersonation(chat_id).await.is_some() {
        bot.send_message(
            chat_id,
            "You are already in impersonation mode. Use /quit first.",
        )
        .await?;
        return Ok(());
    }

    let Some(username) = student_name else {
        // No student specified: show selection UI
        show_student_selection(bot, chat_id, user_id, callback_storage, state).await?;
        return Ok(());
    };

    let normalised = username.trim_start_matches('@').to_lowercase();
    if normalised.is_empty() {
        bot.send_message(chat_id, "Usage: /impersonate <student_username>")
            .await?;
        return Ok(());
    }

    match state.get_role(&normalised).await {
        Some(UserRole::Student(_)) => {
            state.impersonate(chat_id, normalised.clone()).await;
            bot.send_message(
                chat_id,
                format!(
                    "Now impersonating @{}. All commands will behave as if you were that student. \
                     Use /help to see available commands, use /quit to exit",
                    normalised
                ),
            )
            .await?;
        }
        Some(UserRole::Teacher(_)) => {
            // Check if this teacher is also a student (dual role)
            if state.is_both_teacher_and_student(&normalised).await {
                state.impersonate(chat_id, normalised.clone()).await;
                bot.send_message(
                    chat_id,
                    format!(
                        "Now impersonating @{} (who is also a teacher). All commands will behave as if you were that student. \
                         Use /help to see available commands, use /quit to exit",
                        normalised
                    ),
                )
                .await?;
            } else {
                bot.send_message(
                    chat_id,
                    format!("@{} is a teacher, not a student.", normalised),
                )
                .await?;
            }
        }
        None => {
            bot.send_message(
                chat_id,
                format!("Student @{} was not found in the spreadsheet.", normalised),
            )
            .await?;
        }
    }

    Ok(())
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
