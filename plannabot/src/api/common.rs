use crate::models::UserRole;
use crate::state::BotState;
use anyhow::Result;
use std::sync::Arc;
use telluride::markdown::MarkdownStringMessage;
use telluride::{markdown_format, markdown_string};
use teloxide::prelude::*;

/// Reset user to default state and show a role-appropriate welcome message.
///
/// Clears any active impersonation or admin mode (no-ops if not active), then
/// sends a welcome message. This is the single implementation for all four
/// modes' Start command.
pub async fn start(bot: &Bot, chat_id: ChatId, role: &UserRole, state: &Arc<BotState>) -> Result<()> {
    state.clear_impersonation(chat_id).await;
    state.exit_admin_mode(chat_id).await;

    let name = match role {
        UserRole::Student(s) => s.name.clone(),
        UserRole::Teacher(t) => t.telegram_name.to_string(),
    };

    let mut text = markdown_string!("👋 *Welcome to Plannabot\\!*\n\n");
    let greeting = markdown_format!(
        "Hello, {}\\! Use /help to see available commands\\.",
        name
    );
    text.push(&greeting);

    if let UserRole::Teacher(teacher) = role
        && state.is_both_teacher_and_student(teacher.telegram_name.as_str()).await
    {
        let info = markdown_string!(
            "\n\n📌 *Note:* You are registered as both a teacher and a student\\."
        );
        text.push(&info);
    }

    bot.send_markdown_message(chat_id, text).await?;
    Ok(())
}

/// Formats a lesson duration in minutes to a human-readable string.
///
/// - `90`  → `"1h 30m"`
/// - `120` → `"2h"`
/// - `45`  → `"45m"`
pub fn format_duration(minutes: i64) -> String {
    let hours = minutes / 60;
    let mins = minutes % 60;
    match (hours, mins) {
        (0, m) => format!("{}m", m),
        (h, 0) => format!("{}h", h),
        (h, m) => format!("{}h {}m", h, m),
    }
}
