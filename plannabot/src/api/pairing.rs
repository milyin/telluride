use crate::api::common::PageInfo;
use crate::api::context::BotCtx;
use crate::api::traits::{PairingParams, StudentCommand};
use crate::models::{TelegramName, TeacherStudentPairing};
use anyhow::Result;
use telluride::command::{CallbackBitcode, CallbackKey, InlineKeyboardButtonPackedExt};
use telluride::data_store::UserProxy;
use telluride::markdown::MarkdownString;
use telluride::{markdown_format, markdown_string};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

const PAGE_SIZE: usize = 12;
const COLS: usize = 3;

pub async fn student<Cmd: StudentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    params: &str,
    teacher: &TelegramName,
) -> Result<()> {
    let params: PairingParams = params.parse()?;
    match params {
        PairingParams::P0 => show_list(ctx, teacher).await,
        PairingParams::PA(page) => show_add_selection(ctx, teacher, page).await,
        PairingParams::PAN(name) => show_add_form(ctx, teacher, name).await,
        PairingParams::PANF(name, dur, cost) => exec_add(ctx, teacher, name, dur, cost).await,
        PairingParams::PE(page) => show_edit_selection(ctx, teacher, page).await,
        PairingParams::PEN(name) => show_edit_form(ctx, teacher, name).await,
        PairingParams::PENF(name, dur, cost) => exec_edit(ctx, teacher, name, dur, cost).await,
        PairingParams::PR(page) => show_remove_selection(ctx, teacher, page).await,
        PairingParams::PRN(name) => show_remove_confirm(ctx, teacher, name).await,
        PairingParams::PRNF(name) => exec_remove(ctx, teacher, name).await,
    }
}

// ---------------------------------------------------------------------------
// List view
// ---------------------------------------------------------------------------

async fn show_list<Cmd: StudentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
) -> Result<()> {
    let pairings = ctx.state.get_pairings_for_teacher(teacher).await;
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);

    let add_key = CallbackKey::pack(Cmd::student(PairingParams::PA(0)), &user_proxy).await;
    let edit_key = CallbackKey::pack(Cmd::student(PairingParams::PE(0)), &user_proxy).await;
    let remove_key = CallbackKey::pack(Cmd::student(PairingParams::PR(0)), &user_proxy).await;

    let text = if pairings.is_empty() {
        markdown_string!("*Your students:* none assigned\\.")
    } else {
        let mut msg = markdown_string!("*Your students:*");
        for p in &pairings {
            msg.push(&markdown_format!("\n• {} — {} min, cost {}", p.student_telegram.to_string(), p.duration_minutes as i64, p.cost));
        }
        msg
    };

    let buttons = vec![
        vec![
            InlineKeyboardButton::callback_key("Add", &add_key),
            InlineKeyboardButton::callback_key("Edit", &edit_key),
            InlineKeyboardButton::callback_key("Remove", &remove_key),
        ],
    ];

    ctx.update_markdown_message(text, Some(InlineKeyboardMarkup::new(buttons))).await
}

// ---------------------------------------------------------------------------
// Add flow
// ---------------------------------------------------------------------------

async fn show_add_selection<Cmd: StudentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    page: i32,
) -> Result<()> {
    let all_students = ctx.state.get_all_students().await;
    let paired: std::collections::HashSet<String> = ctx
        .state
        .get_pairings_for_teacher(teacher)
        .await
        .into_iter()
        .map(|p| p.student_telegram.as_str().to_string())
        .collect();

    let available: Vec<_> = all_students
        .into_iter()
        .filter(|s| !paired.contains(s.telegram_name.as_str()))
        .collect();

    let total = available.len();
    let info = PageInfo::with_size(page, total, PAGE_SIZE);
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(Cmd::student(PairingParams::P0), &user_proxy).await;

    let text = if total == 0 {
        markdown_string!("*Add student:* all students are already assigned\\.")
    } else {
        let total_pages = (total + PAGE_SIZE - 1) / PAGE_SIZE;
        let mut msg = markdown_format!("*Add student \\(page {}/{}\\):*\n\nSelect a student:", info.page + 1, total_pages);
        for s in &available[info.start..info.end] {
            msg.push(&markdown_format!("\n• {} \\({}\\)", s.telegram_name.to_string(), MarkdownString::escape(s.name.clone())));
        }
        msg
    };

    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    let mut row: Vec<InlineKeyboardButton> = Vec::new();
    for s in &available[info.start..info.end] {
        let key = CallbackKey::pack(Cmd::student(PairingParams::PAN(s.telegram_name.clone())), &user_proxy).await;
        row.push(InlineKeyboardButton::callback_key(s.telegram_name.to_string(), &key));
        if row.len() == COLS {
            buttons.push(std::mem::take(&mut row));
        }
    }
    if !row.is_empty() {
        buttons.push(row);
    }

    let mut nav_row: Vec<InlineKeyboardButton> = Vec::new();
    if info.has_prev() {
        let k = CallbackKey::pack(Cmd::student(PairingParams::PA(info.page - 1)), &user_proxy).await;
        nav_row.push(InlineKeyboardButton::callback_key("◀", &k));
    }
    if info.has_next() {
        let k = CallbackKey::pack(Cmd::student(PairingParams::PA(info.page + 1)), &user_proxy).await;
        nav_row.push(InlineKeyboardButton::callback_key("▶", &k));
    }
    if !nav_row.is_empty() {
        buttons.push(nav_row);
    }
    buttons.push(vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)]);

    ctx.update_markdown_message(text, Some(InlineKeyboardMarkup::new(buttons))).await
}

async fn show_add_form<Cmd: StudentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    student: TelegramName,
) -> Result<()> {
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(Cmd::student(PairingParams::PA(0)), &user_proxy).await;

    let add_cmd = format!("/student {}", PairingParams::PANF(student.clone(), 60, 0));

    let text = match ctx.state.get_student(&student).await {
        Some(s) => markdown_format!(
            "*Add student: {}*\n\n\
            • Name: {}\n\
            • Timezone: {}\n\n\
            Fill in duration \\(minutes\\) and cost, then send the command\\.",
            student.to_string(),
            MarkdownString::escape(s.name),
            MarkdownString::escape(s.timezone.to_string())
        ),
        None => markdown_format!(
            "*Add student: {}*\n\nFill in duration \\(minutes\\) and cost, then send the command\\.",
            student.to_string()
        ),
    };

    let _ = teacher;
    let buttons = vec![
        vec![InlineKeyboardButton::switch_inline_query_current_chat("📝 Fill in and add", add_cmd)],
        vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)],
    ];

    ctx.update_markdown_message(text, Some(InlineKeyboardMarkup::new(buttons))).await
}

async fn exec_add<Cmd: StudentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    student: TelegramName,
    duration_minutes: u64,
    cost: i64,
) -> Result<()> {
    ctx.state.sheets.add_pairing(teacher, &student, duration_minutes, cost).await?;
    let _ = ctx.state.refresh().await;

    let text = markdown_format!(
        "✅ Student {} added with {} min / cost {}\\.",
        student.to_string(),
        duration_minutes as i64,
        cost
    );
    ctx.update_markdown_message(text, None).await
}

// ---------------------------------------------------------------------------
// Edit flow
// ---------------------------------------------------------------------------

async fn show_edit_selection<Cmd: StudentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    page: i32,
) -> Result<()> {
    show_paired_list(ctx, teacher, page, PairingAction::Edit).await
}

async fn show_edit_form<Cmd: StudentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    student: TelegramName,
) -> Result<()> {
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(Cmd::student(PairingParams::PE(0)), &user_proxy).await;

    let pairing = ctx.state.get_pairing(&student, teacher).await;
    let (dur, cost) = pairing.as_ref().map(|p| (p.duration_minutes, p.cost)).unwrap_or((60, 0));
    let edit_cmd = format!("/student {}", PairingParams::PENF(student.clone(), dur, cost));

    let text = match pairing {
        Some(p) => markdown_format!(
            "*Edit student: {}*\n\n\
            • Duration: {} min\n\
            • Cost: {}\n\n\
            Edit duration and cost, then send the command\\.",
            student.to_string(),
            p.duration_minutes as i64,
            p.cost
        ),
        None => markdown_format!(
            "*Edit student: {}*\n\nPairing not found in cache \\(may not be synced yet\\)\\.",
            student.to_string()
        ),
    };

    let buttons = vec![
        vec![InlineKeyboardButton::switch_inline_query_current_chat("✏️ Edit!", edit_cmd)],
        vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)],
    ];

    ctx.update_markdown_message(text, Some(InlineKeyboardMarkup::new(buttons))).await
}

async fn exec_edit<Cmd: StudentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    student: TelegramName,
    duration_minutes: u64,
    cost: i64,
) -> Result<()> {
    ctx.state.sheets.update_pairing(teacher, &student, duration_minutes, cost).await?;
    let _ = ctx.state.refresh().await;

    let text = markdown_format!(
        "✅ Pairing with {} updated: {} min / cost {}\\.",
        student.to_string(),
        duration_minutes as i64,
        cost
    );
    ctx.update_markdown_message(text, None).await
}

// ---------------------------------------------------------------------------
// Remove flow
// ---------------------------------------------------------------------------

async fn show_remove_selection<Cmd: StudentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    page: i32,
) -> Result<()> {
    show_paired_list(ctx, teacher, page, PairingAction::Remove).await
}

async fn show_remove_confirm<Cmd: StudentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    student: TelegramName,
) -> Result<()> {
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(Cmd::student(PairingParams::PR(0)), &user_proxy).await;
    let remove_cmd = format!("/student {}", PairingParams::PRNF(student.clone()));

    let text = match ctx.state.get_pairing(&student, teacher).await {
        Some(p) => markdown_format!(
            "*Remove student: {}*\n\n\
            • Duration: {} min\n\
            • Cost: {}\n\n\
            Confirm removal:",
            student.to_string(),
            p.duration_minutes as i64,
            p.cost
        ),
        None => markdown_format!(
            "*Remove student: {}*\n\nConfirm removal:",
            student.to_string()
        ),
    };

    let buttons = vec![
        vec![InlineKeyboardButton::switch_inline_query_current_chat("🗑 Remove!", remove_cmd)],
        vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)],
    ];

    ctx.update_markdown_message(text, Some(InlineKeyboardMarkup::new(buttons))).await
}

async fn exec_remove<Cmd: StudentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    student: TelegramName,
) -> Result<()> {
    ctx.state.sheets.delete_pairing(teacher, &student).await?;
    let _ = ctx.state.refresh().await;

    let text = markdown_format!("✅ Student {} removed\\.", student.to_string());
    ctx.update_markdown_message(text, None).await
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum PairingAction {
    Edit,
    Remove,
}

async fn show_paired_list<Cmd: StudentCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    page: i32,
    action: PairingAction,
) -> Result<()> {
    let pairings = ctx.state.get_pairings_for_teacher(teacher).await;
    let total = pairings.len();
    let info = PageInfo::with_size(page, total, PAGE_SIZE);
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(Cmd::student(PairingParams::P0), &user_proxy).await;

    let action_label = match action { PairingAction::Edit => "edit", PairingAction::Remove => "remove" };

    let text = if total == 0 {
        markdown_string!("*Your students:* none assigned\\.")
    } else {
        let total_pages = (total + PAGE_SIZE - 1) / PAGE_SIZE;
        let mut msg = markdown_format!(
            "*Students \\(page {}/{}\\):*\n\nSelect a student to {}:",
            info.page + 1,
            total_pages,
            action_label
        );
        for p in &pairings[info.start..info.end] {
            msg.push(&markdown_format!("\n• {} — {} min, cost {}", p.student_telegram.to_string(), p.duration_minutes as i64, p.cost));
        }
        msg
    };

    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    let mut row: Vec<InlineKeyboardButton> = Vec::new();
    for p in &pairings[info.start..info.end] {
        let param = pairing_action_param(action, &p);
        let key = CallbackKey::pack(Cmd::student(param), &user_proxy).await;
        row.push(InlineKeyboardButton::callback_key(p.student_telegram.to_string(), &key));
        if row.len() == COLS {
            buttons.push(std::mem::take(&mut row));
        }
    }
    if !row.is_empty() {
        buttons.push(row);
    }

    let mut nav_row: Vec<InlineKeyboardButton> = Vec::new();
    if info.has_prev() {
        let k = CallbackKey::pack(Cmd::student(page_param(action, info.page - 1)), &user_proxy).await;
        nav_row.push(InlineKeyboardButton::callback_key("◀", &k));
    }
    if info.has_next() {
        let k = CallbackKey::pack(Cmd::student(page_param(action, info.page + 1)), &user_proxy).await;
        nav_row.push(InlineKeyboardButton::callback_key("▶", &k));
    }
    if !nav_row.is_empty() {
        buttons.push(nav_row);
    }
    buttons.push(vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)]);

    ctx.update_markdown_message(text, Some(InlineKeyboardMarkup::new(buttons))).await
}

fn pairing_action_param(action: PairingAction, p: &TeacherStudentPairing) -> PairingParams {
    match action {
        PairingAction::Edit => PairingParams::PEN(p.student_telegram.clone()),
        PairingAction::Remove => PairingParams::PRN(p.student_telegram.clone()),
    }
}

fn page_param(action: PairingAction, page: i32) -> PairingParams {
    match action {
        PairingAction::Edit => PairingParams::PE(page),
        PairingAction::Remove => PairingParams::PR(page),
    }
}
