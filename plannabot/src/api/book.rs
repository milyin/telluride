use crate::types::Duration;
use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate, NaiveTime};
use telluride::command::CallbackBitcode;
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
use crate::api::traits::{BookCommand, BookParams, BookingActor};
use crate::models::{TelegramName, TimePeriod};
use crate::sheets::worktime::worktime_periods;

pub async fn book<Cmd: BookCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    params: &str,
    actor: &BookingActor,
) -> Result<()> {
    let params: BookParams = params.parse()?;
    match params {
        BookParams::L0() => book_L0(ctx, actor).await,
        BookParams::L1(teacher) => book_L1(ctx, teacher, actor).await,
        BookParams::L2(teacher, student) => book_L2(ctx, teacher, student).await,
        BookParams::L3(teacher, student, year) => book_L3(ctx, teacher, student, year).await,
        BookParams::L4(teacher, student, year, month) => {
            book_L4(ctx, teacher, student, year, month).await
        }
        BookParams::L5(teacher, student, year, month, day) => {
            book_L5(ctx, teacher, student, year, month, day).await
        }
        BookParams::L6(teacher, student, date) => book_L6(ctx, teacher, student, date).await,
        BookParams::L7(teacher, student, date, hour) => {
            book_L7(ctx, teacher, student, date, hour, actor).await
        }
        BookParams::L8(teacher, student, date, hour, duration) => {
            book_L8(ctx, teacher, student, date, hour, duration, actor).await
        }
    }
}

async fn book_L0<Cmd: BookCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    actor: &BookingActor,
) -> Result<()> {
    match actor {
        BookingActor::Student(student) => {
            let pairings = ctx.state.get_pairings_for_student(student).await;
            if pairings.len() == 1 {
                return book_L1(ctx, pairings[0].teacher_telegram.clone(), actor).await;
            }
            let names = pairings.into_iter().map(|p| p.teacher_telegram).collect();
            show_name_list(
                ctx,
                markdown_string!("📅 Select a teacher to book a lesson:"),
                markdown_string!("No paired teachers found\\."),
                names,
                |t| Cmd::book(BookParams::L1(t)),
                None,
            )
            .await
        }
        BookingActor::Teacher(teacher) => {
            book_L1(ctx, teacher.clone(), actor).await
        }
    }
}

async fn book_L1<Cmd: BookCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: TelegramName,
    actor: &BookingActor,
) -> Result<()> {
    match actor {
        BookingActor::Student(student) => {
            book_L2(ctx, teacher, student.clone()).await
        }
        BookingActor::Teacher(teacher_actor) => {
            let pairings = ctx.state.get_pairings_for_teacher(teacher_actor).await;
            if pairings.len() == 1 {
                return book_L2(ctx, teacher, pairings[0].student_telegram.clone()).await;
            }
            let t = teacher.clone();
            let names = pairings.into_iter().map(|p| p.student_telegram).collect();
            show_name_list(
                ctx,
                markdown_string!("📅 Select a student to book a lesson:"),
                markdown_string!("No paired students found\\."),
                names,
                move |s| Cmd::book(BookParams::L2(t.clone(), s)),
                None,
            )
            .await
        }
    }
}

async fn book_L2<Cmd: BookCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: TelegramName,
    student: TelegramName,
) -> Result<()> {
    let now = Local::now();
    book_L5(ctx, teacher, student, now.year(), now.month(), now.day()).await
}

async fn book_L3<Cmd: BookCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: TelegramName,
    student: TelegramName,
    year: i32,
) -> Result<()> {
    show_year_selection(
        ctx,
        year,
        move |y| Cmd::book(BookParams::L5(teacher.clone(), student.clone(), y, 1, 1)),
    )
    .await
}

async fn book_L4<Cmd: BookCommand + CallbackBitcode + 'static>(
    ctx: &BotCtx<Cmd>,
    teacher: TelegramName,
    student: TelegramName,
    year: i32,
    _month: u32,
) -> Result<()> {
    show_month_selection(
        ctx,
        year,
        move |m| Cmd::book(BookParams::L5(teacher.clone(), student.clone(), year, m, 1)),
    )
    .await
}

async fn book_L5<Cmd: BookCommand + CallbackBitcode + 'static>(
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
        move |date| Cmd::book(BookParams::L6(t_date.clone(), s_date.clone(), date)),
        move |py, pm| Cmd::book(BookParams::L5(t_prev.clone(), s_prev.clone(), py, pm, 1)),
        move |ny, nm| Cmd::book(BookParams::L5(t_next.clone(), s_next.clone(), ny, nm, 1)),
        move || Cmd::book(BookParams::L3(t_year.clone(), s_year.clone(), year)),
        move || {
            Cmd::book(BookParams::L4(
                t_month.clone(),
                s_month.clone(),
                year,
                month,
            ))
        },
        Some(Cmd::book(BookParams::L2(t_back, s_back))),
    )
    .await
}

async fn book_L6<Cmd: BookCommand + CallbackBitcode + 'static>(
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
        move |time| Cmd::book(BookParams::L7(t_cmd.clone(), s_cmd.clone(), date, time)),
        Some(Cmd::book(BookParams::L5(
            t_back,
            s_back,
            date.year(),
            date.month(),
            date.day(),
        ))),
    )
    .await
}

async fn book_L7<Cmd: Send + Sync + Clone>(
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

    let student_data = ctx.state.get_student(&student).await;
    let currency = student_data
        .as_ref()
        .map(|s| s.currency.as_str())
        .unwrap_or("");

    let mut text = markdown_string!("📅 *Book a Lesson*\n\n");
    let teacher_str = teacher.to_string();
    text.push(&markdown_format!("👨\\-🏫 Teacher: {}\n", teacher_str));
    let student_str = student.to_string();
    text.push(&markdown_format!("👨\\-🎓 Student: {}\n", student_str));
    let date_str = date.to_string();
    text.push(&markdown_format!("📆 Date: {}\n", date_str));
    let hour_str = hour.format("%H:%M").to_string();
    text.push(&markdown_format!("⏰ Time: {}\n", hour_str));
    let dur_str = format_duration(pairing.duration_minutes as i64);
    text.push(&markdown_format!("⏱ Duration: {}\n", dur_str));
    let cost_str = if currency.is_empty() {
        pairing.cost.to_string()
    } else {
        format!("{} {}", pairing.cost, currency)
    };
    text.push(&markdown_format!("💰 Cost: {}\n", cost_str));

    let l8_params = format!(
        "/book {}",
        BookParams::L8(teacher, student, date, hour, duration)
    );
    let button = InlineKeyboardButton::switch_inline_query_current_chat("📅 Book", l8_params);
    let keyboard = InlineKeyboardMarkup::new(vec![vec![button]]);

    ctx.bot
        .send_markdown_message(ctx.chat_id, text)
        .reply_markup(keyboard)
        .await?;
    Ok(())
}

async fn book_L8<Cmd: Send + Sync + Clone>(
    ctx: &BotCtx<Cmd>,
    teacher: TelegramName,
    student: TelegramName,
    date: NaiveDate,
    hour: NaiveTime,
    duration: Duration,
    actor: &BookingActor,
) -> Result<()> {
    match actor {
        BookingActor::Student(s) if s != &student => {
            let text = markdown_string!("⚠️ You can only book as yourself\\.");
            ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
            return Ok(());
        }
        BookingActor::Teacher(t) if t != &teacher => {
            let text = markdown_string!("⚠️ You can only book for your own students\\.");
            ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
            return Ok(());
        }
        _ => {}
    }

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
