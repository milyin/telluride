#![allow(non_snake_case)]
use anyhow::Result;
use chrono::Utc;
use telluride::command::{CallbackBitcode, CallbackKey, InlineKeyboardButtonPackedExt};
use telluride::data_store::UserProxy;
use telluride::markdown::MarkdownStringMessage;
use telluride::{markdown_format, markdown_string};
use teloxide::payloads::{EditMessageTextSetters, SendMessageSetters};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::api::common::format_duration;
use crate::api::context::BotCtx;
use crate::api::traits::payment::{format_amount, PaymentParams};
use crate::api::traits::PaymentCommand;
use crate::models::{LessonStatus, Teacher, TelegramName};

pub async fn payment<Cmd: PaymentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    params: &str,
    teacher: &Teacher,
) -> Result<()> {
    let params: PaymentParams = params.parse()?;
    match params {
        PaymentParams::P0 => payment_P0(ctx, teacher).await,
        PaymentParams::P1(student) => payment_P1(ctx, teacher, student).await,
        PaymentParams::P2(student, amt) => payment_P2(ctx, teacher, student, amt).await,
        PaymentParams::PF(student, amt) => payment_PF(ctx, teacher, student, amt).await,
    }
}

// ---------------------------------------------------------------------------
// P0 — list all students with their balances
// ---------------------------------------------------------------------------

async fn payment_P0<Cmd: PaymentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &Teacher,
) -> Result<()> {
    let pairings = ctx
        .state
        .get_pairings_for_teacher(&teacher.telegram_name)
        .await;

    if pairings.is_empty() {
        ctx.bot
            .send_markdown_message(
                ctx.chat_id,
                markdown_string!("💰 *Payments*\n\nNo students assigned to you\\."),
            )
            .await?;
        return Ok(());
    }

    let (schedule, _) = ctx.state.sheets.get_schedule().await?;
    let (all_payments, _) = ctx.state.sheets.get_payments().await?;

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    for pairing in &pairings {
        let student = &pairing.student_telegram;
        let earned: i64 = schedule
            .iter()
            .filter(|e| {
                &e.teacher_telegram == &teacher.telegram_name
                    && &e.student_telegram == student
                    && e.status == Some(LessonStatus::Passed)
            })
            .map(|e| e.cost)
            .sum();
        let paid: i64 = all_payments
            .iter()
            .filter(|p| &p.student_telegram == student)
            .map(|p| p.sum)
            .sum();
        let balance = earned - paid;

        let label = if balance <= 0 {
            format!("{} ✓ paid", student)
        } else {
            let currency = ctx
                .state
                .get_student(student)
                .await
                .map(|s| s.currency)
                .unwrap_or_default();
            if currency.is_empty() {
                format!("{}: {} owed", student, format_amount(balance))
            } else {
                format!("{}: {} {} owed", student, format_amount(balance), currency)
            }
        };

        let cmd = Cmd::payment(PaymentParams::P1(student.clone()));
        let key = CallbackKey::pack(cmd, &user_proxy).await;
        buttons.push(vec![InlineKeyboardButton::callback_key(label, &key)]);
    }

    let keyboard = InlineKeyboardMarkup::new(buttons);
    let text = markdown_string!("💰 *Payments*\n\nSelect a student to view their balance:");

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
// P1 — balance breakdown for one student
// ---------------------------------------------------------------------------

async fn payment_P1<Cmd: PaymentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &Teacher,
    student: TelegramName,
) -> Result<()> {
    let student_data = ctx.state.get_student(&student).await;
    let currency = student_data
        .as_ref()
        .map(|s| s.currency.as_str())
        .unwrap_or("")
        .to_string();
    let tz = student_data
        .as_ref()
        .map(|s| s.timezone)
        .unwrap_or(chrono_tz::UTC);

    let (schedule, _) = ctx.state.sheets.get_schedule().await?;
    let (all_payments, _) = ctx.state.sheets.get_payments().await?;

    let passed_lessons: Vec<_> = schedule
        .iter()
        .filter(|e| {
            &e.teacher_telegram == &teacher.telegram_name
                && &e.student_telegram == &student
                && e.status == Some(LessonStatus::Passed)
        })
        .collect();

    let student_payments: Vec<_> = all_payments
        .iter()
        .filter(|p| &p.student_telegram == &student)
        .collect();

    let earned: i64 = passed_lessons.iter().map(|e| e.cost).sum();
    let paid: i64 = student_payments.iter().map(|p| p.sum).sum();
    let balance = earned - paid;

    let fmt_cost = |cents: i64| -> String {
        if currency.is_empty() {
            format_amount(cents)
        } else {
            format!("{} {}", format_amount(cents), currency)
        }
    };

    let mut text = markdown_format!("💰 *Balance for {}*\n\n", student.to_string());

    if passed_lessons.is_empty() {
        text.push(&markdown_string!("_No completed lessons yet_\n"));
    } else {
        text.push(&markdown_string!("*Completed lessons:*\n"));
        let mut sorted = passed_lessons.clone();
        sorted.sort_by_key(|e| e.datetime);
        for entry in &sorted {
            let local = entry.datetime.with_timezone(&tz);
            let date_str = local.format("%Y-%m-%d").to_string();
            let dur_str = format_duration(entry.duration_minutes as i64);
            let line = markdown_format!(
                "  {} \\| {} \\| {}\n",
                date_str,
                dur_str,
                fmt_cost(entry.cost)
            );
            text.push(&line);
        }
    }

    text.push(&markdown_format!(
        "\n*Total earned:* {}\n",
        fmt_cost(earned)
    ));

    if student_payments.is_empty() {
        text.push(&markdown_string!("\n_No payments recorded yet_\n"));
    } else {
        text.push(&markdown_string!("\n*Payments received:*\n"));
        let mut sorted_payments = student_payments.clone();
        sorted_payments.sort_by_key(|p| p.date);
        for p in &sorted_payments {
            let local = p.date.with_timezone(&tz);
            let date_str = local.format("%Y-%m-%d").to_string();
            let line = markdown_format!("  {} \\| {}\n", date_str, fmt_cost(p.sum));
            text.push(&line);
        }
    }

    text.push(&markdown_format!("\n*Total paid:* {}\n", fmt_cost(paid)));

    if balance > 0 {
        text.push(&markdown_format!(
            "\n*Balance owed:* {}\n",
            fmt_cost(balance)
        ));
    } else {
        text.push(&markdown_string!("\n*Balance:* ✓ fully paid\n"));
    }

    text.push(&markdown_format!(
        "\nTo record a payment: `/payment {} 100`",
        student.to_string()
    ));

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(Cmd::payment(PaymentParams::P0), &user_proxy).await;
    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback_key("↩ Back", &back_key),
    ]]);

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
// P2 — confirm recording a payment
// ---------------------------------------------------------------------------

async fn payment_P2<Cmd: PaymentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &Teacher,
    student: TelegramName,
    amount_cents: i64,
) -> Result<()> {
    let student_data = ctx.state.get_student(&student).await;
    let currency = student_data
        .as_ref()
        .map(|s| s.currency.as_str())
        .unwrap_or("")
        .to_string();

    let amount_str = if currency.is_empty() {
        format_amount(amount_cents)
    } else {
        format!("{} {}", format_amount(amount_cents), currency)
    };

    let text = markdown_format!(
        "💰 *Record Payment*\n\nRecord payment of *{}* from {}?",
        amount_str,
        student.to_string()
    );

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let confirm_key = CallbackKey::pack(
        Cmd::payment(PaymentParams::PF(student.clone(), amount_cents)),
        &user_proxy,
    )
    .await;
    let cancel_key = CallbackKey::pack(
        Cmd::payment(PaymentParams::P1(student.clone())),
        &user_proxy,
    )
    .await;
    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback_key("✓ Confirm", &confirm_key),
        InlineKeyboardButton::callback_key("✗ Cancel", &cancel_key),
    ]]);

    // Suppress unused variable warning; teacher is available for future use (e.g. audit).
    let _ = teacher;

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
// PF — execute: write payment to Payments sheet
// ---------------------------------------------------------------------------

async fn payment_PF<Cmd: PaymentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &Teacher,
    student: TelegramName,
    amount_cents: i64,
) -> Result<()> {
    let student_data = ctx.state.get_student(&student).await;
    let currency = student_data
        .as_ref()
        .map(|s| s.currency.as_str())
        .unwrap_or("")
        .to_string();

    ctx.state
        .sheets
        .add_payment(&student, Utc::now(), amount_cents)
        .await?;

    let amount_str = if currency.is_empty() {
        format_amount(amount_cents)
    } else {
        format!("{} {}", format_amount(amount_cents), currency)
    };

    let text = markdown_format!(
        "✅ Payment of *{}* from {} recorded\\.",
        amount_str,
        student.to_string()
    );

    // Suppress unused variable warning; teacher is available for future use (e.g. audit).
    let _ = teacher;

    ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
    Ok(())
}
