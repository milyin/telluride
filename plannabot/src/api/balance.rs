#![allow(non_snake_case)]
use anyhow::Result;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use chrono_tz::Tz;
use telluride::command::{CallbackBitcode, CallbackKey, InlineKeyboardButtonPackedExt};
use telluride::data_store::UserProxy;
use telluride::{markdown_format, markdown_string};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::api::common::{PageInfo, fmt_money, fmt_money_plain};
use crate::api::context::BotCtx;
use crate::api::traits::{BalanceActor, BalanceCommand, BalanceParams};
use crate::models::TelegramName;

pub async fn balance<Cmd: BalanceCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    params: &str,
    actor: &BalanceActor,
) -> Result<()> {
    let params: BalanceParams = params.parse()?;
    match params {
        BalanceParams::L0(page) => match actor {
            BalanceActor::Student(name) => balance_L1(ctx, actor, name.clone()).await,
            _ => balance_L0(ctx, actor, page).await,
        },
        BalanceParams::L1(student) => balance_L1(ctx, actor, student).await,
        BalanceParams::L2(student, year, month) => {
            balance_L2(ctx, actor, student, year, month).await
        }
    }
}

// ---------------------------------------------------------------------------
// L0 — paginated student list (teacher / admin only)
// ---------------------------------------------------------------------------

async fn balance_L0<Cmd: BalanceCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    actor: &BalanceActor,
    page: i32,
) -> Result<()> {
    let mut students: Vec<TelegramName> = match actor {
        BalanceActor::Teacher(t) => ctx
            .state
            .get_pairings_for_teacher(t)
            .await
            .into_iter()
            .map(|p| p.student_telegram)
            .collect(),
        BalanceActor::Admin => ctx.state.get_student_names().await,
        BalanceActor::Student(_) => unreachable!(),
    };
    students.sort();
    students.dedup();

    let total = students.len();
    let info = PageInfo::new(page, total);

    if total == 0 {
        return ctx
            .update_markdown_message(
                markdown_string!("💰 *Balance*\n\nNo students found\\."),
                None,
            )
            .await;
    }

    let (schedule, _) = ctx.state.sheets.get_schedule().await?;
    let (all_payments, _) = ctx.state.sheets.get_payments().await?;

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    for student in &students[info.start..info.end] {
        let allocated: i64 = schedule
            .iter()
            .filter(|e| &e.student_telegram == student && e.status.is_some())
            .map(|e| e.cost)
            .sum();
        let paid: i64 = all_payments
            .iter()
            .filter(|p| &p.student_telegram == student)
            .map(|p| p.sum)
            .sum();
        let remains = paid - allocated;

        let currency = ctx.state.get_student(student).await.and_then(|s| s.currency);
        let balance_str = fmt_money_plain(remains, currency);
        let label = format!("{}: {}", student, balance_str);

        let key = CallbackKey::pack(
            Cmd::balance(BalanceParams::L1(student.clone())),
            &user_proxy,
        )
        .await;
        buttons.push(vec![InlineKeyboardButton::callback_key(label, &key)]);
    }

    let mut nav_row: Vec<InlineKeyboardButton> = Vec::new();
    if info.has_prev() {
        let k = CallbackKey::pack(Cmd::balance(BalanceParams::L0(info.page - 1)), &user_proxy).await;
        nav_row.push(InlineKeyboardButton::callback_key("<", &k));
    }
    if info.has_next() {
        let k = CallbackKey::pack(Cmd::balance(BalanceParams::L0(info.page + 1)), &user_proxy).await;
        nav_row.push(InlineKeyboardButton::callback_key(">", &k));
    }
    if !nav_row.is_empty() {
        buttons.push(nav_row);
    }

    let keyboard = InlineKeyboardMarkup::new(buttons);
    ctx.update_markdown_message(
        markdown_string!("💰 *Balance*\n\nSelect a student:"),
        Some(keyboard),
    )
    .await
}

// ---------------------------------------------------------------------------
// L1 — redirect to current month's view
// ---------------------------------------------------------------------------

async fn balance_L1<Cmd: BalanceCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    actor: &BalanceActor,
    student: TelegramName,
) -> Result<()> {
    let now = chrono::Utc::now();
    balance_L2(ctx, actor, student, now.year(), now.month()).await
}

// ---------------------------------------------------------------------------
// L2 — monthly balance view
// ---------------------------------------------------------------------------

async fn balance_L2<Cmd: BalanceCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    actor: &BalanceActor,
    student: TelegramName,
    year: i32,
    month: u32,
) -> Result<()> {
    let student_data = ctx.state.get_student(&student).await;
    let currency = student_data.as_ref().and_then(|s| s.currency);
    let tz: Tz = student_data
        .as_ref()
        .map(|s| s.timezone)
        .unwrap_or(chrono_tz::UTC);

    let (schedule, _) = ctx.state.sheets.get_schedule().await?;
    let (all_payments, _) = ctx.state.sheets.get_payments().await?;

    // All-time totals
    let allocated: i64 = schedule
        .iter()
        .filter(|e| &e.student_telegram == &student && e.status.is_some())
        .map(|e| e.cost)
        .sum();
    let paid: i64 = all_payments
        .iter()
        .filter(|p| &p.student_telegram == &student)
        .map(|p| p.sum)
        .sum();
    let remains = paid - allocated;

    // Build header
    let mut text = markdown_format!("💰 *Balance: {}*\n\n", student.to_string());
    text.push(&markdown_format!("Balance: {}\n", fmt_money(paid, currency)));
    text.push(&markdown_format!("Allocated: {}\n", fmt_money(allocated, currency)));
    text.push(&markdown_format!("Remains: {}\n", fmt_money(remains, currency)));

    // Month name
    let month_label = NaiveDate::from_ymd_opt(year, month, 1)
        .map(|d| d.format("%B %Y").to_string())
        .unwrap_or_else(|| format!("{}/{}", month, year));
    text.push(&markdown_format!("\n*Transactions on {}*\n", month_label));

    // Collect this month's transactions sorted by datetime
    let mut txns: Vec<(DateTime<Utc>, String)> = Vec::new();

    for p in all_payments
        .iter()
        .filter(|p| &p.student_telegram == &student)
        .filter(|p| {
            let local = p.date.with_timezone(&tz);
            local.year() == year && local.month() == month
        })
    {
        let date_str = p.date.with_timezone(&tz).format("%d %b").to_string();
        txns.push((
            p.date,
            format!("{} +{} paid", date_str, fmt_money_plain(p.sum, currency)),
        ));
    }

    for e in schedule
        .iter()
        .filter(|e| &e.student_telegram == &student && e.status.is_some())
        .filter(|e| {
            let local = e.datetime.with_timezone(&tz);
            local.year() == year && local.month() == month
        })
    {
        let date_str = e.datetime.with_timezone(&tz).format("%d %b").to_string();
        txns.push((
            e.datetime,
            format!(
                "{} -{} lesson with {}",
                date_str,
                fmt_money_plain(e.cost, currency),
                e.teacher_telegram
            ),
        ));
    }

    txns.sort_by_key(|(dt, _)| *dt);

    if txns.is_empty() {
        text.push(&markdown_string!("_no transactions_\n"));
    } else {
        for (_, line) in &txns {
            text.push(&markdown_format!("{}\n", line));
        }
    }

    // Navigation buttons
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let (py, pm) = prev_month(year, month);
    let (ny, nm) = next_month(year, month);
    let prev_label = format!(
        "< {}",
        NaiveDate::from_ymd_opt(py, pm, 1)
            .map(|d| d.format("%B %Y").to_string())
            .unwrap_or_default()
    );
    let next_label = format!(
        "{} >",
        NaiveDate::from_ymd_opt(ny, nm, 1)
            .map(|d| d.format("%B %Y").to_string())
            .unwrap_or_default()
    );
    let prev_key = CallbackKey::pack(
        Cmd::balance(BalanceParams::L2(student.clone(), py, pm)),
        &user_proxy,
    )
    .await;
    let next_key = CallbackKey::pack(
        Cmd::balance(BalanceParams::L2(student.clone(), ny, nm)),
        &user_proxy,
    )
    .await;

    let mut rows: Vec<Vec<InlineKeyboardButton>> = vec![vec![
        InlineKeyboardButton::callback_key(prev_label, &prev_key),
        InlineKeyboardButton::callback_key(next_label, &next_key),
    ]];

    match actor {
        BalanceActor::Teacher(_) | BalanceActor::Admin => {
            let back_key =
                CallbackKey::pack(Cmd::balance(BalanceParams::L0(0)), &user_proxy).await;
            rows.push(vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)]);
        }
        BalanceActor::Student(_) => {}
    }

    ctx.update_markdown_message(text, Some(InlineKeyboardMarkup::new(rows)))
        .await
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn prev_month(year: i32, month: u32) -> (i32, u32) {
    if month == 1 {
        (year - 1, 12)
    } else {
        (year, month - 1)
    }
}

fn next_month(year: i32, month: u32) -> (i32, u32) {
    if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    }
}
