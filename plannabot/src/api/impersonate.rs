use crate::api;
use crate::api::context::BotCtx;
use crate::bot::teacher::TeacherCommand;
use crate::models::{Teacher, TelegramName, UserRole};
use teloxide::prelude::*;
use anyhow::Result;
use telluride::command::{CallbackKey, InlineKeyboardButtonPackedExt};
use telluride::data_store::UserProxy;
use telluride::markdown::MarkdownStringMessage;
use telluride::markdown_string;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub async fn impersonate(
    ctx: &BotCtx<TeacherCommand>,
    student_name: Option<TelegramName>,
) -> Result<()> {
    if ctx.state.get_impersonation(ctx.chat_id).await.is_some() {
        ctx.bot
            .send_message(
                ctx.chat_id,
                "You are already in impersonation mode. Use /quit first.",
            )
            .await?;
        return Ok(());
    }

    let Some(name) = student_name else {
        show_student_selection(ctx).await?;
        return Ok(());
    };

    ctx.state.exit_admin_mode(ctx.chat_id).await;

    match ctx.state.get_role(name.as_str()).await {
        Some(UserRole::Student(_)) => {
            ctx.state.impersonate(ctx.chat_id, name.clone()).await;
            ctx.bot
                .send_message(
                    ctx.chat_id,
                    format!(
                        "Now impersonating {}. All commands will behave as if you were that student. \
                         Use /help to see available commands, use /quit to exit",
                        name
                    ),
                )
                .await?;
        }
        Some(UserRole::Teacher(_)) => {
            if ctx.state.is_both_teacher_and_student(name.as_str()).await {
                ctx.state.impersonate(ctx.chat_id, name.clone()).await;
                ctx.bot
                    .send_message(
                        ctx.chat_id,
                        format!(
                            "Now impersonating {} (who is also a teacher). All commands will behave as if you were that student. \
                             Use /help to see available commands, use /quit to exit",
                            name
                        ),
                    )
                    .await?;
            } else {
                ctx.bot
                    .send_message(
                        ctx.chat_id,
                        format!("{} is a teacher, not a student.", name),
                    )
                    .await?;
            }
        }
        None => {
            ctx.bot
                .send_message(
                    ctx.chat_id,
                    format!("Student {} was not found in the spreadsheet.", name),
                )
                .await?;
        }
    }

    Ok(())
}

async fn show_student_selection(ctx: &BotCtx<TeacherCommand>) -> Result<()> {
    let student_names = ctx.state.get_student_names().await;

    if student_names.is_empty() {
        ctx.bot
            .send_message(
                ctx.chat_id,
                "No students are registered in the spreadsheet yet.",
            )
            .await?;
        return Ok(());
    }

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);

    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    for name in student_names {
        let label = name.to_string();
        let key = CallbackKey::pack(TeacherCommand::Impersonate(Some(name)), &user_proxy).await;
        let button = InlineKeyboardButton::callback_key(label, &key);
        buttons.push(vec![button]);
    }

    let keyboard = InlineKeyboardMarkup::new(buttons);
    ctx.bot
        .send_message(ctx.chat_id, "Select the student you want to impersonate:")
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

pub async fn help(ctx: &BotCtx<impl Send + Sync + Clone>) -> Result<()> {
    let text = markdown_string!(
        "*Available Commands \\(Impersonation Mode\\):*\n\n\
        /start \\- Exit impersonation and restart the bot\n\
        /help \\- Display this help message\n\
        /schedule \\- Show the impersonated student's planned lessons\n\
        /book \\- Book a lesson as the impersonated student\n\
        /quit \\- Exit impersonation mode"
    );
    ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
    Ok(())
}

pub async fn schedule(ctx: &BotCtx<impl Send + Sync + Clone>) -> Result<()> {
    let Some(student_name) = ctx.state.get_impersonation(ctx.chat_id).await else {
        return Ok(());
    };
    let Some(student) = ctx.state.get_student(&student_name).await else {
        ctx.state.clear_impersonation(ctx.chat_id).await;
        ctx.bot
            .send_message(
                ctx.chat_id,
                format!(
                    "Student {} was not found in the spreadsheet. \
                     Impersonation mode has been deactivated.",
                    student_name
                ),
            )
            .await?;
        return Ok(());
    };
    api::student::schedule(ctx, &student).await
}

pub async fn quit(ctx: &BotCtx<impl Send + Sync + Clone>, teacher: &Teacher) -> Result<()> {
    ctx.state.clear_impersonation(ctx.chat_id).await;
    ctx.bot
        .send_message(
            ctx.chat_id,
            format!(
                "Exited impersonation mode. You are back as {} (teacher).",
                teacher.telegram_name
            ),
        )
        .await?;
    Ok(())
}
