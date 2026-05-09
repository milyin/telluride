#![allow(non_snake_case)]
use anyhow::Result;
use chrono::{NaiveDate, NaiveTime, Utc};
use telluride::command::{CallbackBitcode, CallbackKey, InlineKeyboardButtonPackedExt};
use telluride::data_store::UserProxy;
use telluride::{markdown_format, markdown_string};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::api::common::{PageInfo, fmt_money, fmt_money_plain};
use crate::api::context::BotCtx;
use crate::api::traits::payment::PaymentParams;
use crate::api::traits::{PaymentActor, PaymentCommand};
use crate::models::{LessonStatus, TelegramName};

pub async fn payment<Cmd: PaymentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    params: &str,
    actor: &PaymentActor,
) -> Result<()> {
    let params: PaymentParams = params.parse()?;
    match params {
        PaymentParams::P0(page) => payment_P0(ctx, actor, page).await,
        PaymentParams::P1(student) => payment_P1(ctx, student).await,
        PaymentParams::PL(student, page) => payment_PL(ctx, student, page).await,
        PaymentParams::PS(student, date, time) => payment_PS(ctx, student, date, time).await,
        PaymentParams::PD(student, date, time) => payment_PD(ctx, student, date, time).await,
        PaymentParams::PDF(student, date, time) => payment_PDF(ctx, student, date, time).await,
        PaymentParams::PN(student) => payment_PN(ctx, student).await,
        PaymentParams::PNF(student, amount) => payment_PNF(ctx, student, amount).await,
    }
}

// ---------------------------------------------------------------------------
// P0 — paginated student list
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
        return ctx
            .update_markdown_message(
                markdown_string!("💰 *Payments*\n\nNo students assigned to you\\."),
                None,
            )
            .await;
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
        let k =
            CallbackKey::pack(Cmd::payment(PaymentParams::P0(info.page - 1)), &user_proxy).await;
        nav_row.push(InlineKeyboardButton::callback_key("<", &k));
    }
    if info.has_next() {
        let k =
            CallbackKey::pack(Cmd::payment(PaymentParams::P0(info.page + 1)), &user_proxy).await;
        nav_row.push(InlineKeyboardButton::callback_key(">", &k));
    }
    if !nav_row.is_empty() {
        buttons.push(nav_row);
    }

    let keyboard = InlineKeyboardMarkup::new(buttons);
    ctx.update_markdown_message(
        markdown_string!("💰 *Payments*\n\nSelect a student:"),
        Some(keyboard),
    )
    .await
}

// ---------------------------------------------------------------------------
// P1 — student menu
// ---------------------------------------------------------------------------

async fn payment_P1<Cmd: PaymentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    student: TelegramName,
) -> Result<()> {
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);

    let list_key = CallbackKey::pack(
        Cmd::payment(PaymentParams::PL(student.clone(), 0)),
        &user_proxy,
    )
    .await;
    let new_key = CallbackKey::pack(
        Cmd::payment(PaymentParams::PN(student.clone())),
        &user_proxy,
    )
    .await;
    let back_key = CallbackKey::pack(Cmd::payment(PaymentParams::P0(0)), &user_proxy).await;

    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback_key(
            "📋 List Payments",
            &list_key,
        )],
        vec![InlineKeyboardButton::callback_key(
            "➕ New Payment",
            &new_key,
        )],
        vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)],
    ]);

    let text = markdown_format!("💰 *Payments: {}*", student.to_string());
    ctx.update_markdown_message(text, Some(keyboard)).await
}

// ---------------------------------------------------------------------------
// PL — paginated payment list for student
// ---------------------------------------------------------------------------

async fn payment_PL<Cmd: PaymentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    student: TelegramName,
    page: i32,
) -> Result<()> {
    let mut payments = ctx.state.sheets.get_payments_for_student(&student).await?;
    payments.sort_by(|a, b| b.date.cmp(&a.date));

    let currency = ctx.state.get_student(&student).await.and_then(|s| s.currency);

    let total = payments.len();
    let info = PageInfo::new(page, total);

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    for payment in &payments[info.start..info.end] {
        let date = payment.date.date_naive();
        let time = payment.date.time();
        let label = format!(
            "{} {}: {}",
            date,
            time.format("%H:%M"),
            fmt_money_plain(payment.sum, currency)
        );
        let key = CallbackKey::pack(
            Cmd::payment(PaymentParams::PS(student.clone(), date, time)),
            &user_proxy,
        )
        .await;
        buttons.push(vec![InlineKeyboardButton::callback_key(label, &key)]);
    }

    let mut nav_row: Vec<InlineKeyboardButton> = Vec::new();
    if info.has_prev() {
        let k = CallbackKey::pack(
            Cmd::payment(PaymentParams::PL(student.clone(), info.page - 1)),
            &user_proxy,
        )
        .await;
        nav_row.push(InlineKeyboardButton::callback_key("<", &k));
    }
    if info.has_next() {
        let k = CallbackKey::pack(
            Cmd::payment(PaymentParams::PL(student.clone(), info.page + 1)),
            &user_proxy,
        )
        .await;
        nav_row.push(InlineKeyboardButton::callback_key(">", &k));
    }
    if !nav_row.is_empty() {
        buttons.push(nav_row);
    }

    let back_key = CallbackKey::pack(
        Cmd::payment(PaymentParams::P1(student.clone())),
        &user_proxy,
    )
    .await;
    buttons.push(vec![InlineKeyboardButton::callback_key(
        "↩ Back", &back_key,
    )]);

    let header = if total == 0 {
        markdown_format!(
            "💰 *Payments: {}*\n\nNo payments found\\.",
            student.to_string()
        )
    } else {
        markdown_format!(
            "💰 *Payments: {}*\n\nSelect a payment:",
            student.to_string()
        )
    };

    ctx.update_markdown_message(header, Some(InlineKeyboardMarkup::new(buttons)))
        .await
}

// ---------------------------------------------------------------------------
// PS — show single payment
// ---------------------------------------------------------------------------

async fn payment_PS<Cmd: PaymentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    student: TelegramName,
    date: NaiveDate,
    time: NaiveTime,
) -> Result<()> {
    let currency = ctx.state.get_student(&student).await.and_then(|s| s.currency);

    let dt = date.and_time(time).and_utc();
    let payments = ctx.state.sheets.get_payments_for_student(&student).await?;
    let payment = payments.iter().find(|p| p.date == dt);

    let sum = payment.map(|p| p.sum).unwrap_or(0);

    let mut text = markdown_format!("💰 *Payment: {}*\n\n", student.to_string());
    text.push(&markdown_format!("📆 Date: {}\n", date.to_string()));
    text.push(&markdown_format!(
        "⏰ Time: {}\n",
        time.format("%H:%M").to_string()
    ));
    text.push(&markdown_format!("💵 Amount: {}\n", fmt_money(sum, currency)));

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(
        Cmd::payment(PaymentParams::PL(student.clone(), 0)),
        &user_proxy,
    )
    .await;

    let delete_key = CallbackKey::pack(
        Cmd::payment(PaymentParams::PD(student.clone(), date, time)),
        &user_proxy,
    )
    .await;
    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback_key("🗑️ Delete", &delete_key)],
        vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)],
    ]);

    ctx.update_markdown_message(text, Some(keyboard)).await
}

// ---------------------------------------------------------------------------
// PD — confirm delete
// ---------------------------------------------------------------------------

async fn payment_PD<Cmd: PaymentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    student: TelegramName,
    date: NaiveDate,
    time: NaiveTime,
) -> Result<()> {
    let currency = ctx.state.get_student(&student).await.and_then(|s| s.currency);

    let dt = date.and_time(time).and_utc();
    let payments = ctx.state.sheets.get_payments_for_student(&student).await?;
    let payment = payments.iter().find(|p| p.date == dt);
    let sum = payment.map(|p| p.sum).unwrap_or(0);

    let mut text = markdown_string!("🗑️ *Delete Payment*\n\nDelete this payment?\n\n");
    text.push(&markdown_format!("🎓 Student: {}\n", student.to_string()));
    text.push(&markdown_format!("📆 Date: {}\n", date.to_string()));
    text.push(&markdown_format!(
        "⏰ Time: {}\n",
        time.format("%H:%M").to_string()
    ));
    text.push(&markdown_format!("💵 Amount: {}\n", fmt_money(sum, currency)));

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(
        Cmd::payment(PaymentParams::PS(student.clone(), date, time)),
        &user_proxy,
    )
    .await;
    let confirm_params = format!(
        "/payment {}",
        PaymentParams::PDF(student.clone(), date, time)
    );

    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback_key("↩ Back", &back_key),
        InlineKeyboardButton::switch_inline_query_current_chat("🗑️ Yes, delete it", confirm_params),
    ]]);

    ctx.update_markdown_message(text, Some(keyboard)).await
}

// ---------------------------------------------------------------------------
// PDF — execute delete
// ---------------------------------------------------------------------------

async fn payment_PDF<Cmd: Send + Sync + Clone>(
    ctx: &BotCtx<Cmd>,
    student: TelegramName,
    date: NaiveDate,
    time: NaiveTime,
) -> Result<()> {
    let dt = date.and_time(time).and_utc();
    ctx.state.sheets.delete_payment(&student, dt).await?;

    let mut text = markdown_string!("✅ *Payment Deleted*\n\n");
    text.push(&markdown_format!("🎓 Student: {}\n", student.to_string()));
    text.push(&markdown_format!("📆 Date: {}\n", date.to_string()));
    text.push(&markdown_format!(
        "⏰ Time: {}\n",
        time.format("%H:%M").to_string()
    ));

    ctx.update_markdown_message(text, None).await
}

// ---------------------------------------------------------------------------
// PN — new payment prompt
// ---------------------------------------------------------------------------

async fn payment_PN<Cmd: PaymentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    student: TelegramName,
) -> Result<()> {
    let (all_payments, _) = ctx.state.sheets.get_payments().await?;
    let (schedule, _) = ctx.state.sheets.get_schedule().await?;

    let currency = ctx.state.get_student(&student).await.and_then(|s| s.currency);

    let now = Utc::now();
    let spent: i64 = schedule
        .iter()
        .filter(|e| e.student_telegram == student && e.status != Some(LessonStatus::Cancelled))
        .filter(|e| e.status.is_some() || e.datetime <= now)
        .map(|e| e.cost)
        .sum();
    let paid: i64 = all_payments
        .iter()
        .filter(|p| p.student_telegram == student)
        .map(|p| p.sum)
        .sum();
    let balance = paid - spent;
    let mut text = markdown_format!("💰 *New Payment: {}*\n\n", student.to_string());
    text.push(&markdown_format!("Balance: {}\n\n", fmt_money(balance, currency)));
    text.push(&markdown_string!(
        "Type the amount after the command below and send:"
    ));

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(
        Cmd::payment(PaymentParams::P1(student.clone())),
        &user_proxy,
    )
    .await;

    let new_payment_cmd = format!("/payment new! {} ", student);
    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::switch_inline_query_current_chat(
            "➕ Enter amount",
            new_payment_cmd,
        )],
        vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)],
    ]);

    ctx.update_markdown_message(text, Some(keyboard)).await
}

// ---------------------------------------------------------------------------
// PNF — create payment
// ---------------------------------------------------------------------------

async fn payment_PNF<Cmd: Send + Sync + Clone>(
    ctx: &BotCtx<Cmd>,
    student: TelegramName,
    amount: i64,
) -> Result<()> {
    let currency = ctx.state.get_student(&student).await.and_then(|s| s.currency);

    let now = Utc::now();
    ctx.state.sheets.add_payment(&student, now, amount).await?;

    let mut text = markdown_string!("✅ *Payment Added*\n\n");
    text.push(&markdown_format!("🎓 Student: {}\n", student.to_string()));
    text.push(&markdown_format!("💵 Amount: {}\n", fmt_money(amount, currency)));

    ctx.update_markdown_message(text, None).await
}
