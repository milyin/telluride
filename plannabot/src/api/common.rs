use crate::api::context::BotCtx;
use crate::api::traits::BookingActor;
use crate::models::{LessonStatus, ScheduleEntry, UserRole};
use anyhow::Result;
use rusty_money::iso;
use telluride::markdown::{MarkdownString, MarkdownStringMessage};
use telluride::{markdown_format, markdown_string};

pub async fn start(ctx: &BotCtx<impl Send + Sync + Clone>, username: &str) -> Result<()> {
    ctx.state.clear_student_impersonation(ctx.chat_id).await;
    ctx.state.clear_teacher_impersonation(ctx.chat_id).await;
    ctx.state.exit_admin_mode(ctx.chat_id).await;

    let role = ctx.state.get_role(username).await;

    let mut text = markdown_string!("👋 *Welcome to Plannabot\\!*\n\n");
    let greeting = markdown_format!(
        "Hello, {}\\! Use /help to see available commands\\.",
        role.as_ref().map(|r| r.name()).unwrap_or(username)
    );
    text.push(&greeting);

    if let Some(UserRole::Teacher(teacher)) = &role {
        if ctx.state
            .is_both_teacher_and_student(teacher.telegram_name.as_str())
            .await
        {
            let info = markdown_string!(
                "\n\n📌 *Note:* You are registered as both a teacher and a student\\."
            );
            text.push(&info);
        }
    }

    ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
    Ok(())
}

pub fn status_label(status: &Option<LessonStatus>) -> &'static str {
    match status {
        None => "📅",
        Some(LessonStatus::Passed) => "✅",
        Some(LessonStatus::Absent) => "🚫",
    }
}

pub fn format_entry_label(entry: &ScheduleEntry, actor: &BookingActor) -> String {
    let (role_icon, other) = match actor {
        BookingActor::Student(_) => ("🏫", entry.teacher_telegram.to_string()),
        BookingActor::Teacher(_) => ("🎓", entry.student_telegram.to_string()),
    };
    let duration = format_duration(entry.duration_minutes as i64);
    format!(
        "{} {} {} {} {}",
        status_label(&entry.status),
        entry.datetime.time().format("%H:%M"),
        duration,
        role_icon,
        other
    )
}

pub struct PageInfo {
    pub start: usize,
    pub end: usize,
    pub page: i32,
    pub total: usize,
}

impl PageInfo {
    pub const PAGE_SIZE: usize = 10;
    pub fn new(page: i32, total: usize) -> Self {
        let page_idx = page.max(0) as usize;
        let start = (page_idx * Self::PAGE_SIZE).min(total);
        let end = (start + Self::PAGE_SIZE).min(total);
        Self { start, end, page, total }
    }
    pub fn has_prev(&self) -> bool { self.page > 0 }
    pub fn has_next(&self) -> bool { self.end < self.total }
}

/// Plain formatted money for button labels: "100 $" or "-100 $".
pub fn fmt_money_plain(amount: i64, currency: Option<&'static iso::Currency>) -> String {
    match currency {
        Some(c) => format!("{} {}", amount, c.symbol),
        None => amount.to_string(),
    }
}

/// MarkdownString formatted money for message text: negative amounts are bold.
pub fn fmt_money(amount: i64, currency: Option<&'static iso::Currency>) -> MarkdownString {
    let Some(c) = currency else {
        return MarkdownString::escape(amount.to_string());
    };
    let text = format!("{} {}", amount, c.symbol);
    if amount < 0 {
        let escaped = MarkdownString::escape(&text).into_string();
        MarkdownString::from_validated_string(format!("*{}*", escaped))
    } else {
        MarkdownString::escape(&text)
    }
}

pub fn format_duration(minutes: i64) -> String {
    let hours = minutes / 60;
    let mins = minutes % 60;
    match (hours, mins) {
        (0, m) => format!("{}m", m),
        (h, 0) => format!("{}h", h),
        (h, m) => format!("{}h{}m", h, m),
    }
}
