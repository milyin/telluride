use crate::api::context::BotCtx;
use crate::api::traits::{ImpersonateCommand, ImpersonateParams};
use anyhow::Result;
use telluride::command::{CallbackBitcode, CallbackKey, InlineKeyboardButtonPackedExt};
use telluride::data_store::UserProxy;
use telluride::{markdown_format, markdown_string};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub async fn impersonate<Cmd: ImpersonateCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    params: &str,
) -> Result<()> {
    if ctx.state.get_student_impersonation(ctx.chat_id).await.is_some()
        || ctx.state.get_teacher_impersonation(ctx.chat_id).await.is_some()
    {
        ctx.update_markdown_message(
            markdown_string!("You are already in impersonation mode\\. Use /quit first\\."),
            None,
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
                    ctx.state.impersonate_student(ctx.chat_id, name.clone()).await;
                    let text = markdown_format!(
                        "Now impersonating {}\\. All commands will behave as if you were that student\\. \
                         Use /help to see available commands, use /quit to exit",
                        name.to_string()
                    );
                    ctx.update_markdown_message(text, None).await?;
                }
                None => {
                    let text = markdown_format!(
                        "Student {} was not found in the spreadsheet\\.",
                        name.to_string()
                    );
                    ctx.update_markdown_message(text, None).await?;
                }
            }
            Ok(())
        }
        ImpersonateParams::Teacher(None) => show_teacher_selection(ctx).await,
        ImpersonateParams::Teacher(Some(name)) => {
            match ctx.state.get_teacher(&name).await {
                Some(_) => {
                    ctx.state.exit_admin_mode(ctx.chat_id).await;
                    ctx.state.impersonate_teacher(ctx.chat_id, name.clone()).await;
                    let text = markdown_format!(
                        "Now impersonating teacher {}\\. All commands will behave as if you were that teacher\\. \
                         Use /help to see available commands, use /quit to exit",
                        name.to_string()
                    );
                    ctx.update_markdown_message(text, None).await?;
                }
                None => {
                    let text = markdown_format!(
                        "Teacher {} was not found in the spreadsheet\\.",
                        name.to_string()
                    );
                    ctx.update_markdown_message(text, None).await?;
                }
            }
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

    ctx.update_markdown_message(
        markdown_string!("Select role to impersonate:"),
        Some(InlineKeyboardMarkup::new(buttons)),
    )
    .await
}

async fn show_student_selection<Cmd: ImpersonateCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
) -> Result<()> {
    let student_names = ctx.state.get_student_names().await;

    if student_names.is_empty() {
        ctx.update_markdown_message(
            markdown_string!("No students are registered in the spreadsheet yet\\."),
            None,
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
        buttons.push(vec![InlineKeyboardButton::callback_key(label, &key)]);
    }

    let back_key = CallbackKey::pack(Cmd::impersonate(ImpersonateParams::I0), &user_proxy).await;
    buttons.push(vec![InlineKeyboardButton::callback_key("↩ Back".to_string(), &back_key)]);

    ctx.update_markdown_message(
        markdown_string!("Select the student you want to impersonate:"),
        Some(InlineKeyboardMarkup::new(buttons)),
    )
    .await
}

async fn show_teacher_selection<Cmd: ImpersonateCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
) -> Result<()> {
    let teacher_names = ctx.state.get_teacher_names().await;

    if teacher_names.is_empty() {
        ctx.update_markdown_message(
            markdown_string!("No teachers are registered in the spreadsheet yet\\."),
            None,
        )
        .await?;
        return Ok(());
    }

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);

    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    for name in teacher_names {
        let label = name.to_string();
        let key = CallbackKey::pack(
            Cmd::impersonate(ImpersonateParams::Teacher(Some(name))),
            &user_proxy,
        )
        .await;
        buttons.push(vec![InlineKeyboardButton::callback_key(label, &key)]);
    }

    let back_key = CallbackKey::pack(Cmd::impersonate(ImpersonateParams::I0), &user_proxy).await;
    buttons.push(vec![InlineKeyboardButton::callback_key("↩ Back".to_string(), &back_key)]);

    ctx.update_markdown_message(
        markdown_string!("Select the teacher you want to impersonate:"),
        Some(InlineKeyboardMarkup::new(buttons)),
    )
    .await
}
