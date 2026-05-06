//! Worktime sheet reader and available-slot calculation.

use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate, NaiveTime, Weekday};

use super::{SheetSchema, SheetsClient, SHEET_WORKTIME, WORKTIME_COLS};
use crate::models::{ScheduleEntry, SheetParseError, TelegramName, TimePeriod, Worktime};

impl SheetsClient {
    /// Reads all rows from the `Worktime` sheet and returns them together with
    /// any [`SheetParseError`]s encountered while parsing.
    pub async fn get_worktime(&self) -> Result<(Vec<Worktime>, Vec<SheetParseError>)> {
        let range = format!("{SHEET_WORKTIME}!A:Z");
        let rows = self
            .get_values(&range)
            .await
            .context("Failed to get Worktime sheet data")?;

        if rows.is_empty() {
            return Ok((vec![], vec![]));
        }

        let schema = SheetSchema::new(SHEET_WORKTIME.to_string(), rows[0].clone());
        let mut entries: Vec<Worktime> = Vec::new();
        let mut errors: Vec<SheetParseError> = Vec::new();

        for (row_idx, row) in rows.iter().enumerate().skip(1) {
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }
            let row_num = row_idx + 1;

            let Some(teacher_telegram) = schema.get_field::<Option<TelegramName>>(
                row, row_num, Worktime::TEACHER_TELEGRAM, &mut errors,
            ) else {
                continue;
            };

            entries.push(Worktime {
                teacher_telegram,
                day_of_week: schema.get_field(row, row_num, Worktime::DAY_OF_WEEK, &mut errors),
                date: schema.get_field(row, row_num, Worktime::DATE, &mut errors),
                start_time: schema.get_field(row, row_num, Worktime::START_TIME, &mut errors),
                end_time: schema.get_field(row, row_num, Worktime::END_TIME, &mut errors),
                custom: schema.get_custom(row, WORKTIME_COLS),
            });
        }

        Ok((entries, errors))
    }
}

/// Returns the applicable worktime windows for `teacher` on `date` as [`TimePeriod`]s.
///
/// Specific-date rows take precedence over day-of-week rows.
pub fn worktime_periods(worktime: &[Worktime], teacher: &TelegramName, date: NaiveDate) -> Vec<TimePeriod> {
    let weekday: Weekday = date.weekday();

    let for_teacher: Vec<&Worktime> =
        worktime.iter().filter(|w| &w.teacher_telegram == teacher).collect();

    let specific: Vec<&Worktime> =
        for_teacher.iter().filter(|w| w.date == Some(date)).copied().collect();

    let applicable: Vec<&Worktime> = if !specific.is_empty() {
        specific
    } else {
        for_teacher.iter().filter(|w| w.day_of_week == Some(weekday)).copied().collect()
    };

    applicable.iter().map(|w| w.time_period(date)).collect()
}

/// Returns the sorted list of free slots for `teacher` / `student` on `date`.
///
/// Only slots where a lesson of `lesson_duration` fits entirely within the remaining free
/// worktime (after subtracting existing planned lessons) are returned.
pub fn available_slots(
    worktime: &[Worktime],
    schedule: &[ScheduleEntry],
    teacher: &TelegramName,
    student: &TelegramName,
    date: NaiveDate,
    lesson_duration: chrono::Duration,
) -> Vec<NaiveTime> {
    let mut free = worktime_periods(worktime, teacher, date);

    // Subtract planned lessons that block the teacher or student.
    let booked: Vec<TimePeriod> = schedule
        .iter()
        .filter(|e| e.is_planned())
        .filter(|e| &e.teacher_telegram == teacher || &e.student_telegram == student)
        .map(|e| e.time_period())
        .collect();

    for blocked in &booked {
        free = free.into_iter().flat_map(|p| p.subtract(blocked)).collect();
    }

    // Collect hour-aligned slots of lesson_duration from all remaining free windows.
    let mut slots: Vec<NaiveTime> =
        free.iter().flat_map(|p| p.hour_slots(lesson_duration)).map(|dt| dt.time()).collect();

    slots.sort();
    slots.dedup();
    slots
}
