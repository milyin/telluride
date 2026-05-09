use crate::api::common::PageInfo;
use crate::api::context::BotCtx;
use crate::api::traits::{UserCommand, UserParams};
use crate::types::{Currency, Timezone};
use crate::models::{Student, Teacher, TelegramName};
use anyhow::Result;
use telluride::command::{CallbackBitcode, CallbackKey, InlineKeyboardButtonPackedExt};
use telluride::data_store::UserProxy;
use telluride::markdown::MarkdownString;
use telluride::{markdown_format, markdown_string};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub async fn user<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    params: &str,
) -> Result<()> {
    let params: UserParams = params.parse()?;
    match params {
        UserParams::U0 => show_role_selection(ctx).await,
        UserParams::US(page) => show_student_submenu(ctx, page).await,
        UserParams::UT(page) => show_teacher_submenu(ctx, page).await,
        UserParams::USA => show_student_add(ctx).await,
        UserParams::UTA => show_teacher_add(ctx).await,
        UserParams::USAF(name, tz, currency, display_name) => {
            exec_student_add(ctx, name, tz, currency, display_name).await
        }
        UserParams::UTAF(name, tz, display_name) => {
            exec_teacher_add(ctx, name, tz, display_name).await
        }
        UserParams::USD(page) => show_student_list(ctx, page, ListAction::Delete).await,
        UserParams::UTD(page) => show_teacher_list(ctx, page, ListAction::Delete).await,
        UserParams::USDN(name) => show_student_delete_confirm(ctx, name).await,
        UserParams::UTDN(name) => show_teacher_delete_confirm(ctx, name).await,
        UserParams::USDNF(name) => exec_student_delete(ctx, name).await,
        UserParams::UTDNF(name) => exec_teacher_delete(ctx, name).await,
        UserParams::USE(page) => show_student_list(ctx, page, ListAction::Edit).await,
        UserParams::UTE(page) => show_teacher_list(ctx, page, ListAction::Edit).await,
        UserParams::USEN(name) => show_student_edit(ctx, name).await,
        UserParams::UTEN(name) => show_teacher_edit(ctx, name).await,
        UserParams::USENF(name, tz, currency, display_name) => {
            exec_student_edit(ctx, name, tz, currency, display_name).await
        }
        UserParams::UTENF(name, tz, display_name) => {
            exec_teacher_edit(ctx, name, tz, display_name).await
        }
    }
}

// ---------------------------------------------------------------------------
// Role selection
// ---------------------------------------------------------------------------

async fn show_role_selection<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
) -> Result<()> {
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);

    let student_key = CallbackKey::pack(Cmd::user(UserParams::US(0)), &user_proxy).await;
    let teacher_key = CallbackKey::pack(Cmd::user(UserParams::UT(0)), &user_proxy).await;

    let buttons = vec![
        vec![InlineKeyboardButton::callback_key("Student", &student_key)],
        vec![InlineKeyboardButton::callback_key("Teacher", &teacher_key)],
    ];

    ctx.update_markdown_message(
        markdown_string!("Select role to manage:"),
        Some(InlineKeyboardMarkup::new(buttons)),
    )
    .await
}

// ---------------------------------------------------------------------------
// Submenus — Add / Edit / Delete in one row
// ---------------------------------------------------------------------------

async fn show_student_submenu<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    page: i32,
) -> Result<()> {
    let students = ctx.state.get_all_students().await;
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);

    let add_key = CallbackKey::pack(Cmd::user(UserParams::USA), &user_proxy).await;
    let edit_key = CallbackKey::pack(Cmd::user(UserParams::USE(0)), &user_proxy).await;
    let del_key = CallbackKey::pack(Cmd::user(UserParams::USD(0)), &user_proxy).await;
    let back_key = CallbackKey::pack(Cmd::user(UserParams::U0), &user_proxy).await;

    let total = students.len();
    let info = PageInfo::new(page, total);
    let total_pages = (total + PageInfo::PAGE_SIZE - 1) / PageInfo::PAGE_SIZE;

    let mut text = if total == 0 {
        markdown_string!("*Students:* none registered\\.")
    } else {
        markdown_format!("*Students \\(page {}/{}\\):*", info.page + 1, total_pages)
    };
    for student in &students[info.start..info.end] {
        text.push(&markdown_format!("\n• {}", student_one_line(student)));
    }
    text.push(&markdown_string!("\n\nManage students:"));

    let mut buttons = vec![vec![
        InlineKeyboardButton::callback_key("Add", &add_key),
        InlineKeyboardButton::callback_key("Edit", &edit_key),
        InlineKeyboardButton::callback_key("Delete", &del_key),
    ]];
    let mut nav_row: Vec<InlineKeyboardButton> = Vec::new();
    if info.has_prev() {
        let k = CallbackKey::pack(Cmd::user(UserParams::US(info.page - 1)), &user_proxy).await;
        nav_row.push(InlineKeyboardButton::callback_key("◀", &k));
    }
    if info.has_next() {
        let k = CallbackKey::pack(Cmd::user(UserParams::US(info.page + 1)), &user_proxy).await;
        nav_row.push(InlineKeyboardButton::callback_key("▶", &k));
    }
    if !nav_row.is_empty() {
        buttons.push(nav_row);
    }
    buttons.push(vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)]);

    ctx.update_markdown_message(text, Some(InlineKeyboardMarkup::new(buttons))).await
}

async fn show_teacher_submenu<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    page: i32,
) -> Result<()> {
    let teachers = ctx.state.get_all_teachers().await;
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);

    let add_key = CallbackKey::pack(Cmd::user(UserParams::UTA), &user_proxy).await;
    let edit_key = CallbackKey::pack(Cmd::user(UserParams::UTE(0)), &user_proxy).await;
    let del_key = CallbackKey::pack(Cmd::user(UserParams::UTD(0)), &user_proxy).await;
    let back_key = CallbackKey::pack(Cmd::user(UserParams::U0), &user_proxy).await;

    let total = teachers.len();
    let info = PageInfo::new(page, total);
    let total_pages = (total + PageInfo::PAGE_SIZE - 1) / PageInfo::PAGE_SIZE;

    let mut text = if total == 0 {
        markdown_string!("*Teachers:* none registered\\.")
    } else {
        markdown_format!("*Teachers \\(page {}/{}\\):*", info.page + 1, total_pages)
    };
    for teacher in &teachers[info.start..info.end] {
        text.push(&markdown_format!("\n• {}", teacher_one_line(teacher)));
    }
    text.push(&markdown_string!("\n\nManage teachers:"));

    let mut buttons = vec![vec![
        InlineKeyboardButton::callback_key("Add", &add_key),
        InlineKeyboardButton::callback_key("Edit", &edit_key),
        InlineKeyboardButton::callback_key("Delete", &del_key),
    ]];
    let mut nav_row: Vec<InlineKeyboardButton> = Vec::new();
    if info.has_prev() {
        let k = CallbackKey::pack(Cmd::user(UserParams::UT(info.page - 1)), &user_proxy).await;
        nav_row.push(InlineKeyboardButton::callback_key("◀", &k));
    }
    if info.has_next() {
        let k = CallbackKey::pack(Cmd::user(UserParams::UT(info.page + 1)), &user_proxy).await;
        nav_row.push(InlineKeyboardButton::callback_key("▶", &k));
    }
    if !nav_row.is_empty() {
        buttons.push(nav_row);
    }
    buttons.push(vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)]);

    ctx.update_markdown_message(text, Some(InlineKeyboardMarkup::new(buttons))).await
}

// ---------------------------------------------------------------------------
// Add
// ---------------------------------------------------------------------------

async fn show_student_add<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
) -> Result<()> {
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(Cmd::user(UserParams::US(0)), &user_proxy).await;

    let text = markdown_string!(
        "*Add student*\n\n\
        Click the button below to fill in the command, then send it\\.\n\n\
        Format: `/user student add\\! @username TZ USD Name Surname`\n\n\
        • `@username` — Telegram username\n\
        • `TZ` — timezone: IANA name \\(`Europe/Berlin`\\) or UTC offset \\(`\\+3`, `\\-5`, `0`\\)\n\
        • `USD` — ISO 4217 currency code \\(`USD`, `EUR`, `RUB`, …\\); invalid codes are rejected\n\
        • `Name Surname` — display name \\(spaces OK, must be last\\)"
    );

    let buttons = vec![
        vec![InlineKeyboardButton::switch_inline_query_current_chat(
            "📝 Fill in and add",
            "/user student add! @username TZ USD Name Surname",
        )],
        vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)],
    ];

    ctx.update_markdown_message(text, Some(InlineKeyboardMarkup::new(buttons)))
        .await
}

async fn show_teacher_add<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
) -> Result<()> {
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(Cmd::user(UserParams::UT(0)), &user_proxy).await;

    let text = markdown_string!(
        "*Add teacher*\n\n\
        Click the button below to fill in the command, then send it\\.\n\n\
        Format: `/user teacher add\\! @username TZ Name Surname`\n\n\
        • `@username` — Telegram username\n\
        • `TZ` — timezone: IANA name \\(`Europe/Berlin`\\) or UTC offset \\(`\\+3`, `\\-5`, `0`\\)\n\
        • `Name Surname` — display name \\(spaces OK, must be last\\)"
    );

    let buttons = vec![
        vec![InlineKeyboardButton::switch_inline_query_current_chat(
            "📝 Fill in and add",
            "/user teacher add! @username TZ Name Surname",
        )],
        vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)],
    ];

    ctx.update_markdown_message(text, Some(InlineKeyboardMarkup::new(buttons)))
        .await
}

async fn exec_student_add<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    name: TelegramName,
    tz: Timezone,
    currency: Currency,
    display_name: String,
) -> Result<()> {
    ctx.state
        .sheets
        .add_student(&name, &tz.to_string(), currency.0.iso_alpha_code, &display_name)
        .await?;
    let _ = ctx.state.refresh().await;

    let text = markdown_format!(
        "✅ Student {} \\({}\\) added successfully\\.",
        name.to_string(),
        MarkdownString::escape(display_name)
    );
    ctx.update_markdown_message(text, None).await
}

async fn exec_teacher_add<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    name: TelegramName,
    tz: Timezone,
    display_name: String,
) -> Result<()> {
    ctx.state
        .sheets
        .add_teacher(&name, &tz.to_string(), &display_name)
        .await?;
    let _ = ctx.state.refresh().await;

    let text = markdown_format!(
        "✅ Teacher {} \\({}\\) added successfully\\.",
        name.to_string(),
        MarkdownString::escape(display_name)
    );
    ctx.update_markdown_message(text, None).await
}

// ---------------------------------------------------------------------------
// Shared paged list (Delete or Edit)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum ListAction {
    Delete,
    Edit,
}

fn student_one_line(s: &Student) -> String {
    let currency_str = s.currency.map(|c| c.iso_alpha_code).unwrap_or("-");
    format!("{} · {} · {} · {}", s.telegram_name, s.name, Timezone(s.timezone), currency_str)
}

fn teacher_one_line(t: &Teacher) -> String {
    format!("{} · {} · {}", t.telegram_name, t.name, Timezone(t.timezone))
}

async fn show_student_list<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    page: i32,
    action: ListAction,
) -> Result<()> {
    let students = ctx.state.get_all_students().await;
    let back_to_submenu = CallbackKey::pack(Cmd::user(UserParams::US(0)), &UserProxy::new(ctx.callback_storage.clone(), ctx.user_id)).await;

    if students.is_empty() {
        return ctx
            .update_markdown_message(
                markdown_string!("No students registered\\."),
                Some(InlineKeyboardMarkup::new(vec![vec![
                    InlineKeyboardButton::callback_key("↩ Back", &back_to_submenu),
                ]])),
            )
            .await;
    }

    let total = students.len();
    let info = PageInfo::new(page, total);
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    for student in &students[info.start..info.end] {
        let label = student_one_line(student);
        let param = match action {
            ListAction::Delete => UserParams::USDN(student.telegram_name.clone()),
            ListAction::Edit => UserParams::USEN(student.telegram_name.clone()),
        };
        let key = CallbackKey::pack(Cmd::user(param), &user_proxy).await;
        buttons.push(vec![InlineKeyboardButton::callback_key(label, &key)]);
    }

    let mut nav_row: Vec<InlineKeyboardButton> = Vec::new();
    if info.has_prev() {
        let p = match action {
            ListAction::Delete => UserParams::USD(info.page - 1),
            ListAction::Edit => UserParams::USE(info.page - 1),
        };
        let k = CallbackKey::pack(Cmd::user(p), &user_proxy).await;
        nav_row.push(InlineKeyboardButton::callback_key("◀", &k));
    }
    if info.has_next() {
        let p = match action {
            ListAction::Delete => UserParams::USD(info.page + 1),
            ListAction::Edit => UserParams::USE(info.page + 1),
        };
        let k = CallbackKey::pack(Cmd::user(p), &user_proxy).await;
        nav_row.push(InlineKeyboardButton::callback_key("▶", &k));
    }
    if !nav_row.is_empty() {
        buttons.push(nav_row);
    }

    let back_key = CallbackKey::pack(Cmd::user(UserParams::US(0)), &user_proxy).await;
    buttons.push(vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)]);

    let title = match action {
        ListAction::Delete => markdown_string!("Select student to delete:"),
        ListAction::Edit => markdown_string!("Select student to edit:"),
    };
    ctx.update_markdown_message(title, Some(InlineKeyboardMarkup::new(buttons)))
        .await
}

async fn show_teacher_list<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    page: i32,
    action: ListAction,
) -> Result<()> {
    let teachers = ctx.state.get_all_teachers().await;
    let back_to_submenu = CallbackKey::pack(Cmd::user(UserParams::UT(0)), &UserProxy::new(ctx.callback_storage.clone(), ctx.user_id)).await;

    if teachers.is_empty() {
        return ctx
            .update_markdown_message(
                markdown_string!("No teachers registered\\."),
                Some(InlineKeyboardMarkup::new(vec![vec![
                    InlineKeyboardButton::callback_key("↩ Back", &back_to_submenu),
                ]])),
            )
            .await;
    }

    let total = teachers.len();
    let info = PageInfo::new(page, total);
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    for teacher in &teachers[info.start..info.end] {
        let label = teacher_one_line(teacher);
        let param = match action {
            ListAction::Delete => UserParams::UTDN(teacher.telegram_name.clone()),
            ListAction::Edit => UserParams::UTEN(teacher.telegram_name.clone()),
        };
        let key = CallbackKey::pack(Cmd::user(param), &user_proxy).await;
        buttons.push(vec![InlineKeyboardButton::callback_key(label, &key)]);
    }

    let mut nav_row: Vec<InlineKeyboardButton> = Vec::new();
    if info.has_prev() {
        let p = match action {
            ListAction::Delete => UserParams::UTD(info.page - 1),
            ListAction::Edit => UserParams::UTE(info.page - 1),
        };
        let k = CallbackKey::pack(Cmd::user(p), &user_proxy).await;
        nav_row.push(InlineKeyboardButton::callback_key("◀", &k));
    }
    if info.has_next() {
        let p = match action {
            ListAction::Delete => UserParams::UTD(info.page + 1),
            ListAction::Edit => UserParams::UTE(info.page + 1),
        };
        let k = CallbackKey::pack(Cmd::user(p), &user_proxy).await;
        nav_row.push(InlineKeyboardButton::callback_key("▶", &k));
    }
    if !nav_row.is_empty() {
        buttons.push(nav_row);
    }

    let back_key = CallbackKey::pack(Cmd::user(UserParams::UT(0)), &user_proxy).await;
    buttons.push(vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)]);

    let title = match action {
        ListAction::Delete => markdown_string!("Select teacher to delete:"),
        ListAction::Edit => markdown_string!("Select teacher to edit:"),
    };
    ctx.update_markdown_message(title, Some(InlineKeyboardMarkup::new(buttons)))
        .await
}

// ---------------------------------------------------------------------------
// Delete — confirm
// ---------------------------------------------------------------------------

async fn show_student_delete_confirm<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    name: TelegramName,
) -> Result<()> {
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(Cmd::user(UserParams::USD(0)), &user_proxy).await;
    let delete_cmd = format!("/user {}", UserParams::USDNF(name.clone()));

    let text = match ctx.state.get_student(&name).await {
        Some(student) => {
            let currency_str = student.currency.map(|c| c.iso_alpha_code).unwrap_or("-");
            let zoom_str = student.zoom_url.as_deref().unwrap_or("—");
            let board_str = student.board_url.as_deref().unwrap_or("—");
            markdown_format!(
                "*Student: {}*\n\n\
                • Name: {}\n\
                • Timezone: {}\n\
                • Currency: {}\n\
                • Zoom: {}\n\
                • Board: {}",
                name.to_string(),
                MarkdownString::escape(student.name),
                MarkdownString::escape(Timezone(student.timezone).to_string()),
                MarkdownString::escape(currency_str.to_string()),
                MarkdownString::escape(zoom_str.to_string()),
                MarkdownString::escape(board_str.to_string())
            )
        }
        None => markdown_format!(
            "Student {} not found in cache \\(may not be synced yet\\)\\.",
            name.to_string()
        ),
    };

    let buttons = vec![
        vec![InlineKeyboardButton::switch_inline_query_current_chat("🗑 Delete!", delete_cmd)],
        vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)],
    ];

    ctx.update_markdown_message(text, Some(InlineKeyboardMarkup::new(buttons)))
        .await
}

async fn show_teacher_delete_confirm<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    name: TelegramName,
) -> Result<()> {
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(Cmd::user(UserParams::UTD(0)), &user_proxy).await;
    let delete_cmd = format!("/user {}", UserParams::UTDNF(name.clone()));

    let text = match ctx.state.get_teacher(&name).await {
        Some(teacher) => {
            let admin_str = if teacher.admin { "yes" } else { "no" };
            markdown_format!(
                "*Teacher: {}*\n\n\
                • Name: {}\n\
                • Timezone: {}\n\
                • Admin: {}",
                name.to_string(),
                MarkdownString::escape(teacher.name),
                MarkdownString::escape(Timezone(teacher.timezone).to_string()),
                admin_str
            )
        }
        None => markdown_format!(
            "Teacher {} not found in cache \\(may not be synced yet\\)\\.",
            name.to_string()
        ),
    };

    let buttons = vec![
        vec![InlineKeyboardButton::switch_inline_query_current_chat("🗑 Delete!", delete_cmd)],
        vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)],
    ];

    ctx.update_markdown_message(text, Some(InlineKeyboardMarkup::new(buttons)))
        .await
}

// ---------------------------------------------------------------------------
// Delete — execute
// ---------------------------------------------------------------------------

async fn exec_student_delete<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    name: TelegramName,
) -> Result<()> {
    ctx.state.sheets.delete_student(&name).await?;
    let _ = ctx.state.refresh().await;
    let text = markdown_format!("✅ Student {} deleted successfully\\.", name.to_string());
    ctx.update_markdown_message(text, None).await
}

async fn exec_teacher_delete<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    name: TelegramName,
) -> Result<()> {
    ctx.state.sheets.delete_teacher(&name).await?;
    let _ = ctx.state.refresh().await;
    let text = markdown_format!("✅ Teacher {} deleted successfully\\.", name.to_string());
    ctx.update_markdown_message(text, None).await
}

// ---------------------------------------------------------------------------
// Edit — show pre-filled command
// ---------------------------------------------------------------------------

async fn show_student_edit<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    name: TelegramName,
) -> Result<()> {
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(Cmd::user(UserParams::USE(0)), &user_proxy).await;

    let (text, edit_cmd) = match ctx.state.get_student(&name).await {
        Some(student) => {
            let currency_opt: Option<Currency> = student.currency.map(Currency);
            let currency_display = currency_opt.as_ref().map(|c| c.0.iso_alpha_code).unwrap_or("-");
            let zoom_str = student.zoom_url.as_deref().unwrap_or("—");
            let board_str = student.board_url.as_deref().unwrap_or("—");
            let tz = Timezone(student.timezone);

            let edit_params = UserParams::USENF(
                name.clone(),
                tz,
                currency_opt,
                student.name.clone(),
            );
            let info = markdown_format!(
                "*Student: {}*\n\n\
                • Name: {}\n\
                • Timezone: {}\n\
                • Currency: {}\n\
                • Zoom: {}\n\
                • Board: {}\n\n\
                Click below to edit timezone, currency, and name\\.",
                name.to_string(),
                MarkdownString::escape(student.name),
                MarkdownString::escape(tz.to_string()),
                MarkdownString::escape(currency_display.to_string()),
                MarkdownString::escape(zoom_str.to_string()),
                MarkdownString::escape(board_str.to_string())
            );
            (info, format!("/user {edit_params}"))
        }
        None => {
            let edit_params = UserParams::USENF(
                name.clone(),
                Timezone(chrono_tz::UTC),
                None,
                "Name Surname".to_string(),
            );
            let info = markdown_format!(
                "Student {} not found in cache \\(may not be synced yet\\)\\.\n\nFill in values:",
                name.to_string()
            );
            (info, format!("/user {edit_params}"))
        }
    };

    let buttons = vec![
        vec![InlineKeyboardButton::switch_inline_query_current_chat("✏️ Edit!", edit_cmd)],
        vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)],
    ];

    ctx.update_markdown_message(text, Some(InlineKeyboardMarkup::new(buttons)))
        .await
}

async fn show_teacher_edit<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    name: TelegramName,
) -> Result<()> {
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(Cmd::user(UserParams::UTE(0)), &user_proxy).await;

    let (text, edit_cmd) = match ctx.state.get_teacher(&name).await {
        Some(teacher) => {
            let admin_str = if teacher.admin { "yes" } else { "no" };
            let tz = Timezone(teacher.timezone);

            let edit_params = UserParams::UTENF(
                name.clone(),
                tz,
                teacher.name.clone(),
            );
            let info = markdown_format!(
                "*Teacher: {}*\n\n\
                • Name: {}\n\
                • Timezone: {}\n\
                • Admin: {}\n\n\
                Click below to edit timezone and name\\.",
                name.to_string(),
                MarkdownString::escape(teacher.name),
                MarkdownString::escape(tz.to_string()),
                admin_str
            );
            (info, format!("/user {edit_params}"))
        }
        None => {
            let edit_params = UserParams::UTENF(
                name.clone(),
                Timezone(chrono_tz::UTC),
                "Name Surname".to_string(),
            );
            let info = markdown_format!(
                "Teacher {} not found in cache \\(may not be synced yet\\)\\.\n\nFill in values:",
                name.to_string()
            );
            (info, format!("/user {edit_params}"))
        }
    };

    let buttons = vec![
        vec![InlineKeyboardButton::switch_inline_query_current_chat("✏️ Edit!", edit_cmd)],
        vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)],
    ];

    ctx.update_markdown_message(text, Some(InlineKeyboardMarkup::new(buttons)))
        .await
}

// ---------------------------------------------------------------------------
// Edit — execute
// ---------------------------------------------------------------------------

async fn exec_student_edit<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    name: TelegramName,
    tz: Timezone,
    currency: Option<Currency>,
    display_name: String,
) -> Result<()> {
    let currency_str = currency.as_ref().map(|c| c.0.iso_alpha_code).unwrap_or("");
    ctx.state
        .sheets
        .update_student(&name, &tz.to_string(), currency_str, &display_name)
        .await?;
    let _ = ctx.state.refresh().await;
    let text = markdown_format!("✅ Student {} updated successfully\\.", name.to_string());
    ctx.update_markdown_message(text, None).await
}

async fn exec_teacher_edit<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    name: TelegramName,
    tz: Timezone,
    display_name: String,
) -> Result<()> {
    ctx.state
        .sheets
        .update_teacher(&name, &tz.to_string(), &display_name)
        .await?;
    let _ = ctx.state.refresh().await;
    let text = markdown_format!("✅ Teacher {} updated successfully\\.", name.to_string());
    ctx.update_markdown_message(text, None).await
}
