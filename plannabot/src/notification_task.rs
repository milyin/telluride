use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use telluride::markdown::MarkdownStringMessage;
use teloxide::prelude::*;
use teloxide::types::ChatId;

use crate::api::notification::format_notification_message;
use crate::api::traits::BookParams;
use crate::state::BotState;

const TICK_INTERVAL: Duration = Duration::from_secs(30);

pub async fn run(bot: Bot, state: Arc<BotState>) {
    let mut interval = tokio::time::interval(TICK_INTERVAL);
    loop {
        interval.tick().await;
        if let Err(e) = check_and_send(&bot, &state).await {
            log::error!("Notification task error: {e}");
        }
    }
}

async fn check_and_send(bot: &Bot, state: &Arc<BotState>) -> anyhow::Result<()> {
    let now = Utc::now();

    let students = state.get_all_students().await;

    let students_with_delay: Vec<_> = students
        .into_iter()
        .filter(|s| s.notification_delay.is_some() && s.chat_id != 0)
        .collect();

    if students_with_delay.is_empty() {
        return Ok(());
    }

    let (schedule, _) = state.sheets.get_schedule().await?;

    for student in students_with_delay {
        let delay = student.notification_delay.as_ref().unwrap();
        let delay_chrono =
            chrono::Duration::from_std(*delay.as_ref()).unwrap_or(chrono::Duration::zero());

        let next_lesson = schedule
            .iter()
            .filter(|e| {
                e.student_telegram == student.telegram_name && e.is_planned() && e.datetime > now
            })
            .min_by_key(|e| e.datetime);

        let Some(lesson) = next_lesson else {
            continue;
        };

        let notify_at = lesson.datetime - delay_chrono;
        if notify_at > now {
            continue;
        }

        if state
            .is_notification_sent(&student.telegram_name, lesson.datetime)
            .await
        {
            continue;
        }

        let pairing = state
            .get_pairing(&student.telegram_name, &lesson.teacher_telegram)
            .await;
        let zoom = pairing.as_ref().and_then(|p| p.zoom_url.as_deref());
        let board = pairing.as_ref().and_then(|p| p.board_url.as_deref());

        let text = format_notification_message(
            lesson.datetime,
            &lesson.teacher_telegram,
            lesson.duration_minutes,
            zoom,
            board,
            student.timezone,
        );

        let lesson_cmd = format!(
            "/book {}",
            BookParams::L1(
                lesson.teacher_telegram.clone(),
                lesson.student_telegram.clone(),
                lesson.datetime.date_naive(),
                lesson.datetime.time(),
            )
        );
        let keyboard = teloxide::types::InlineKeyboardMarkup::new(vec![vec![
            teloxide::types::InlineKeyboardButton::switch_inline_query_current_chat(
                "📋 Lesson Details",
                lesson_cmd,
            ),
        ]]);

        let chat_id = ChatId(student.chat_id);
        let result = bot
            .send_markdown_message(chat_id, text)
            .reply_markup(keyboard)
            .await;

        match result {
            Ok(_) => {
                state
                    .mark_notification_sent(student.telegram_name.clone(), lesson.datetime)
                    .await;
                log::info!(
                    "Sent lesson notification to {} for lesson at {}",
                    student.telegram_name,
                    lesson.datetime
                );
            }
            Err(e) => {
                log::error!(
                    "Failed to send notification to {}: {e}",
                    student.telegram_name
                );
            }
        }
    }

    Ok(())
}
