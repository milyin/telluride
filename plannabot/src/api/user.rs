use crate::api::context::BotCtx;
use crate::api::traits::{UserCommand, UserParams};
use crate::models::TelegramName;
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
        UserParams::US => show_student_submenu(ctx).await,
        UserParams::UT => show_teacher_submenu(ctx).await,
        UserParams::USA => show_student_add(ctx).await,
        UserParams::UTA => show_teacher_add(ctx).await,
        UserParams::USAF(name, tz, currency, display_name) => {
            exec_student_add(ctx, name, tz, currency, display_name).await
        }
        UserParams::UTAF(name, tz, display_name) => {
            exec_teacher_add(ctx, name, tz, display_name).await
        }
        UserParams::USD => show_student_delete_list(ctx).await,
        UserParams::UTD => show_teacher_delete_list(ctx).await,
        UserParams::USDN(name) => show_student_delete_confirm(ctx, name).await,
        UserParams::UTDN(name) => show_teacher_delete_confirm(ctx, name).await,
        UserParams::USDNF(name) => exec_student_delete(ctx, name).await,
        UserParams::UTDNF(name) => exec_teacher_delete(ctx, name).await,
        UserParams::USE => show_student_edit_list(ctx).await,
        UserParams::UTE => show_teacher_edit_list(ctx).await,
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

    let student_key =
        CallbackKey::pack(Cmd::user(UserParams::US), &user_proxy).await;
    let teacher_key =
        CallbackKey::pack(Cmd::user(UserParams::UT), &user_proxy).await;

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
// Submenus
// ---------------------------------------------------------------------------

async fn show_student_submenu<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
) -> Result<()> {
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);

    let add_key = CallbackKey::pack(Cmd::user(UserParams::USA), &user_proxy).await;
    let edit_key = CallbackKey::pack(Cmd::user(UserParams::USE), &user_proxy).await;
    let del_key = CallbackKey::pack(Cmd::user(UserParams::USD), &user_proxy).await;
    let back_key = CallbackKey::pack(Cmd::user(UserParams::U0), &user_proxy).await;

    let buttons = vec![
        vec![InlineKeyboardButton::callback_key("Add", &add_key)],
        vec![InlineKeyboardButton::callback_key("Edit", &edit_key)],
        vec![InlineKeyboardButton::callback_key("Delete", &del_key)],
        vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)],
    ];

    ctx.update_markdown_message(
        markdown_string!("Manage students:"),
        Some(InlineKeyboardMarkup::new(buttons)),
    )
    .await
}

async fn show_teacher_submenu<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
) -> Result<()> {
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);

    let add_key = CallbackKey::pack(Cmd::user(UserParams::UTA), &user_proxy).await;
    let edit_key = CallbackKey::pack(Cmd::user(UserParams::UTE), &user_proxy).await;
    let del_key = CallbackKey::pack(Cmd::user(UserParams::UTD), &user_proxy).await;
    let back_key = CallbackKey::pack(Cmd::user(UserParams::U0), &user_proxy).await;

    let buttons = vec![
        vec![InlineKeyboardButton::callback_key("Add", &add_key)],
        vec![InlineKeyboardButton::callback_key("Edit", &edit_key)],
        vec![InlineKeyboardButton::callback_key("Delete", &del_key)],
        vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)],
    ];

    ctx.update_markdown_message(
        markdown_string!("Manage teachers:"),
        Some(InlineKeyboardMarkup::new(buttons)),
    )
    .await
}

// ---------------------------------------------------------------------------
// Add
// ---------------------------------------------------------------------------

async fn show_student_add<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
) -> Result<()> {
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(Cmd::user(UserParams::US), &user_proxy).await;

    let text = markdown_string!(
        "*Add student*\n\n\
        Click the button below to fill in the command, then send it\\.\n\n\
        Format: `/user student add\\! @username TZ USD Name Surname`\n\n\
        • `@username` — Telegram username\n\
        • `TZ` — timezone, e\\.g\\. `Europe/Berlin` or `Asia/Tokyo`\n\
        • `USD` — ISO currency code, e\\.g\\. `USD`, `EUR`, `RUB`\n\
        • `Name Surname` — full display name \\(spaces allowed, must be last\\)"
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
    let back_key = CallbackKey::pack(Cmd::user(UserParams::UT), &user_proxy).await;

    let text = markdown_string!(
        "*Add teacher*\n\n\
        Click the button below to fill in the command, then send it\\.\n\n\
        Format: `/user teacher add\\! @username TZ Name Surname`\n\n\
        • `@username` — Telegram username\n\
        • `TZ` — timezone, e\\.g\\. `Europe/Berlin` or `Asia/Tokyo`\n\
        • `Name Surname` — full display name \\(spaces allowed, must be last\\)"
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
    tz: String,
    currency: String,
    display_name: String,
) -> Result<()> {
    ctx.state
        .sheets
        .add_student(&name, &tz, &currency, &display_name)
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
    tz: String,
    display_name: String,
) -> Result<()> {
    ctx.state
        .sheets
        .add_teacher(&name, &tz, &display_name)
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
// Delete — list
// ---------------------------------------------------------------------------

async fn show_student_delete_list<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
) -> Result<()> {
    let names = ctx.state.get_student_names().await;

    if names.is_empty() {
        let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
        let back_key = CallbackKey::pack(Cmd::user(UserParams::US), &user_proxy).await;
        return ctx
            .update_markdown_message(
                markdown_string!("No students registered\\."),
                Some(InlineKeyboardMarkup::new(vec![vec![
                    InlineKeyboardButton::callback_key("↩ Back", &back_key),
                ]])),
            )
            .await;
    }

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    for name in names {
        let label = name.to_string();
        let key = CallbackKey::pack(
            Cmd::user(UserParams::USDN(name)),
            &user_proxy,
        )
        .await;
        buttons.push(vec![InlineKeyboardButton::callback_key(label, &key)]);
    }

    let back_key = CallbackKey::pack(Cmd::user(UserParams::US), &user_proxy).await;
    buttons.push(vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)]);

    ctx.update_markdown_message(
        markdown_string!("Select the student to delete:"),
        Some(InlineKeyboardMarkup::new(buttons)),
    )
    .await
}

async fn show_teacher_delete_list<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
) -> Result<()> {
    let names = ctx.state.get_teacher_names().await;

    if names.is_empty() {
        let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
        let back_key = CallbackKey::pack(Cmd::user(UserParams::UT), &user_proxy).await;
        return ctx
            .update_markdown_message(
                markdown_string!("No teachers registered\\."),
                Some(InlineKeyboardMarkup::new(vec![vec![
                    InlineKeyboardButton::callback_key("↩ Back", &back_key),
                ]])),
            )
            .await;
    }

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    for name in names {
        let label = name.to_string();
        let key = CallbackKey::pack(
            Cmd::user(UserParams::UTDN(name)),
            &user_proxy,
        )
        .await;
        buttons.push(vec![InlineKeyboardButton::callback_key(label, &key)]);
    }

    let back_key = CallbackKey::pack(Cmd::user(UserParams::UT), &user_proxy).await;
    buttons.push(vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)]);

    ctx.update_markdown_message(
        markdown_string!("Select the teacher to delete:"),
        Some(InlineKeyboardMarkup::new(buttons)),
    )
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
    let back_key = CallbackKey::pack(Cmd::user(UserParams::USD), &user_proxy).await;

    let delete_cmd = format!("/user {}", UserParams::USDNF(name.clone()));

    let text = match ctx.state.get_student(&name).await {
        Some(student) => {
            let currency_str = student
                .currency
                .map(|c| c.iso_alpha_code)
                .unwrap_or("-");
            let zoom_str = student.zoom_url.as_deref().unwrap_or("-");
            let board_str = student.board_url.as_deref().unwrap_or("-");
            markdown_format!(
                "*Student: {}*\n\n\
                • Name: {}\n\
                • Timezone: {}\n\
                • Currency: {}\n\
                • Zoom: {}\n\
                • Board: {}",
                name.to_string(),
                MarkdownString::escape(student.name),
                MarkdownString::escape(student.timezone.to_string()),
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
        vec![InlineKeyboardButton::switch_inline_query_current_chat(
            "🗑 Delete!",
            delete_cmd,
        )],
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
    let back_key = CallbackKey::pack(Cmd::user(UserParams::UTD), &user_proxy).await;

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
                MarkdownString::escape(teacher.timezone.to_string()),
                admin_str
            )
        }
        None => markdown_format!(
            "Teacher {} not found in cache \\(may not be synced yet\\)\\.",
            name.to_string()
        ),
    };

    let buttons = vec![
        vec![InlineKeyboardButton::switch_inline_query_current_chat(
            "🗑 Delete!",
            delete_cmd,
        )],
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
// Edit — list
// ---------------------------------------------------------------------------

async fn show_student_edit_list<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
) -> Result<()> {
    let names = ctx.state.get_student_names().await;

    if names.is_empty() {
        let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
        let back_key = CallbackKey::pack(Cmd::user(UserParams::US), &user_proxy).await;
        return ctx
            .update_markdown_message(
                markdown_string!("No students registered\\."),
                Some(InlineKeyboardMarkup::new(vec![vec![
                    InlineKeyboardButton::callback_key("↩ Back", &back_key),
                ]])),
            )
            .await;
    }

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    for name in names {
        let label = name.to_string();
        let key = CallbackKey::pack(
            Cmd::user(UserParams::USEN(name)),
            &user_proxy,
        )
        .await;
        buttons.push(vec![InlineKeyboardButton::callback_key(label, &key)]);
    }

    let back_key = CallbackKey::pack(Cmd::user(UserParams::US), &user_proxy).await;
    buttons.push(vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)]);

    ctx.update_markdown_message(
        markdown_string!("Select the student to edit:"),
        Some(InlineKeyboardMarkup::new(buttons)),
    )
    .await
}

async fn show_teacher_edit_list<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
) -> Result<()> {
    let names = ctx.state.get_teacher_names().await;

    if names.is_empty() {
        let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
        let back_key = CallbackKey::pack(Cmd::user(UserParams::UT), &user_proxy).await;
        return ctx
            .update_markdown_message(
                markdown_string!("No teachers registered\\."),
                Some(InlineKeyboardMarkup::new(vec![vec![
                    InlineKeyboardButton::callback_key("↩ Back", &back_key),
                ]])),
            )
            .await;
    }

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    for name in names {
        let label = name.to_string();
        let key = CallbackKey::pack(
            Cmd::user(UserParams::UTEN(name)),
            &user_proxy,
        )
        .await;
        buttons.push(vec![InlineKeyboardButton::callback_key(label, &key)]);
    }

    let back_key = CallbackKey::pack(Cmd::user(UserParams::UT), &user_proxy).await;
    buttons.push(vec![InlineKeyboardButton::callback_key("↩ Back", &back_key)]);

    ctx.update_markdown_message(
        markdown_string!("Select the teacher to edit:"),
        Some(InlineKeyboardMarkup::new(buttons)),
    )
    .await
}

// ---------------------------------------------------------------------------
// Edit — show pre-filled command
// ---------------------------------------------------------------------------

async fn show_student_edit<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    name: TelegramName,
) -> Result<()> {
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let back_key = CallbackKey::pack(Cmd::user(UserParams::USE), &user_proxy).await;

    let (text, edit_cmd) = match ctx.state.get_student(&name).await {
        Some(student) => {
            let currency_str = student
                .currency
                .map(|c| c.iso_alpha_code)
                .unwrap_or("")
                .to_string();
            let zoom_str = student.zoom_url.as_deref().unwrap_or("-");
            let board_str = student.board_url.as_deref().unwrap_or("-");

            let edit_params = UserParams::USENF(
                name.clone(),
                student.timezone.to_string(),
                currency_str.clone(),
                student.name.clone(),
            );
            let cmd = format!("/user {edit_params}");

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
                MarkdownString::escape(student.timezone.to_string()),
                MarkdownString::escape(currency_str),
                MarkdownString::escape(zoom_str.to_string()),
                MarkdownString::escape(board_str.to_string())
            );
            (info, cmd)
        }
        None => {
            let edit_params = UserParams::USENF(
                name.clone(),
                "TZ".to_string(),
                "USD".to_string(),
                "Name Surname".to_string(),
            );
            let cmd = format!("/user {edit_params}");
            let info = markdown_format!(
                "Student {} not found in cache \\(may not be synced yet\\)\\.\n\nEdit command \\(fill in values\\):",
                name.to_string()
            );
            (info, cmd)
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
    let back_key = CallbackKey::pack(Cmd::user(UserParams::UTE), &user_proxy).await;

    let (text, edit_cmd) = match ctx.state.get_teacher(&name).await {
        Some(teacher) => {
            let admin_str = if teacher.admin { "yes" } else { "no" };

            let edit_params = UserParams::UTENF(
                name.clone(),
                teacher.timezone.to_string(),
                teacher.name.clone(),
            );
            let cmd = format!("/user {edit_params}");

            let info = markdown_format!(
                "*Teacher: {}*\n\n\
                • Name: {}\n\
                • Timezone: {}\n\
                • Admin: {}\n\n\
                Click below to edit timezone and name\\.",
                name.to_string(),
                MarkdownString::escape(teacher.name),
                MarkdownString::escape(teacher.timezone.to_string()),
                admin_str
            );
            (info, cmd)
        }
        None => {
            let edit_params = UserParams::UTENF(
                name.clone(),
                "TZ".to_string(),
                "Name Surname".to_string(),
            );
            let cmd = format!("/user {edit_params}");
            let info = markdown_format!(
                "Teacher {} not found in cache \\(may not be synced yet\\)\\.\n\nEdit command \\(fill in values\\):",
                name.to_string()
            );
            (info, cmd)
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
    tz: String,
    currency: String,
    display_name: String,
) -> Result<()> {
    ctx.state
        .sheets
        .update_student(&name, &tz, &currency, &display_name)
        .await?;
    let _ = ctx.state.refresh().await;

    let text = markdown_format!(
        "✅ Student {} updated successfully\\.",
        name.to_string()
    );
    ctx.update_markdown_message(text, None).await
}

async fn exec_teacher_edit<Cmd: UserCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    name: TelegramName,
    tz: String,
    display_name: String,
) -> Result<()> {
    ctx.state
        .sheets
        .update_teacher(&name, &tz, &display_name)
        .await?;
    let _ = ctx.state.refresh().await;

    let text = markdown_format!(
        "✅ Teacher {} updated successfully\\.",
        name.to_string()
    );
    ctx.update_markdown_message(text, None).await
}
