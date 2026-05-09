#![allow(non_snake_case)]
use anyhow::Result;
use chrono::Utc;
use telluride::command::{CallbackBitcode, CallbackKey, InlineKeyboardButtonPackedExt};
use telluride::data_store::UserProxy;
use telluride::markdown::MarkdownString;
use telluride::{markdown_format, markdown_string};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::api::common::format_duration;
use crate::api::context::BotCtx;
use crate::api::traits::{BookCommand, BookParams, NotificationCommand, NotificationParams};
use crate::models::Student;
use crate::types::Duration;

pub async fn notification<Cmd: NotificationCommand + BookCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    params: &str,
    student: &Student,
) -> Result<()> {
    match params.parse::<NotificationParams>()? {
        NotificationParams::N0 => notification_N0(ctx, student).await,
        NotificationParams::S0 => notification_S0(ctx, student).await,
        NotificationParams::SF(dur) => notification_SF(ctx, student, dur).await,
    }
}

async fn notification_N0<Cmd: NotificationCommand + BookCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    student: &Student,
) -> Result<()> {
    let (all_entries, _) = ctx.state.sheets.get_schedule().await?;
    let now = Utc::now();

    let next_lesson = all_entries
        .iter()
        .filter(|e| e.student_telegram == student.telegram_name && e.is_planned() && e.datetime > now)
        .min_by_key(|e| e.datetime);

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let setup_key = CallbackKey::pack(
        Cmd::notification(NotificationParams::S0),
        &user_proxy,
    )
    .await;

    let delay_str = match &student.notification_delay {
        None => markdown_string!("_not set_"),
        Some(d) => markdown_format!("{} before", MarkdownString::escape(d.to_string())),
    };

    let Some(lesson) = next_lesson else {
        let mut text = markdown_string!("🔔 *Notifications*\n\n");
        text.push(&markdown_string!("No upcoming lessons scheduled\\.\n\n"));
        text.push(&markdown_format!("🔔 Notification: {}\n", delay_str));
        let keyboard = InlineKeyboardMarkup::new(vec![vec![
            InlineKeyboardButton::callback_key("⚙️ Setup", &setup_key),
        ]]);
        return ctx.update_markdown_message(text, Some(keyboard)).await;
    };

    let pairing = ctx.state.get_pairing(&student.telegram_name, &lesson.teacher_telegram).await;

    let tz = student.timezone;
    let local_dt = lesson.datetime.with_timezone(&tz);

    let mut text = markdown_string!("🔔 *Next Lesson*\n\n");
    text.push(&markdown_format!(
        "🏫 Teacher: {}\n",
        lesson.teacher_telegram.to_string()
    ));
    text.push(&markdown_format!(
        "📆 Date: {}\n",
        local_dt.format("%Y\\-%m\\-%d").to_string()
    ));
    text.push(&markdown_format!(
        "⏰ Time: {}\n",
        local_dt.format("%H:%M").to_string()
    ));
    text.push(&markdown_format!(
        "⏱ Duration: {}\n",
        format_duration(lesson.duration_minutes as i64)
    ));

    if let Some(p) = &pairing {
        if let Some(zoom) = &p.zoom_url {
            text.push(&markdown_format!(
                "🎥 Zoom: {}\n",
                MarkdownString::escape(zoom.clone())
            ));
        }
        if let Some(board) = &p.board_url {
            text.push(&markdown_format!(
                "📋 Board: {}\n",
                MarkdownString::escape(board.clone())
            ));
        }
    }

    text.push(&markdown_format!("\n🔔 Notification: {}\n", delay_str));

    let lesson_params = BookParams::L1(
        lesson.teacher_telegram.clone(),
        lesson.student_telegram.clone(),
        lesson.datetime.date_naive(),
        lesson.datetime.time(),
    );
    let lesson_key = CallbackKey::pack(Cmd::book(lesson_params), &user_proxy).await;

    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback_key("📋 Lesson", &lesson_key),
        InlineKeyboardButton::callback_key("⚙️ Setup", &setup_key),
    ]]);

    ctx.update_markdown_message(text, Some(keyboard)).await
}

async fn notification_S0<Cmd: NotificationCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    student: &Student,
) -> Result<()> {
    let delay_str = match &student.notification_delay {
        None => "not set".to_string(),
        Some(d) => format!("{} before the lesson", d),
    };

    let prefill = format!(
        "/notification {}",
        NotificationParams::SF(Duration::from(std::time::Duration::from_secs(3600)))
    );

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(
        Cmd::notification(NotificationParams::N0),
        &user_proxy,
    )
    .await;

    let text = markdown_format!(
        "⚙️ *Notification Setup*\n\n\
        Current setting: {}\n\n\
        Press *Set\\.\\.\\.* to prefill the command, then replace `1h` with your preferred delay \\(e\\.g\\. `30m`, `2h`\\) and send\\.\n\
        Send `0s` to disable notifications\\.",
        MarkdownString::escape(delay_str)
    );

    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::switch_inline_query_current_chat("✏️ Set...", prefill)],
        vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)],
    ]);

    ctx.update_markdown_message(text, Some(keyboard)).await
}

async fn notification_SF<Cmd: Send + Sync + Clone>(
    ctx: &BotCtx<Cmd>,
    student: &Student,
    duration: Duration,
) -> Result<()> {
    let disable = duration.as_secs() == 0;

    ctx.state
        .sheets
        .update_student_notification_delay(
            &student.telegram_name,
            if disable { None } else { Some(&duration) },
        )
        .await?;

    let _ = ctx.state.refresh().await;

    let text = if disable {
        markdown_string!("✅ Notifications disabled\\.")
    } else {
        markdown_format!(
            "✅ Notification set: {} before the lesson\\.",
            MarkdownString::escape(duration.to_string())
        )
    };

    ctx.update_markdown_message(text, None).await
}

/// Formats a lesson notification message for the background task.
pub fn format_notification_message(
    lesson_datetime: chrono::DateTime<Utc>,
    teacher: &crate::models::TelegramName,
    duration_minutes: u64,
    zoom_url: Option<&str>,
    board_url: Option<&str>,
    tz: chrono_tz::Tz,
) -> MarkdownString {
    let local_dt = lesson_datetime.with_timezone(&tz);

    let mut text = markdown_string!("🔔 *Upcoming Lesson Reminder*\n\n");
    text.push(&markdown_format!("🏫 Teacher: {}\n", teacher.to_string()));
    text.push(&markdown_format!(
        "📆 {}\n",
        local_dt.format("%Y\\-%m\\-%d at %H:%M").to_string()
    ));
    text.push(&markdown_format!(
        "⏱ Duration: {}\n",
        format_duration(duration_minutes as i64)
    ));

    if let Some(zoom) = zoom_url {
        if !zoom.is_empty() {
            text.push(&markdown_format!(
                "🎥 Zoom: {}\n",
                MarkdownString::escape(zoom.to_string())
            ));
        }
    }
    if let Some(board) = board_url {
        if !board.is_empty() {
            text.push(&markdown_format!(
                "📋 Board: {}\n",
                MarkdownString::escape(board.to_string())
            ));
        }
    }

    text
}
