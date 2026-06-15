#![allow(non_snake_case)]
use anyhow::Result;
use chrono::{Datelike, Duration as ChronoDuration, NaiveDate, Weekday};
use chrono_tz::Tz;
use telluride::command::{CallbackBitcode, CallbackKey, InlineKeyboardButtonPackedExt};
use telluride::data_store::UserProxy;
use telluride::{markdown_format, markdown_string};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::api::common::format_entry_label;
use crate::api::context::BotCtx;
use crate::api::traits::{BookingActor, ScheduleCommand, ScheduleParams};

pub async fn schedule<Cmd: ScheduleCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    params: &str,
    actor: &BookingActor,
    tz: Tz,
) -> Result<()> {
    match params.parse::<ScheduleParams>()? {
        ScheduleParams::L0 => schedule_L1(ctx, actor, tz, 0).await,
        ScheduleParams::L1(w) => schedule_L1(ctx, actor, tz, w).await,
    }
}

async fn schedule_L1<Cmd: ScheduleCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    actor: &BookingActor,
    tz: Tz,
    week_offset: i32,
) -> Result<()> {
    let monday = week_monday(tz, week_offset);
    let sunday = monday + ChronoDuration::days(6);

    let (all_entries, _) = ctx.state.sheets.get_schedule().await?;
    let mut entries: Vec<_> = all_entries
        .into_iter()
        .filter(|e| match actor {
            BookingActor::Student(s) => &e.student_telegram == s,
            BookingActor::Teacher(t) => &e.teacher_telegram == t,
        })
        .filter(|e| {
            let d = e.datetime.with_timezone(&tz).date_naive();
            d >= monday && d <= sunday
        })
        .collect();
    entries.sort_by_key(|e| e.datetime);

    let mut text = markdown_format!(
        "📅 *Week: {} — {}*\n\n",
        monday.format("%d %b").to_string(),
        sunday.format("%d %b %Y").to_string()
    );

    for i in 0u64..7 {
        let day = monday + ChronoDuration::days(i as i64);
        text.push(&markdown_format!(
            "*{} {}*\n",
            weekday_abbr(day.weekday()),
            day.format("%d %b").to_string()
        ));
        let day_entries: Vec<_> = entries
            .iter()
            .filter(|e| e.datetime.with_timezone(&tz).date_naive() == day)
            .collect();
        if day_entries.is_empty() {
            text.push(&markdown_string!("_no lessons_\n"));
        } else {
            for entry in day_entries {
                let label = format_entry_label(entry, actor);
                text.push(&markdown_format!("{}\n", label));
            }
        }
        text.push(&markdown_string!("\n"));
    }

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let prev_key = CallbackKey::pack(
        Cmd::schedule(ScheduleParams::L1(week_offset - 1)),
        &user_proxy,
    )
    .await;
    let this_key = CallbackKey::pack(Cmd::schedule(ScheduleParams::L1(0)), &user_proxy).await;
    let next_key = CallbackKey::pack(
        Cmd::schedule(ScheduleParams::L1(week_offset + 1)),
        &user_proxy,
    )
    .await;
    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback_key("<", &prev_key),
        InlineKeyboardButton::callback_key("This week", &this_key),
        InlineKeyboardButton::callback_key(">", &next_key),
    ]]);

    ctx.update_markdown_message(text, Some(keyboard)).await
}

fn week_monday(tz: Tz, week_offset: i32) -> NaiveDate {
    let today = chrono::Utc::now().with_timezone(&tz).date_naive();
    let this_monday = today - ChronoDuration::days(today.weekday().num_days_from_monday() as i64);
    this_monday + ChronoDuration::weeks(week_offset as i64)
}

fn weekday_abbr(wd: Weekday) -> &'static str {
    match wd {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    }
}
