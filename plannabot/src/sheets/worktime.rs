//! Worktime sheet reader and available-slot calculation.

use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate, NaiveTime, Timelike, Weekday};

use super::{SheetSchema, SheetsClient, SHEET_WORKTIME, WORKTIME_COLS};
use crate::models::{SheetParseError, TelegramName, Worktime};

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

/// Returns the sorted list of 1-hour time slots available for `teacher` on
/// `date`, derived from the supplied `worktime` rows.
///
/// A *slot* is identified by its start time (always on a full hour).  A slot
/// starting at hour `H` spans `H:00–(H+1):00`.
///
/// **Slot boundaries from a working window:**
/// - The first slot starts at the first full hour **≥ `start_time`**.
///   - `start_time = 11:30` → first slot at `12:00`
///   - `start_time = 11:00` → first slot at `11:00`
/// - A slot is included only if its end (`(H+1):00`) is **≤ `end_time`**.
///
/// **Override logic:** if any row for this teacher has `date == Some(date)`,
/// only those rows are used; otherwise rows matching `day_of_week` are used.
pub fn available_slots(worktime: &[Worktime], teacher: &TelegramName, date: NaiveDate) -> Vec<NaiveTime> {
    let weekday: Weekday = date.weekday();

    let for_teacher: Vec<&Worktime> = worktime
        .iter()
        .filter(|w| &w.teacher_telegram == teacher)
        .collect();

    // Specific-date rows override day-of-week rows.
    let specific: Vec<&Worktime> = for_teacher
        .iter()
        .filter(|w| w.date == Some(date))
        .copied()
        .collect();

    let applicable: Vec<&Worktime> = if !specific.is_empty() {
        specific
    } else {
        for_teacher
            .iter()
            .filter(|w| w.day_of_week == Some(weekday))
            .copied()
            .collect()
    };

    let mut slots: Vec<NaiveTime> = Vec::new();

    for entry in applicable {
        // First slot starts at the next full hour at or after start_time.
        let first_h = if entry.start_time.minute() == 0 && entry.start_time.second() == 0 {
            entry.start_time.hour()
        } else {
            entry.start_time.hour() + 1
        };

        let mut h = first_h;
        loop {
            let Some(slot_end) = NaiveTime::from_hms_opt(h + 1, 0, 0) else {
                break; // h+1 overflows valid hour range (≥24)
            };
            if slot_end > entry.end_time {
                break;
            }
            slots.push(NaiveTime::from_hms_opt(h, 0, 0).unwrap());
            h += 1;
        }
    }

    slots.sort();
    slots.dedup();
    slots
}
