#![allow(non_snake_case)]
use crate::types::Duration;
use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate, NaiveTime};
use telluride::command::{CallbackBitcode, CallbackKey, InlineKeyboardButtonPackedExt};
use telluride::data_store::UserProxy;
use telluride::markdown::MarkdownStringMessage;
use telluride::{markdown_format, markdown_string};
use teloxide::payloads::SendMessageSetters;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::api::common::format_duration;
use crate::api::context::BotCtx;
use crate::api::menus::{
    show_date_selection, show_month_selection, show_name_list, show_slot_selection,
    show_year_selection,
};
use crate::api::traits::{BookCommand, BookParams, BookSubcmd, BookingActor};
use crate::models::{TelegramName, TimePeriod};
use crate::sheets::worktime::worktime_periods;

pub async fn book<Cmd: BookCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    params: &str,
    actor: &BookingActor,
) -> Result<()> {
    let params: BookParams = params.parse()?;
    match params {
        BookParams::M0 => book_M0(ctx).await,
        BookParams::M1(s) => book_M1(ctx, s, actor).await,
        BookParams::C0() => book_C0(ctx, actor).await,
        BookParams::C1(teacher) => book_C1(ctx, teacher, actor).await,
        BookParams::C2(teacher, student) => book_C2(ctx, teacher, student).await,
        BookParams::C3(teacher, student, year) => book_C3(ctx, teacher, student, year).await,
        BookParams::C4(teacher, student, year, month) => {
            book_C4(ctx, teacher, student, year, month).await
        }
        BookParams::C5(teacher, student, year, month, day) => {
            book_C5(ctx, teacher, student, year, month, day).await
        }
        BookParams::C6(teacher, student, date) => book_C6(ctx, teacher, student, date).await,
        BookParams::C7(teacher, student, date, hour) => {
            book_C7(ctx, teacher, student, date, hour, actor).await
        }
        BookParams::C8(teacher, student, date, hour, duration) => {
            book_C8(ctx, teacher, student, date, hour, duration).await
        }
        BookParams::CF(teacher, student, date, hour, duration) => {
            book_CF(ctx, teacher, student, date, hour, duration).await
        }
        BookParams::D0() => book_D0(ctx, actor).await,
        BookParams::D1(teacher, student, od, ot) => book_D1(ctx, teacher, student, od, ot).await,
        BookParams::DF(teacher, student, od, ot) => book_DF(ctx, teacher, student, od, ot).await,
        BookParams::E0() => book_E0(ctx, actor).await,
        BookParams::E1(teacher, student, od, ot, ny, nm) => {
            book_E1(ctx, teacher, student, od, ot, ny, nm).await
        }
        BookParams::E2(teacher, student, od, ot, nd) => {
            book_E2(ctx, teacher, student, od, ot, nd).await
        }
        BookParams::E3(teacher, student, od, ot, nd, nt) => {
            book_E3(ctx, teacher, student, od, ot, nd, nt).await
        }
        BookParams::EF(teacher, student, od, ot, nd, nt) => {
            book_EF(ctx, teacher, student, od, ot, nd, nt).await
        }
    }
}

// ---------------------------------------------------------------------------
// Menu
// ---------------------------------------------------------------------------

async fn book_M0<Cmd: BookCommand + CallbackBitcode + 'static>(ctx: &BotCtx<Cmd>) -> Result<()> {
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let create_key = CallbackKey::pack(Cmd::book(BookParams::M1(BookSubcmd::Create(false))), &user_proxy).await;
    let edit_key = CallbackKey::pack(Cmd::book(BookParams::M1(BookSubcmd::Edit(false))), &user_proxy).await;
    let delete_key = CallbackKey::pack(Cmd::book(BookParams::M1(BookSubcmd::Delete(false))), &user_proxy).await;
    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback_key("📅 Create", &create_key),
        InlineKeyboardButton::callback_key("✏️ Edit", &edit_key),
        InlineKeyboardButton::callback_key("🗑️ Cancel", &delete_key),
    ]]);
    ctx.bot
        .send_markdown_message(ctx.chat_id, markdown_string!("📅 *Book a Lesson*\nWhat would you like to do?"))
        .reply_markup(keyboard)
        .await?;
    Ok(())
}

async fn book_M1<Cmd: BookCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    subcmd: BookSubcmd,
    actor: &BookingActor,
) -> Result<()> {
    match subcmd {
        BookSubcmd::Create(_) => book_C0(ctx, actor).await,
        BookSubcmd::Edit(_) => book_E0(ctx, actor).await,
        BookSubcmd::Delete(_) => book_D0(ctx, actor).await,
    }
}

// ---------------------------------------------------------------------------
// Create flow (C0-C8, CF)
// ---------------------------------------------------------------------------

async fn book_C0<Cmd: BookCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    actor: &BookingActor,
) -> Result<()> {
    match actor {
        BookingActor::Student(student) => {
            let pairings = ctx.state.get_pairings_for_student(student).await;
            if pairings.len() == 1 {
                return book_C1(ctx, pairings[0].teacher_telegram.clone(), actor).await;
            }
            let names = pairings.into_iter().map(|p| p.teacher_telegram).collect();
            show_name_list(
                ctx,
                markdown_string!("📅 Select a teacher to book a lesson:"),
                markdown_string!("No paired teachers found\\."),
                names,
                |t| Cmd::book(BookParams::C1(t)),
                None,
            )
            .await
        }
        BookingActor::Teacher(teacher) => book_C1(ctx, teacher.clone(), actor).await,
    }
}

async fn book_C1<Cmd: BookCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: TelegramName,
    actor: &BookingActor,
) -> Result<()> {
    match actor {
        BookingActor::Student(student) => book_C2(ctx, teacher, student.clone()).await,
        BookingActor::Teacher(teacher_actor) => {
            let pairings = ctx.state.get_pairings_for_teacher(teacher_actor).await;
            if pairings.len() == 1 {
                return book_C2(ctx, teacher, pairings[0].student_telegram.clone()).await;
            }
            let t = teacher.clone();
            let names = pairings.into_iter().map(|p| p.student_telegram).collect();
            show_name_list(
                ctx,
                markdown_string!("📅 Select a student to book a lesson:"),
                markdown_string!("No paired students found\\."),
                names,
                move |s| Cmd::book(BookParams::C2(t.clone(), s)),
                None,
            )
            .await
        }
    }
}

async fn book_C2<Cmd: BookCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: TelegramName,
    student: TelegramName,
) -> Result<()> {
    let now = Local::now();
    book_C5(ctx, teacher, student, now.year(), now.month(), now.day()).await
}

async fn book_C3<Cmd: BookCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: TelegramName,
    student: TelegramName,
    year: i32,
) -> Result<()> {
    show_year_selection(
        ctx,
        year,
        move |y| Cmd::book(BookParams::C5(teacher.clone(), student.clone(), y, 1, 1)),
    )
    .await
}

async fn book_C4<Cmd: BookCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: TelegramName,
    student: TelegramName,
    year: i32,
    _month: u32,
) -> Result<()> {
    show_month_selection(
        ctx,
        year,
        move |m| Cmd::book(BookParams::C5(teacher.clone(), student.clone(), year, m, 1)),
    )
    .await
}

async fn book_C5<Cmd: BookCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: TelegramName,
    student: TelegramName,
    year: i32,
    month: u32,
    _day: u32,
) -> Result<()> {
    let t_date = teacher.clone();
    let s_date = student.clone();
    let t_prev = teacher.clone();
    let s_prev = student.clone();
    let t_next = teacher.clone();
    let s_next = student.clone();
    let t_year = teacher.clone();
    let s_year = student.clone();
    let t_month = teacher.clone();
    let s_month = student.clone();
    let t_back = teacher.clone();
    let s_back = student.clone();
    let message = markdown_format!(
        "📅 Book a Lesson\n{} ↔ {} — select a date:",
        teacher.to_string(),
        student.to_string()
    );
    show_date_selection(
        ctx,
        message,
        year,
        month,
        move |date| Cmd::book(BookParams::C6(t_date.clone(), s_date.clone(), date)),
        move |py, pm| Cmd::book(BookParams::C5(t_prev.clone(), s_prev.clone(), py, pm, 1)),
        move |ny, nm| Cmd::book(BookParams::C5(t_next.clone(), s_next.clone(), ny, nm, 1)),
        move || Cmd::book(BookParams::C3(t_year.clone(), s_year.clone(), year)),
        move || {
            Cmd::book(BookParams::C4(
                t_month.clone(),
                s_month.clone(),
                year,
                month,
            ))
        },
        Some(Cmd::book(BookParams::C2(t_back, s_back))),
    )
    .await
}

async fn book_C6<Cmd: BookCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: TelegramName,
    student: TelegramName,
    date: NaiveDate,
) -> Result<()> {
    let t_cmd = teacher.clone();
    let s_cmd = student.clone();
    let t_back = teacher.clone();
    let s_back = student.clone();
    show_slot_selection(
        ctx,
        &teacher,
        &student,
        date,
        move |time| Cmd::book(BookParams::C7(t_cmd.clone(), s_cmd.clone(), date, time)),
        Some(Cmd::book(BookParams::C5(
            t_back,
            s_back,
            date.year(),
            date.month(),
            date.day(),
        ))),
    )
    .await
}

async fn book_C7<Cmd: Send + Sync + Clone>(
    ctx: &BotCtx<Cmd>,
    teacher: TelegramName,
    student: TelegramName,
    date: NaiveDate,
    hour: NaiveTime,
    _actor: &BookingActor,
) -> Result<()> {
    let Some(pairing) = ctx.state.get_pairing(&student, &teacher).await else {
        let text = markdown_string!(
            "⚠️ No pairing found for this teacher\\. Please contact your teacher\\."
        );
        ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
        return Ok(());
    };
    let duration = Duration::from(std::time::Duration::from_secs(
        pairing.duration_minutes * 60,
    ));
    book_C8(ctx, teacher, student, date, hour, duration).await
}

async fn book_C8<Cmd: Send + Sync + Clone>(
    ctx: &BotCtx<Cmd>,
    teacher: TelegramName,
    student: TelegramName,
    date: NaiveDate,
    hour: NaiveTime,
    duration: Duration,
) -> Result<()> {
    let student_data = ctx.state.get_student(&student).await;
    let currency = student_data
        .as_ref()
        .map(|s| s.currency.as_str())
        .unwrap_or("");

    let Some(pairing) = ctx.state.get_pairing(&student, &teacher).await else {
        let text = markdown_string!(
            "⚠️ No pairing found for this teacher\\. Please contact your teacher\\."
        );
        ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
        return Ok(());
    };

    let duration_minutes = duration.as_secs() / 60;
    let actual_cost = if pairing.duration_minutes > 0 {
        pairing.cost * duration_minutes as i64 / pairing.duration_minutes as i64
    } else {
        pairing.cost
    };

    let mut text = markdown_string!("📅 *Book a Lesson*\n\n");
    let teacher_str = teacher.to_string();
    text.push(&markdown_format!("👨\\-🏫 Teacher: {}\n", teacher_str));
    let student_str = student.to_string();
    text.push(&markdown_format!("👨\\-🎓 Student: {}\n", student_str));
    let date_str = date.to_string();
    text.push(&markdown_format!("📆 Date: {}\n", date_str));
    let hour_str = hour.format("%H:%M").to_string();
    text.push(&markdown_format!("⏰ Time: {}\n", hour_str));
    let dur_str = format_duration(duration_minutes as i64);
    text.push(&markdown_format!("⏱ Duration: {}\n", dur_str));
    let cost_str = if currency.is_empty() {
        actual_cost.to_string()
    } else {
        format!("{} {}", actual_cost, currency)
    };
    text.push(&markdown_format!("💰 Cost: {}\n", cost_str));

    let cf_params = format!(
        "/book {}",
        BookParams::CF(teacher, student, date, hour, duration)
    );
    let button = InlineKeyboardButton::switch_inline_query_current_chat("📅 Book", cf_params);
    let keyboard = InlineKeyboardMarkup::new(vec![vec![button]]);

    ctx.bot
        .send_markdown_message(ctx.chat_id, text)
        .reply_markup(keyboard)
        .await?;
    Ok(())
}

async fn book_CF<Cmd: Send + Sync + Clone>(
    ctx: &BotCtx<Cmd>,
    teacher: TelegramName,
    student: TelegramName,
    date: NaiveDate,
    hour: NaiveTime,
    duration: Duration,
) -> Result<()> {
    let Some(pairing) = ctx.state.get_pairing(&student, &teacher).await else {
        let text = markdown_string!("⚠️ Cannot book: you are not paired with this teacher\\.");
        ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
        return Ok(());
    };

    let new_start = date.and_time(hour).and_utc();
    let new_period = TimePeriod::new(
        new_start,
        chrono::Duration::seconds(duration.as_secs() as i64),
    );

    let (worktime_entries, _) = ctx.state.sheets.get_worktime().await?;
    let fits_in_worktime = worktime_periods(&worktime_entries, &teacher, date)
        .iter()
        .any(|wp| wp.contains(&new_period));
    if !fits_in_worktime {
        let text = markdown_string!(
            "⚠️ Cannot book: the lesson extends outside the teacher's working hours\\."
        );
        ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
        return Ok(());
    }

    let (all_entries, _) = ctx.state.sheets.get_schedule().await?;
    let has_overlap = all_entries
        .iter()
        .filter(|e| e.is_planned())
        .filter(|e| e.student_telegram == student || e.teacher_telegram == teacher)
        .any(|e| e.time_period().overlaps(&new_period));

    if has_overlap {
        let text =
            markdown_string!("⚠️ Cannot book: this time slot conflicts with an existing lesson\\.");
        ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
        return Ok(());
    }

    let duration_minutes = duration.as_secs() / 60;
    let actual_cost = if pairing.duration_minutes > 0 {
        pairing.cost * duration_minutes as i64 / pairing.duration_minutes as i64
    } else {
        pairing.cost
    };

    let student_data = ctx.state.get_student(&student).await;
    let currency = student_data
        .as_ref()
        .map(|s| s.currency.as_str())
        .unwrap_or("");

    ctx.state
        .sheets
        .add_schedule_entry(&student, &teacher, new_start, duration_minutes, actual_cost)
        .await?;

    let mut text = markdown_string!("✅ *Lesson Booked\\!*\n\n");
    let teacher_str = teacher.to_string();
    text.push(&markdown_format!("👨\\-🏫 Teacher: {}\n", teacher_str));
    let student_str = student.to_string();
    text.push(&markdown_format!("👨\\-🎓 Student: {}\n", student_str));
    let date_str = date.to_string();
    text.push(&markdown_format!("📆 Date: {}\n", date_str));
    let hour_str = hour.format("%H:%M").to_string();
    text.push(&markdown_format!("⏰ Time: {}\n", hour_str));
    let dur_str = format_duration(duration_minutes as i64);
    text.push(&markdown_format!("⏱ Duration: {}\n", dur_str));
    let cost_str = if currency.is_empty() {
        actual_cost.to_string()
    } else {
        format!("{} {}", actual_cost, currency)
    };
    text.push(&markdown_format!("💰 Cost: {}\n", cost_str));

    ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Delete flow (D0, D1, DF)
// ---------------------------------------------------------------------------

async fn book_D0<Cmd: BookCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    actor: &BookingActor,
) -> Result<()> {
    let (all_entries, _) = ctx.state.sheets.get_schedule().await?;
    let planned: Vec<_> = all_entries
        .into_iter()
        .filter(|e| e.is_planned())
        .filter(|e| match actor {
            BookingActor::Student(s) => &e.student_telegram == s,
            BookingActor::Teacher(t) => &e.teacher_telegram == t,
        })
        .collect();

    if planned.is_empty() {
        ctx.bot
            .send_markdown_message(ctx.chat_id, markdown_string!("No planned lessons found\\."))
            .await?;
        return Ok(());
    }

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    for entry in planned {
        let dt = entry.datetime;
        let date = dt.date_naive();
        let time = dt.time();
        let label = format!(
            "{} {} ↔ {}",
            date.format("%Y-%m-%d"),
            time.format("%H:%M"),
            match actor {
                BookingActor::Student(_) => entry.teacher_telegram.to_string(),
                BookingActor::Teacher(_) => entry.student_telegram.to_string(),
            }
        );
        let cmd = Cmd::book(BookParams::D1(
            entry.teacher_telegram,
            entry.student_telegram,
            date,
            time,
        ));
        let key = CallbackKey::pack(cmd, &user_proxy).await;
        buttons.push(vec![InlineKeyboardButton::callback_key(label, &key)]);
    }

    let keyboard = InlineKeyboardMarkup::new(buttons);
    ctx.bot
        .send_markdown_message(ctx.chat_id, markdown_string!("🗑️ *Cancel a Lesson*\nSelect a lesson to cancel:"))
        .reply_markup(keyboard)
        .await?;
    Ok(())
}

async fn book_D1<Cmd: Send + Sync + Clone>(
    ctx: &BotCtx<Cmd>,
    teacher: TelegramName,
    student: TelegramName,
    od: NaiveDate,
    ot: NaiveTime,
) -> Result<()> {
    let mut text = markdown_string!("🗑️ *Cancel a Lesson*\n\nCancel this lesson?\n\n");
    text.push(&markdown_format!("👨\\-🏫 Teacher: {}\n", teacher.to_string()));
    text.push(&markdown_format!("👨\\-🎓 Student: {}\n", student.to_string()));
    text.push(&markdown_format!("📆 Date: {}\n", od.to_string()));
    text.push(&markdown_format!("⏰ Time: {}\n", ot.format("%H:%M").to_string()));

    let df_params = format!("/book {}", BookParams::DF(teacher, student, od, ot));
    let button = InlineKeyboardButton::switch_inline_query_current_chat("🗑️ Yes, cancel it", df_params);
    let keyboard = InlineKeyboardMarkup::new(vec![vec![button]]);

    ctx.bot
        .send_markdown_message(ctx.chat_id, text)
        .reply_markup(keyboard)
        .await?;
    Ok(())
}

async fn book_DF<Cmd: Send + Sync + Clone>(
    ctx: &BotCtx<Cmd>,
    teacher: TelegramName,
    student: TelegramName,
    od: NaiveDate,
    ot: NaiveTime,
) -> Result<()> {
    ctx.state
        .sheets
        .cancel_schedule_entry(&teacher, &student, od, ot)
        .await?;

    let mut text = markdown_string!("✅ *Lesson Cancelled*\n\n");
    text.push(&markdown_format!("👨\\-🏫 Teacher: {}\n", teacher.to_string()));
    text.push(&markdown_format!("👨\\-🎓 Student: {}\n", student.to_string()));
    text.push(&markdown_format!("📆 Date: {}\n", od.to_string()));
    text.push(&markdown_format!("⏰ Time: {}\n", ot.format("%H:%M").to_string()));

    ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Edit flow (E0, E1, E2, E3, EF)
// ---------------------------------------------------------------------------

async fn book_E0<Cmd: BookCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    actor: &BookingActor,
) -> Result<()> {
    let (all_entries, _) = ctx.state.sheets.get_schedule().await?;
    let planned: Vec<_> = all_entries
        .into_iter()
        .filter(|e| e.is_planned())
        .filter(|e| match actor {
            BookingActor::Student(s) => &e.student_telegram == s,
            BookingActor::Teacher(t) => &e.teacher_telegram == t,
        })
        .collect();

    if planned.is_empty() {
        ctx.bot
            .send_markdown_message(ctx.chat_id, markdown_string!("No planned lessons found\\."))
            .await?;
        return Ok(());
    }

    let now = Local::now();
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    for entry in planned {
        let dt = entry.datetime;
        let date = dt.date_naive();
        let time = dt.time();
        let label = format!(
            "{} {} ↔ {}",
            date.format("%Y-%m-%d"),
            time.format("%H:%M"),
            match actor {
                BookingActor::Student(_) => entry.teacher_telegram.to_string(),
                BookingActor::Teacher(_) => entry.student_telegram.to_string(),
            }
        );
        let cmd = Cmd::book(BookParams::E1(
            entry.teacher_telegram,
            entry.student_telegram,
            date,
            time,
            now.year(),
            now.month(),
        ));
        let key = CallbackKey::pack(cmd, &user_proxy).await;
        buttons.push(vec![InlineKeyboardButton::callback_key(label, &key)]);
    }

    let keyboard = InlineKeyboardMarkup::new(buttons);
    ctx.bot
        .send_markdown_message(ctx.chat_id, markdown_string!("✏️ *Edit a Lesson*\nSelect a lesson to reschedule:"))
        .reply_markup(keyboard)
        .await?;
    Ok(())
}

async fn book_E1<Cmd: BookCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: TelegramName,
    student: TelegramName,
    od: NaiveDate,
    ot: NaiveTime,
    year: i32,
    month: u32,
) -> Result<()> {
    let t_date = teacher.clone();
    let s_date = student.clone();
    let t_prev = teacher.clone();
    let s_prev = student.clone();
    let t_next = teacher.clone();
    let s_next = student.clone();
    let t_back = teacher.clone();
    let s_back = student.clone();

    let message = markdown_format!(
        "✏️ Edit Lesson\n{} ↔ {} — select a new date:",
        teacher.to_string(),
        student.to_string()
    );
    show_date_selection(
        ctx,
        message,
        year,
        month,
        move |date| Cmd::book(BookParams::E2(t_date.clone(), s_date.clone(), od, ot, date)),
        move |py, pm| Cmd::book(BookParams::E1(t_prev.clone(), s_prev.clone(), od, ot, py, pm)),
        move |ny, nm| Cmd::book(BookParams::E1(t_next.clone(), s_next.clone(), od, ot, ny, nm)),
        move || Cmd::book(BookParams::E1(t_back.clone(), s_back.clone(), od, ot, year - 1, month)),
        move || {
            // month nav: cycle months
            let (prev_y, prev_m) = if month > 1 {
                (year, month - 1)
            } else {
                (year - 1, 12)
            };
            Cmd::book(BookParams::E1(teacher.clone(), student.clone(), od, ot, prev_y, prev_m))
        },
        Some(Cmd::book(BookParams::E0())),
    )
    .await
}

async fn book_E2<Cmd: BookCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: TelegramName,
    student: TelegramName,
    od: NaiveDate,
    ot: NaiveTime,
    nd: NaiveDate,
) -> Result<()> {
    let t_cmd = teacher.clone();
    let s_cmd = student.clone();
    let t_back = teacher.clone();
    let s_back = student.clone();
    show_slot_selection(
        ctx,
        &teacher,
        &student,
        nd,
        move |new_time| {
            Cmd::book(BookParams::E3(
                t_cmd.clone(),
                s_cmd.clone(),
                od,
                ot,
                nd,
                new_time,
            ))
        },
        Some(Cmd::book(BookParams::E1(
            t_back,
            s_back,
            od,
            ot,
            nd.year(),
            nd.month(),
        ))),
    )
    .await
}

async fn book_E3<Cmd: Send + Sync + Clone>(
    ctx: &BotCtx<Cmd>,
    teacher: TelegramName,
    student: TelegramName,
    od: NaiveDate,
    ot: NaiveTime,
    nd: NaiveDate,
    nt: NaiveTime,
) -> Result<()> {
    let mut text = markdown_string!("✏️ *Reschedule Lesson*\n\n");
    text.push(&markdown_format!("👨\\-🏫 Teacher: {}\n", teacher.to_string()));
    text.push(&markdown_format!("👨\\-🎓 Student: {}\n", student.to_string()));
    text.push(&markdown_format!("📆 Old date: {}\n", od.to_string()));
    text.push(&markdown_format!("⏰ Old time: {}\n", ot.format("%H:%M").to_string()));
    text.push(&markdown_format!("📆 New date: {}\n", nd.to_string()));
    text.push(&markdown_format!("⏰ New time: {}\n", nt.format("%H:%M").to_string()));

    let ef_params = format!(
        "/book {}",
        BookParams::EF(teacher, student, od, ot, nd, nt)
    );
    let button =
        InlineKeyboardButton::switch_inline_query_current_chat("✅ Confirm reschedule", ef_params);
    let keyboard = InlineKeyboardMarkup::new(vec![vec![button]]);

    ctx.bot
        .send_markdown_message(ctx.chat_id, text)
        .reply_markup(keyboard)
        .await?;
    Ok(())
}

async fn book_EF<Cmd: Send + Sync + Clone>(
    ctx: &BotCtx<Cmd>,
    teacher: TelegramName,
    student: TelegramName,
    od: NaiveDate,
    ot: NaiveTime,
    nd: NaiveDate,
    nt: NaiveTime,
) -> Result<()> {
    let Some(pairing) = ctx.state.get_pairing(&student, &teacher).await else {
        let text = markdown_string!("⚠️ Cannot reschedule: pairing not found\\.");
        ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
        return Ok(());
    };

    let new_start = nd.and_time(nt).and_utc();
    let new_period = TimePeriod::new(
        new_start,
        chrono::Duration::seconds(pairing.duration_minutes as i64 * 60),
    );
    let old_start = od.and_time(ot).and_utc();

    let (worktime_entries, _) = ctx.state.sheets.get_worktime().await?;
    let fits_in_worktime = worktime_periods(&worktime_entries, &teacher, nd)
        .iter()
        .any(|wp| wp.contains(&new_period));
    if !fits_in_worktime {
        let text = markdown_string!(
            "⚠️ Cannot reschedule: new time extends outside the teacher's working hours\\."
        );
        ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
        return Ok(());
    }

    let (all_entries, _) = ctx.state.sheets.get_schedule().await?;
    let has_overlap = all_entries
        .iter()
        .filter(|e| e.is_planned())
        .filter(|e| e.student_telegram == student || e.teacher_telegram == teacher)
        // Exclude the old booking itself from the overlap check.
        .filter(|e| e.datetime != old_start)
        .any(|e| e.time_period().overlaps(&new_period));

    if has_overlap {
        let text = markdown_string!(
            "⚠️ Cannot reschedule: new time conflicts with an existing lesson\\."
        );
        ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
        return Ok(());
    }

    ctx.state
        .sheets
        .cancel_schedule_entry(&teacher, &student, od, ot)
        .await?;

    let duration_minutes = pairing.duration_minutes;
    let actual_cost = pairing.cost;

    ctx.state
        .sheets
        .add_schedule_entry(&student, &teacher, new_start, duration_minutes, actual_cost)
        .await?;

    let mut text = markdown_string!("✅ *Lesson Rescheduled\\!*\n\n");
    text.push(&markdown_format!("👨\\-🏫 Teacher: {}\n", teacher.to_string()));
    text.push(&markdown_format!("👨\\-🎓 Student: {}\n", student.to_string()));
    text.push(&markdown_format!("📆 New date: {}\n", nd.to_string()));
    text.push(&markdown_format!("⏰ New time: {}\n", nt.format("%H:%M").to_string()));

    ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
    Ok(())
}
