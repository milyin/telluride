#![allow(non_snake_case)]
use anyhow::Result;
use chrono::{Datelike, NaiveDate, NaiveTime, Timelike, Weekday};
use std::collections::HashMap;
use telluride::command::{CallbackBitcode, CallbackKey, InlineKeyboardButtonPackedExt};
use telluride::data_store::UserProxy;
use telluride::{markdown_format, markdown_string};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::api::context::BotCtx;
use crate::api::menus::show_date_selection;
use crate::api::traits::WorktimeCommand;
use crate::api::traits::worktime::WorktimeParams;
use crate::models::TelegramName;
use crate::sheets::worktime::{find_exception_entry, find_weekday_entry};

pub async fn worktime<Cmd: WorktimeCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    params: &str,
    teacher: &TelegramName,
) -> Result<()> {
    let params: WorktimeParams = params.parse()?;
    match params {
        WorktimeParams::M0 => worktime_M0(ctx, teacher).await,
        WorktimeParams::WL => worktime_WL(ctx, teacher).await,
        WorktimeParams::WA => worktime_WA(ctx, teacher).await,
        WorktimeParams::WAW(w) => worktime_WAW(ctx, teacher, w).await,
        WorktimeParams::WAWH(w, s) => worktime_WAWH(ctx, teacher, w, s).await,
        WorktimeParams::WAWHH(w, s, e) => worktime_WAWHH(ctx, teacher, w, s, e).await,
        WorktimeParams::WAWHHF(w, s, e) => worktime_WAWHHF(ctx, teacher, w, s, e).await,
        WorktimeParams::WR => worktime_WR(ctx, teacher).await,
        WorktimeParams::WRWH(w, s) => worktime_WRWH(ctx, teacher, w, s).await,
        WorktimeParams::WRWHF(w, s) => worktime_WRWHF(ctx, teacher, w, s).await,
        WorktimeParams::EYM(y, m) => worktime_EYM(ctx, teacher, y, m).await,
        WorktimeParams::EAYM(y, m) => worktime_EAYM(ctx, teacher, y, m).await,
        WorktimeParams::ED(d) => worktime_ED(ctx, teacher, d).await,
        WorktimeParams::EDH(d, s) => worktime_EDH(ctx, teacher, d, s).await,
        WorktimeParams::EDHH(d, s, e) => worktime_EDHH(ctx, teacher, d, s, e).await,
        WorktimeParams::EDHHF(d, s, e) => worktime_EDHHF(ctx, teacher, d, s, e).await,
        WorktimeParams::ERYM(y, m) => worktime_ERYM(ctx, teacher, y, m).await,
        WorktimeParams::ERDH(d, s) => worktime_ERDH(ctx, teacher, d, s).await,
        WorktimeParams::ERDHF(d, s) => worktime_ERDHF(ctx, teacher, d, s).await,
    }
}

// ---------------------------------------------------------------------------
// M0 — main menu
// ---------------------------------------------------------------------------

async fn worktime_M0<Cmd: WorktimeCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
) -> Result<()> {
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let wd_key = CallbackKey::pack(Cmd::worktime(WorktimeParams::WL), &user_proxy).await;
    let now = chrono::Local::now();
    let ex_key = CallbackKey::pack(
        Cmd::worktime(WorktimeParams::EYM(now.year(), now.month())),
        &user_proxy,
    )
    .await;

    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback_key("📅 Week days", &wd_key),
        InlineKeyboardButton::callback_key("📆 Exceptions", &ex_key),
    ]]);

    ctx.update_markdown_message(
        markdown_format!(
            "🕐 *Worktime \\- {}*\n\nSelect category:",
            teacher.to_string()
        ),
        Some(keyboard),
    )
    .await
}

// ---------------------------------------------------------------------------
// WL — weekday list
// ---------------------------------------------------------------------------

async fn worktime_WL<Cmd: WorktimeCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
) -> Result<()> {
    let (worktime, _) = ctx.state.sheets.get_worktime().await?;
    let entries: Vec<_> = worktime
        .iter()
        .filter(|w| &w.teacher_telegram == teacher && w.day_of_week.is_some() && w.date.is_none())
        .collect();

    // Group slots by weekday, in Mon..Sun order
    let weekdays = [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ];
    let mut grouped: HashMap<Weekday, Vec<(NaiveTime, NaiveTime)>> = HashMap::new();
    for e in &entries {
        if let Some(wd) = e.day_of_week {
            grouped
                .entry(wd)
                .or_default()
                .push((e.start_time, e.end_time));
        }
    }
    for slots in grouped.values_mut() {
        slots.sort_by_key(|(s, _)| *s);
    }

    let mut text = markdown_string!("📅 *Week days*\n\n");
    if grouped.is_empty() {
        text.push(&markdown_string!("_No weekday entries\\._\n"));
    } else {
        for wd in &weekdays {
            if let Some(slots) = grouped.get(wd) {
                let slots_str: Vec<String> = slots
                    .iter()
                    .map(|(s, e)| format!("{}\u{2013}{}", s.format("%H:%M"), e.format("%H:%M")))
                    .collect();
                let line = markdown_format!("{}: {}\n", format!("{wd:?}"), slots_str.join(", "));
                text.push(&line);
            }
        }
    }

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let add_key = CallbackKey::pack(Cmd::worktime(WorktimeParams::WA), &user_proxy).await;
    let rm_key = CallbackKey::pack(Cmd::worktime(WorktimeParams::WR), &user_proxy).await;
    let back_key = CallbackKey::pack(Cmd::worktime(WorktimeParams::M0), &user_proxy).await;

    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback_key("➕ Add", &add_key),
            InlineKeyboardButton::callback_key("🗑 Remove", &rm_key),
        ],
        vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)],
    ]);

    ctx.update_markdown_message(text, Some(keyboard)).await
}

// ---------------------------------------------------------------------------
// WA — select weekday to add
// ---------------------------------------------------------------------------

async fn worktime_WA<Cmd: WorktimeCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
) -> Result<()> {
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let weekdays = [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ];
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    let mut row: Vec<InlineKeyboardButton> = Vec::new();
    for wd in weekdays {
        let key = CallbackKey::pack(Cmd::worktime(WorktimeParams::WAW(wd)), &user_proxy).await;
        row.push(InlineKeyboardButton::callback_key(format!("{wd:?}"), &key));
    }
    buttons.push(row);
    let back_key = CallbackKey::pack(Cmd::worktime(WorktimeParams::WL), &user_proxy).await;
    buttons.push(vec![InlineKeyboardButton::callback_key(
        "↩ Back", &back_key,
    )]);

    ctx.update_markdown_message(
        markdown_format!(
            "📅 *Add weekday entry \\- {}*\n\nSelect day of week:",
            teacher.to_string()
        ),
        Some(InlineKeyboardMarkup::new(buttons)),
    )
    .await
}

// ---------------------------------------------------------------------------
// WAW — weekday picked, select start hour
// ---------------------------------------------------------------------------

async fn worktime_WAW<Cmd: WorktimeCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    weekday: Weekday,
) -> Result<()> {
    let mut buttons = hour_buttons(ctx, 0, |h| {
        Cmd::worktime(WorktimeParams::WAWH(
            weekday,
            NaiveTime::from_hms_opt(h, 0, 0).unwrap(),
        ))
    })
    .await;
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(Cmd::worktime(WorktimeParams::WA), &user_proxy).await;
    buttons.push(vec![InlineKeyboardButton::callback_key(
        "↩ Back", &back_key,
    )]);

    ctx.update_markdown_message(
        markdown_format!(
            "📅 *Add weekday entry \\- {} {}*\n\nSelect start time:",
            teacher.to_string(),
            format!("{weekday:?}")
        ),
        Some(InlineKeyboardMarkup::new(buttons)),
    )
    .await
}

// ---------------------------------------------------------------------------
// WAWH — start picked, select end hour
// ---------------------------------------------------------------------------

async fn worktime_WAWH<Cmd: WorktimeCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    weekday: Weekday,
    start: NaiveTime,
) -> Result<()> {
    let mut buttons = hour_buttons(ctx, start.hour() + 1, |h| {
        Cmd::worktime(WorktimeParams::WAWHH(
            weekday,
            start,
            NaiveTime::from_hms_opt(h, 0, 0).unwrap(),
        ))
    })
    .await;
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key =
        CallbackKey::pack(Cmd::worktime(WorktimeParams::WAW(weekday)), &user_proxy).await;
    buttons.push(vec![InlineKeyboardButton::callback_key(
        "↩ Back", &back_key,
    )]);

    ctx.update_markdown_message(
        markdown_format!(
            "📅 *Add weekday entry \\- {}*\n\n{} {} –\n\nSelect end time:",
            teacher.to_string(),
            format!("{weekday:?}"),
            start.format("%H:%M").to_string()
        ),
        Some(InlineKeyboardMarkup::new(buttons)),
    )
    .await
}

// ---------------------------------------------------------------------------
// WAWHH — both times picked, show inline-query confirmation button
// ---------------------------------------------------------------------------

async fn worktime_WAWHH<Cmd: WorktimeCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    weekday: Weekday,
    start: NaiveTime,
    end: NaiveTime,
) -> Result<()> {
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(
        Cmd::worktime(WorktimeParams::WAWH(weekday, start)),
        &user_proxy,
    )
    .await;

    let forced_cmd = format!("/worktime {}", WorktimeParams::WAWHHF(weekday, start, end));
    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback_key("↩ Back", &back_key),
        InlineKeyboardButton::switch_inline_query_current_chat("✅ Add", forced_cmd),
    ]]);

    ctx.update_markdown_message(
        markdown_format!(
            "📅 *Add weekday entry \\- {}*\n\n{} {}\u{2013}{}\n\nConfirm to add this entry:",
            teacher.to_string(),
            format!("{weekday:?}"),
            start.format("%H:%M").to_string(),
            end.format("%H:%M").to_string()
        ),
        Some(keyboard),
    )
    .await
}

// ---------------------------------------------------------------------------
// WAWHHF — forced: write weekday entry
// ---------------------------------------------------------------------------

async fn worktime_WAWHHF<Cmd: WorktimeCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    weekday: Weekday,
    start: NaiveTime,
    end: NaiveTime,
) -> Result<()> {
    ctx.state
        .sheets
        .add_worktime_weekday(teacher, weekday, start, end)
        .await?;

    ctx.update_markdown_message(
        markdown_format!(
            "✅ *Worktime added\\!*\n\n{} {}\u{2013}{} added for {}\\.",
            format!("{weekday:?}"),
            start.format("%H:%M").to_string(),
            end.format("%H:%M").to_string(),
            teacher.to_string()
        ),
        None,
    )
    .await
}

// ---------------------------------------------------------------------------
// WR — weekday remove: show list of entries
// ---------------------------------------------------------------------------

async fn worktime_WR<Cmd: WorktimeCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
) -> Result<()> {
    let (worktime, _) = ctx.state.sheets.get_worktime().await?;
    let mut entries: Vec<_> = worktime
        .iter()
        .filter(|w| &w.teacher_telegram == teacher && w.day_of_week.is_some() && w.date.is_none())
        .collect();
    entries.sort_by(|a, b| {
        a.day_of_week
            .unwrap()
            .num_days_from_monday()
            .cmp(&b.day_of_week.unwrap().num_days_from_monday())
            .then(a.start_time.cmp(&b.start_time))
    });

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    if entries.is_empty() {
        let back_key = CallbackKey::pack(Cmd::worktime(WorktimeParams::WL), &user_proxy).await;
        ctx.update_markdown_message(
            markdown_string!("📅 *Remove weekday entry*\n\n_No weekday entries\\._"),
            Some(InlineKeyboardMarkup::new(vec![vec![
                InlineKeyboardButton::callback_key("↩ Back", &back_key),
            ]])),
        )
        .await?;
        return Ok(());
    }

    for e in &entries {
        let wd = e.day_of_week.unwrap();
        let label = format!(
            "{:?} {}\u{2013}{}",
            wd,
            e.start_time.format("%H:%M"),
            e.end_time.format("%H:%M")
        );
        let key = CallbackKey::pack(
            Cmd::worktime(WorktimeParams::WRWH(wd, e.start_time)),
            &user_proxy,
        )
        .await;
        buttons.push(vec![InlineKeyboardButton::callback_key(label, &key)]);
    }

    let back_key = CallbackKey::pack(Cmd::worktime(WorktimeParams::WL), &user_proxy).await;
    buttons.push(vec![InlineKeyboardButton::callback_key(
        "↩ Back", &back_key,
    )]);

    ctx.update_markdown_message(
        markdown_string!("📅 *Remove weekday entry*\n\nSelect entry to remove:"),
        Some(InlineKeyboardMarkup::new(buttons)),
    )
    .await
}

// ---------------------------------------------------------------------------
// WRWH — confirmation before removing weekday entry
// ---------------------------------------------------------------------------

async fn worktime_WRWH<Cmd: WorktimeCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    weekday: Weekday,
    start: NaiveTime,
) -> Result<()> {
    let (worktime, _) = ctx.state.sheets.get_worktime().await?;
    let period_label = find_weekday_entry(&worktime, teacher, weekday, start)
        .map(|e| {
            format!(
                "{}\u{2013}{}",
                e.start_time.format("%H:%M"),
                e.end_time.format("%H:%M")
            )
        })
        .unwrap_or_else(|| start.format("%H:%M").to_string());

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(Cmd::worktime(WorktimeParams::WR), &user_proxy).await;

    let forced_cmd = format!("/worktime {}", WorktimeParams::WRWHF(weekday, start));
    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback_key("↩ Back", &back_key),
        InlineKeyboardButton::switch_inline_query_current_chat("🗑 Remove", forced_cmd),
    ]]);

    ctx.update_markdown_message(
        markdown_format!(
            "🗑 *Remove weekday entry \\- {}*\n\n{} {}\n\nConfirm removal:",
            teacher.to_string(),
            format!("{weekday:?}"),
            period_label
        ),
        Some(keyboard),
    )
    .await
}

// ---------------------------------------------------------------------------
// WRWHF — forced: delete weekday entry
// ---------------------------------------------------------------------------

async fn worktime_WRWHF<Cmd: WorktimeCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    weekday: Weekday,
    start: NaiveTime,
) -> Result<()> {
    let (worktime, _) = ctx.state.sheets.get_worktime().await?;
    let period_label = find_weekday_entry(&worktime, teacher, weekday, start)
        .map(|e| {
            format!(
                "{}\u{2013}{}",
                e.start_time.format("%H:%M"),
                e.end_time.format("%H:%M")
            )
        })
        .unwrap_or_else(|| start.format("%H:%M").to_string());

    ctx.state
        .sheets
        .delete_worktime_weekday(teacher, weekday, start)
        .await?;

    ctx.update_markdown_message(
        markdown_format!(
            "✅ *Worktime removed\\!*\n\n{} {} removed for {}\\.",
            format!("{weekday:?}"),
            period_label,
            teacher.to_string()
        ),
        None,
    )
    .await
}

// ---------------------------------------------------------------------------
// EYM — exception list for year/month
// ---------------------------------------------------------------------------

async fn worktime_EYM<Cmd: WorktimeCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    year: i32,
    month: u32,
) -> Result<()> {
    let (worktime, _) = ctx.state.sheets.get_worktime().await?;
    let mut entries: Vec<_> = worktime
        .iter()
        .filter(|w| {
            &w.teacher_telegram == teacher
                && w.date.is_some()
                && w.date
                    .map(|d| d.year() == year && d.month() == month)
                    .unwrap_or(false)
        })
        .collect();
    entries.sort_by_key(|e| (e.date, e.start_time));

    let month_label = NaiveDate::from_ymd_opt(year, month, 1)
        .map(|d| d.format("%B %Y").to_string())
        .unwrap_or_else(|| format!("{}/{}", month, year));

    let mut text = markdown_format!("📆 *Exceptions \\- {}*\n\n", month_label);
    if entries.is_empty() {
        text.push(&markdown_string!("_No exceptions for this month\\._\n"));
    } else {
        let mut cur_date: Option<NaiveDate> = None;
        for e in &entries {
            let d = e.date.unwrap();
            if cur_date != Some(d) {
                cur_date = Some(d);
                text.push(&markdown_format!("*{}*\n", d.format("%d %b").to_string()));
            }
            let line = markdown_format!(
                "  {}\u{2013}{}\n",
                e.start_time.format("%H:%M").to_string(),
                e.end_time.format("%H:%M").to_string()
            );
            text.push(&line);
        }
    }

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let (py, pm) = prev_month(year, month);
    let (ny, nm) = next_month(year, month);
    let prev_label = format!(
        "< {}",
        NaiveDate::from_ymd_opt(py, pm, 1)
            .map(|d| d.format("%b %Y").to_string())
            .unwrap_or_default()
    );
    let next_label = format!(
        "{} >",
        NaiveDate::from_ymd_opt(ny, nm, 1)
            .map(|d| d.format("%b %Y").to_string())
            .unwrap_or_default()
    );
    let prev_key = CallbackKey::pack(Cmd::worktime(WorktimeParams::EYM(py, pm)), &user_proxy).await;
    let next_key = CallbackKey::pack(Cmd::worktime(WorktimeParams::EYM(ny, nm)), &user_proxy).await;
    let add_key = CallbackKey::pack(
        Cmd::worktime(WorktimeParams::EAYM(year, month)),
        &user_proxy,
    )
    .await;
    let rm_key = CallbackKey::pack(
        Cmd::worktime(WorktimeParams::ERYM(year, month)),
        &user_proxy,
    )
    .await;
    let back_key = CallbackKey::pack(Cmd::worktime(WorktimeParams::M0), &user_proxy).await;

    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback_key(prev_label, &prev_key),
            InlineKeyboardButton::callback_key(next_label, &next_key),
        ],
        vec![
            InlineKeyboardButton::callback_key("➕ Add", &add_key),
            InlineKeyboardButton::callback_key("🗑 Remove", &rm_key),
        ],
        vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)],
    ]);

    ctx.update_markdown_message(text, Some(keyboard)).await
}

// ---------------------------------------------------------------------------
// EAYM — exception add: show calendar
// ---------------------------------------------------------------------------

async fn worktime_EAYM<Cmd: WorktimeCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    year: i32,
    month: u32,
) -> Result<()> {
    use crate::sheets::worktime::DayAvailability;

    let (worktime, _) = ctx.state.sheets.get_worktime().await?;
    let marked: HashMap<NaiveDate, DayAvailability> = worktime
        .iter()
        .filter(|w| {
            &w.teacher_telegram == teacher
                && w.date
                    .map(|d| d.year() == year && d.month() == month)
                    .unwrap_or(false)
        })
        .filter_map(|w| w.date)
        .map(|d| (d, DayAvailability::Marked))
        .collect();

    let back = Cmd::worktime(WorktimeParams::EYM(year, month));

    show_date_selection(
        ctx,
        markdown_format!(
            "📆 *Add exception \\- {}*\n\nSelect date:",
            teacher.to_string()
        ),
        year,
        month,
        move |date| Cmd::worktime(WorktimeParams::ED(date)),
        move |py, pm| Cmd::worktime(WorktimeParams::EAYM(py, pm)),
        move |ny, nm| Cmd::worktime(WorktimeParams::EAYM(ny, nm)),
        move || Cmd::worktime(WorktimeParams::EAYM(year, month)),
        move || Cmd::worktime(WorktimeParams::EAYM(year, month)),
        Some(back),
        &marked,
    )
    .await
}

// ---------------------------------------------------------------------------
// ED — exception date selected, pick start hour
// ---------------------------------------------------------------------------

async fn worktime_ED<Cmd: WorktimeCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    date: NaiveDate,
) -> Result<()> {
    let mut buttons = hour_buttons(ctx, 0, |h| {
        Cmd::worktime(WorktimeParams::EDH(
            date,
            NaiveTime::from_hms_opt(h, 0, 0).unwrap(),
        ))
    })
    .await;
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(
        Cmd::worktime(WorktimeParams::EAYM(date.year(), date.month())),
        &user_proxy,
    )
    .await;
    buttons.push(vec![InlineKeyboardButton::callback_key(
        "↩ Back", &back_key,
    )]);

    ctx.update_markdown_message(
        markdown_format!(
            "📆 *Add exception \\- {} {}*\n\nSelect start time:",
            teacher.to_string(),
            date.format("%d %b %Y").to_string()
        ),
        Some(InlineKeyboardMarkup::new(buttons)),
    )
    .await
}

// ---------------------------------------------------------------------------
// EDH — start picked, select end hour
// ---------------------------------------------------------------------------

async fn worktime_EDH<Cmd: WorktimeCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    date: NaiveDate,
    start: NaiveTime,
) -> Result<()> {
    let mut buttons = hour_buttons(ctx, start.hour() + 1, |h| {
        Cmd::worktime(WorktimeParams::EDHH(
            date,
            start,
            NaiveTime::from_hms_opt(h, 0, 0).unwrap(),
        ))
    })
    .await;
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(Cmd::worktime(WorktimeParams::ED(date)), &user_proxy).await;
    buttons.push(vec![InlineKeyboardButton::callback_key(
        "↩ Back", &back_key,
    )]);

    ctx.update_markdown_message(
        markdown_format!(
            "📆 *Add exception \\- {}*\n\n{} {} –\n\nSelect end time:",
            teacher.to_string(),
            date.format("%d %b %Y").to_string(),
            start.format("%H:%M").to_string()
        ),
        Some(InlineKeyboardMarkup::new(buttons)),
    )
    .await
}

// ---------------------------------------------------------------------------
// EDHH — both times, show inline-query confirmation
// ---------------------------------------------------------------------------

async fn worktime_EDHH<Cmd: WorktimeCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    date: NaiveDate,
    start: NaiveTime,
    end: NaiveTime,
) -> Result<()> {
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key =
        CallbackKey::pack(Cmd::worktime(WorktimeParams::EDH(date, start)), &user_proxy).await;

    let forced_cmd = format!("/worktime {}", WorktimeParams::EDHHF(date, start, end));
    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback_key("↩ Back", &back_key),
        InlineKeyboardButton::switch_inline_query_current_chat("✅ Add", forced_cmd),
    ]]);

    ctx.update_markdown_message(
        markdown_format!(
            "📆 *Add exception \\- {}*\n\n{} {}\u{2013}{}\n\nConfirm to add this entry:",
            teacher.to_string(),
            date.format("%d %b %Y").to_string(),
            start.format("%H:%M").to_string(),
            end.format("%H:%M").to_string()
        ),
        Some(keyboard),
    )
    .await
}

// ---------------------------------------------------------------------------
// EDHHF — forced: write exception entry
// ---------------------------------------------------------------------------

async fn worktime_EDHHF<Cmd: WorktimeCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    date: NaiveDate,
    start: NaiveTime,
    end: NaiveTime,
) -> Result<()> {
    ctx.state
        .sheets
        .add_worktime_exception(teacher, date, start, end)
        .await?;

    ctx.update_markdown_message(
        markdown_format!(
            "✅ *Worktime added\\!*\n\n{} {}\u{2013}{} added for {}\\.",
            date.format("%d %b %Y").to_string(),
            start.format("%H:%M").to_string(),
            end.format("%H:%M").to_string(),
            teacher.to_string()
        ),
        None,
    )
    .await
}

// ---------------------------------------------------------------------------
// ERYM — exception remove: show dates for month
// ---------------------------------------------------------------------------

async fn worktime_ERYM<Cmd: WorktimeCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    year: i32,
    month: u32,
) -> Result<()> {
    let (worktime, _) = ctx.state.sheets.get_worktime().await?;
    let mut entries: Vec<_> = worktime
        .iter()
        .filter(|w| {
            &w.teacher_telegram == teacher
                && w.date
                    .map(|d| d.year() == year && d.month() == month)
                    .unwrap_or(false)
        })
        .collect();
    entries.sort_by_key(|e| (e.date, e.start_time));

    let month_label = NaiveDate::from_ymd_opt(year, month, 1)
        .map(|d| d.format("%B %Y").to_string())
        .unwrap_or_else(|| format!("{}/{}", month, year));

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let (py, pm) = prev_month(year, month);
    let (ny, nm) = next_month(year, month);
    let prev_key =
        CallbackKey::pack(Cmd::worktime(WorktimeParams::ERYM(py, pm)), &user_proxy).await;
    let next_key =
        CallbackKey::pack(Cmd::worktime(WorktimeParams::ERYM(ny, nm)), &user_proxy).await;
    let back_key =
        CallbackKey::pack(Cmd::worktime(WorktimeParams::EYM(year, month)), &user_proxy).await;

    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    if entries.is_empty() {
        let prev_label = format!(
            "< {}",
            NaiveDate::from_ymd_opt(py, pm, 1)
                .map(|d| d.format("%b %Y").to_string())
                .unwrap_or_default()
        );
        let next_label = format!(
            "{} >",
            NaiveDate::from_ymd_opt(ny, nm, 1)
                .map(|d| d.format("%b %Y").to_string())
                .unwrap_or_default()
        );
        buttons.push(vec![
            InlineKeyboardButton::callback_key(prev_label, &prev_key),
            InlineKeyboardButton::callback_key(next_label, &next_key),
        ]);
        buttons.push(vec![InlineKeyboardButton::callback_key(
            "↩ Back", &back_key,
        )]);

        ctx.update_markdown_message(
            markdown_format!(
                "🗑 *Remove exception \\- {}*\n\n_No exceptions for this month\\._",
                month_label
            ),
            Some(InlineKeyboardMarkup::new(buttons)),
        )
        .await?;
        return Ok(());
    }

    for e in &entries {
        let d = e.date.unwrap();
        let label = format!(
            "{} {}\u{2013}{}",
            d.format("%d %b"),
            e.start_time.format("%H:%M"),
            e.end_time.format("%H:%M")
        );
        let key = CallbackKey::pack(
            Cmd::worktime(WorktimeParams::ERDH(d, e.start_time)),
            &user_proxy,
        )
        .await;
        buttons.push(vec![InlineKeyboardButton::callback_key(label, &key)]);
    }

    let prev_label = format!(
        "< {}",
        NaiveDate::from_ymd_opt(py, pm, 1)
            .map(|d| d.format("%b %Y").to_string())
            .unwrap_or_default()
    );
    let next_label = format!(
        "{} >",
        NaiveDate::from_ymd_opt(ny, nm, 1)
            .map(|d| d.format("%b %Y").to_string())
            .unwrap_or_default()
    );
    buttons.push(vec![
        InlineKeyboardButton::callback_key(prev_label, &prev_key),
        InlineKeyboardButton::callback_key(next_label, &next_key),
    ]);
    buttons.push(vec![InlineKeyboardButton::callback_key(
        "↩ Back", &back_key,
    )]);

    ctx.update_markdown_message(
        markdown_format!(
            "🗑 *Remove exception \\- {}*\n\nSelect date to remove:",
            month_label
        ),
        Some(InlineKeyboardMarkup::new(buttons)),
    )
    .await
}

// ---------------------------------------------------------------------------
// ERDH — confirmation before removing a specific exception slot
// ---------------------------------------------------------------------------

async fn worktime_ERDH<Cmd: WorktimeCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    date: NaiveDate,
    start: NaiveTime,
) -> Result<()> {
    let (worktime, _) = ctx.state.sheets.get_worktime().await?;
    let period_label = find_exception_entry(&worktime, teacher, date, start)
        .map(|e| {
            format!(
                "{}\u{2013}{}",
                e.start_time.format("%H:%M"),
                e.end_time.format("%H:%M")
            )
        })
        .unwrap_or_else(|| start.format("%H:%M").to_string());

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(
        Cmd::worktime(WorktimeParams::ERYM(date.year(), date.month())),
        &user_proxy,
    )
    .await;

    let forced_cmd = format!("/worktime {}", WorktimeParams::ERDHF(date, start));
    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback_key("↩ Back", &back_key),
        InlineKeyboardButton::switch_inline_query_current_chat("🗑 Remove", forced_cmd),
    ]]);

    ctx.update_markdown_message(
        markdown_format!(
            "🗑 *Remove exception \\- {}*\n\n{} {}\n\nConfirm removal:",
            teacher.to_string(),
            date.format("%d %b %Y").to_string(),
            period_label
        ),
        Some(keyboard),
    )
    .await
}

// ---------------------------------------------------------------------------
// ERDHF — forced: delete specific exception slot
// ---------------------------------------------------------------------------

async fn worktime_ERDHF<Cmd: WorktimeCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    date: NaiveDate,
    start: NaiveTime,
) -> Result<()> {
    let (worktime, _) = ctx.state.sheets.get_worktime().await?;
    let period_label = find_exception_entry(&worktime, teacher, date, start)
        .map(|e| {
            format!(
                "{}\u{2013}{}",
                e.start_time.format("%H:%M"),
                e.end_time.format("%H:%M")
            )
        })
        .unwrap_or_else(|| start.format("%H:%M").to_string());

    ctx.state
        .sheets
        .delete_worktime_exception(teacher, date, start)
        .await?;

    ctx.update_markdown_message(
        markdown_format!(
            "✅ *Worktime removed\\!*\n\n{} {} removed for {}\\.",
            date.format("%d %b %Y").to_string(),
            period_label,
            teacher.to_string()
        ),
        None,
    )
    .await
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn hour_buttons<Cmd, F>(
    ctx: &BotCtx<Cmd>,
    from_hour: u32,
    make_cmd: F,
) -> Vec<Vec<InlineKeyboardButton>>
where
    Cmd: CallbackBitcode + 'static,
    F: Fn(u32) -> Cmd,
{
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    let mut row: Vec<InlineKeyboardButton> = Vec::new();
    for h in from_hour..24 {
        let label = format!("{:02}:00", h);
        let key = CallbackKey::pack(make_cmd(h), &user_proxy).await;
        row.push(InlineKeyboardButton::callback_key(label, &key));
        if row.len() == 4 {
            buttons.push(row);
            row = Vec::new();
        }
    }
    if !row.is_empty() {
        buttons.push(row);
    }
    buttons
}

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
