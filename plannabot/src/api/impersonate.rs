use crate::api;
use crate::api::context::BotCtx;
use crate::api::traits::{ImpersonateCommand, ImpersonateParams};
use crate::models::Teacher;
use anyhow::Result;
use telluride::command::{CallbackBitcode, CallbackKey, InlineKeyboardButtonPackedExt};
use telluride::data_store::UserProxy;
use telluride::markdown::MarkdownStringMessage;
use telluride::{markdown_format, markdown_string};
use teloxide::payloads::SendMessageSetters;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub async fn impersonate<Cmd: ImpersonateCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    params: &str,
) -> Result<()> {
    if ctx.state.get_impersonation(ctx.chat_id).await.is_some() {
        ctx.bot
            .send_markdown_message(
                ctx.chat_id,
                markdown_string!("You are already in impersonation mode\\. Use /quit first\\."),
            )
            .await?;
        return Ok(());
    }

    let params: ImpersonateParams = params.parse()?;
    match params {
        ImpersonateParams::I0 => show_role_selection(ctx).await,
        ImpersonateParams::Student(None) => show_student_selection(ctx).await,
        ImpersonateParams::Student(Some(name)) => {
            match ctx.state.get_student(&name).await {
                Some(_) => {
                    ctx.state.exit_admin_mode(ctx.chat_id).await;
                    ctx.state.impersonate(ctx.chat_id, name.clone()).await;
                    let text = markdown_format!(
                        "Now impersonating {}\\. All commands will behave as if you were that student\\. \
                         Use /help to see available commands, use /quit to exit",
                        name.to_string()
                    );
                    ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
                }
                None => {
                    let text = markdown_format!(
                        "Student {} was not found in the spreadsheet\\.",
                        name.to_string()
                    );
                    ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
                }
            }
            Ok(())
        }
        ImpersonateParams::Teacher(_) => {
            ctx.bot
                .send_markdown_message(
                    ctx.chat_id,
                    markdown_string!("Impersonating a teacher is not implemented yet\\."),
                )
                .await?;
            Ok(())
        }
    }
}

async fn show_role_selection<Cmd: ImpersonateCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
) -> Result<()> {
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);

    let student_key = CallbackKey::pack(
        Cmd::impersonate(ImpersonateParams::Student(None)),
        &user_proxy,
    )
    .await;
    let teacher_key = CallbackKey::pack(
        Cmd::impersonate(ImpersonateParams::Teacher(None)),
        &user_proxy,
    )
    .await;

    let buttons = vec![
        vec![InlineKeyboardButton::callback_key("Student".to_string(), &student_key)],
        vec![InlineKeyboardButton::callback_key("Teacher".to_string(), &teacher_key)],
    ];

    let keyboard = InlineKeyboardMarkup::new(buttons);
    ctx.bot
        .send_markdown_message(ctx.chat_id, markdown_string!("Select role to impersonate:"))
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

async fn show_student_selection<Cmd: ImpersonateCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
) -> Result<()> {
    let student_names = ctx.state.get_student_names().await;

    if student_names.is_empty() {
        ctx.bot
            .send_markdown_message(
                ctx.chat_id,
                markdown_string!("No students are registered in the spreadsheet yet\\."),
            )
            .await?;
        return Ok(());
    }

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);

    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    for name in student_names {
        let label = name.to_string();
        let key = CallbackKey::pack(
            Cmd::impersonate(ImpersonateParams::Student(Some(name))),
            &user_proxy,
        )
        .await;
        let button = InlineKeyboardButton::callback_key(label, &key);
        buttons.push(vec![button]);
    }

    let keyboard = InlineKeyboardMarkup::new(buttons);
    ctx.bot
        .send_markdown_message(
            ctx.chat_id,
            markdown_string!("Select the student you want to impersonate:"),
        )
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
        let text = markdown_format!(
            "Student {} was not found in the spreadsheet\\. Impersonation mode has been deactivated\\.",
            student_name.to_string()
        );
        ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
        return Ok(());
    };
    api::student::schedule(ctx, &student).await
}

pub async fn quit(ctx: &BotCtx<impl Send + Sync + Clone>, teacher: &Teacher) -> Result<()> {
    ctx.state.clear_impersonation(ctx.chat_id).await;
    let text = markdown_format!(
        "Exited impersonation mode\\. You are back as {} \\(teacher\\)\\.",
        teacher.telegram_name.to_string()
    );
    ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
    Ok(())
}
