//! Schedule data reader for the `Schedule` sheet tab.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

use super::from_sheet::FromSheetValue;
use super::{SheetSchema, SheetsClient, SCHEDULE_COLS, SHEET_SCHEDULE};
use crate::models::{LessonStatus, ScheduleEntry, SheetParseError, TelegramName};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Combines separate date and time strings and parses them as a UTC datetime.
/// Delegates format detection to [`FromSheetValue`] for `DateTime<Utc>`.
fn parse_date_time(date_str: &str, time_str: &str) -> Option<DateTime<Utc>> {
    let combined = format!("{} {}", date_str.trim(), time_str.trim());
    DateTime::<Utc>::from_sheet_value(&combined).ok()
}

// ---------------------------------------------------------------------------
// Schedule readers
// ---------------------------------------------------------------------------

impl SheetsClient {
    /// Reads **all** rows from the `Schedule` sheet and returns them as a
    /// `Vec<ScheduleEntry>`, together with any [`SheetParseError`]s encountered
    /// while parsing rows.
    pub async fn get_schedule(
        &self,
    ) -> Result<(Vec<ScheduleEntry>, Vec<SheetParseError>)> {
        let range = format!("{SHEET_SCHEDULE}!A:Z");
        let rows = self
            .get_values(&range)
            .await
            .context("Failed to get Schedule sheet data")?;

        if rows.is_empty() {
            return Ok((vec![], vec![]));
        }

        let schema = SheetSchema::new(SHEET_SCHEDULE.to_string(), rows[0].clone());

        let mut entries: Vec<ScheduleEntry> = Vec::new();
        let mut errors: Vec<SheetParseError> = Vec::new();

        for (row_idx, row) in rows.iter().enumerate().skip(1) {
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }

            let row_num = row_idx + 1;

            // --- datetime -------------------------------------------------------
            // Try separate date + time columns first, then a combined datetime.
            let date_str = schema.get_str(row, "date");
            let time_str = schema.get_str(row, "time");

            let datetime = if !date_str.is_empty() && !time_str.is_empty() {
                match parse_date_time(date_str, time_str) {
                    Some(dt) => dt,
                    None => {
                        let err = SheetParseError {
                            sheet: SHEET_SCHEDULE.to_string(),
                            row: row_num,
                            column: "date/time".to_string(),
                            message: format!(
                                "cannot parse date '{}' and time '{}'",
                                date_str, time_str
                            ),
                        };
                        log::error!("{err}");
                        errors.push(err);
                        continue;
                    }
                }
            } else {
                match schema.get_field::<Option<DateTime<Utc>>>(row, row_num, ScheduleEntry::DATETIME, &mut errors) {
                    Some(dt) => dt,
                    None => continue,
                }
            };

            // --- telegram names -------------------------------------------------
            let Some(student_telegram) = schema.get_field::<Option<TelegramName>>(row, row_num, ScheduleEntry::STUDENT_TELEGRAM, &mut errors) else {
                continue;
            };
            let Some(teacher_telegram) = schema.get_field::<Option<TelegramName>>(row, row_num, ScheduleEntry::TEACHER_TELEGRAM, &mut errors) else {
                continue;
            };

            // --- numeric fields -------------------------------------------------
            let cost: i64 = schema.get_field(row, row_num, ScheduleEntry::COST, &mut errors);
            let duration_minutes: u64 = schema
                .get_str(row, ScheduleEntry::DURATION_MINUTES)
                .parse()
                .unwrap_or(60);

            // --- status ---------------------------------------------------------
            let status: Option<LessonStatus> =
                schema.get_field(row, row_num, ScheduleEntry::STATUS, &mut errors);

            entries.push(ScheduleEntry {
                student_telegram,
                teacher_telegram,
                datetime,
                duration_minutes,
                cost,
                status,
                custom: schema.get_custom(row, SCHEDULE_COLS),
            });
        }

        Ok((entries, errors))
    }

    /// Returns only the schedule entries for the given student.
    pub async fn get_student_schedule(&self, telegram_name: &str) -> Result<Vec<ScheduleEntry>> {
        let Ok(normalised) = TelegramName::try_from(telegram_name) else {
            return Ok(vec![]);
        };
        let (all, _) = self.get_schedule().await?;
        Ok(all
            .into_iter()
            .filter(|e| e.student_telegram == normalised)
            .collect())
    }

    /// Appends a new planned lesson to the Schedule sheet.
    pub async fn add_schedule_entry(
        &self,
        student_telegram: &TelegramName,
        teacher_telegram: &TelegramName,
        datetime: DateTime<Utc>,
        duration_minutes: u64,
        cost: i64,
    ) -> Result<()> {
        let header_range = format!("{SHEET_SCHEDULE}!1:1");
        let rows = self
            .get_values(&header_range)
            .await
            .context("Failed to read Schedule header row")?;
        let headers = rows.into_iter().next().unwrap_or_default();

        let datetime_str = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
        let mut values: HashMap<&str, String> = HashMap::new();
        values.insert("student_telegram", student_telegram.as_str().to_string());
        values.insert("teacher_telegram", teacher_telegram.as_str().to_string());
        values.insert("datetime", datetime_str);
        values.insert("duration_minutes", duration_minutes.to_string());
        values.insert("cost", cost.to_string());

        let row: Vec<serde_json::Value> = headers
            .iter()
            .map(|h| {
                serde_json::Value::String(
                    values.get(h.as_str()).cloned().unwrap_or_default(),
                )
            })
            .collect();

        self.append_row(SHEET_SCHEDULE, row).await
    }

    /// Returns only the schedule entries for the given teacher.
    pub async fn get_teacher_schedule(&self, telegram_name: &str) -> Result<Vec<ScheduleEntry>> {
        let Ok(normalised) = TelegramName::try_from(telegram_name) else {
            return Ok(vec![]);
        };
        let (all, _) = self.get_schedule().await?;
        Ok(all
            .into_iter()
            .filter(|e| e.teacher_telegram == normalised)
            .collect())
    }
}
