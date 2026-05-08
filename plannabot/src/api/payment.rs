#![allow(non_snake_case)]
use anyhow::Result;
use telluride::command::{CallbackBitcode, CallbackKey, InlineKeyboardButtonPackedExt};
use telluride::data_store::UserProxy;
use telluride::markdown::MarkdownStringMessage;
use telluride::markdown_string;
use teloxide::payloads::{EditMessageTextSetters, SendMessageSetters};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::api::common::PageInfo;
use crate::api::context::BotCtx;
use crate::api::traits::payment::PaymentParams;
use crate::api::traits::{PaymentActor, PaymentCommand};
use crate::models::TelegramName;

pub async fn payment<Cmd: PaymentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    params: &str,
    actor: &PaymentActor,
) -> Result<()> {
    let params: PaymentParams = params.parse()?;
    match params {
        PaymentParams::P0(page) => payment_P0(ctx, actor, page).await,
        PaymentParams::P1(student) => payment_P1(ctx, actor, student).await,
    }
}

// ---------------------------------------------------------------------------
// P0 — paginated student list (names only)
// ---------------------------------------------------------------------------

async fn payment_P0<Cmd: PaymentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    actor: &PaymentActor,
    page: i32,
) -> Result<()> {
    let mut students: Vec<TelegramName> = match actor {
        PaymentActor::Teacher(t) => ctx
            .state
            .get_pairings_for_teacher(t)
            .await
            .into_iter()
            .map(|p| p.student_telegram)
            .collect(),
        PaymentActor::Admin => ctx.state.get_student_names().await,
    };
    students.sort();
    students.dedup();

    let total = students.len();
    let info = PageInfo::new(page, total);

    if total == 0 {
        let text = markdown_string!("💰 *Payments*\n\nNo students assigned to you\\.");
        match ctx.message_id {
            Some(id) => {
                ctx.bot
                    .edit_markdown_message_text(ctx.chat_id, id, text)
                    .await?;
            }
            None => {
                ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
            }
        }
        return Ok(());
    }

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    for student in &students[info.start..info.end] {
        let cmd = Cmd::payment(PaymentParams::P1(student.clone()));
        let key = CallbackKey::pack(cmd, &user_proxy).await;
        buttons.push(vec![InlineKeyboardButton::callback_key(
            student.to_string(),
            &key,
        )]);
    }

    let mut nav_row: Vec<InlineKeyboardButton> = Vec::new();
    if info.has_prev() {
        let k = CallbackKey::pack(
            Cmd::payment(PaymentParams::P0(info.page - 1)),
            &user_proxy,
        )
        .await;
        nav_row.push(InlineKeyboardButton::callback_key("<", &k));
    }
    if info.has_next() {
        let k = CallbackKey::pack(
            Cmd::payment(PaymentParams::P0(info.page + 1)),
            &user_proxy,
        )
        .await;
        nav_row.push(InlineKeyboardButton::callback_key(">", &k));
    }
    if !nav_row.is_empty() {
        buttons.push(nav_row);
    }

    let keyboard = InlineKeyboardMarkup::new(buttons);
    let text = markdown_string!("💰 *Payments*\n\nSelect a student:");

    match ctx.message_id {
        Some(id) => {
            ctx.bot
                .edit_markdown_message_text(ctx.chat_id, id, text)
                .reply_markup(keyboard)
                .await?;
        }
        None => {
            ctx.bot
                .send_markdown_message(ctx.chat_id, text)
                .reply_markup(keyboard)
                .await?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// P1 — student detail (not yet implemented)
// ---------------------------------------------------------------------------

async fn payment_P1<Cmd: PaymentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    _actor: &PaymentActor,
    _student: TelegramName,
) -> Result<()> {
    let text = markdown_string!("💰 *Payments*\n\n_Not implemented yet\\._");

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(Cmd::payment(PaymentParams::P0(0)), &user_proxy).await;
    let keyboard = InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback_key(
        "↩ Back",
        &back_key,
    )]]);

    match ctx.message_id {
        Some(id) => {
            ctx.bot
                .edit_markdown_message_text(ctx.chat_id, id, text)
                .reply_markup(keyboard)
                .await?;
        }
        None => {
            ctx.bot
                .send_markdown_message(ctx.chat_id, text)
                .reply_markup(keyboard)
                .await?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// P2 and PF are reserved for future payment recording implementation.
// ---------------------------------------------------------------------------

// async fn payment_P2<Cmd: PaymentCommand + CallbackBitcode + 'static>(
//     ctx: &BotCtx<Cmd>,
//     actor: &PaymentActor,
//     student: TelegramName,
//     amount_cents: i64,
// ) -> Result<()> {
//     todo!()
// }

// async fn payment_PF<Cmd: PaymentCommand + CallbackBitcode + 'static>(
//     ctx: &BotCtx<Cmd>,
//     actor: &PaymentActor,
//     student: TelegramName,
//     amount_cents: i64,
// ) -> Result<()> {
//     todo!()
// }
