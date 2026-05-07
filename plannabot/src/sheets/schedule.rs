//! Schedule data reader for the `Schedule` sheet tab.

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use std::collections::HashMap;

use super::from_sheet::FromSheetValue;
use super::{col_index_to_letter, SheetSchema, SheetsClient, SCHEDULE_COLS, SHEET_SCHEDULE};
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

    /// Deletes a planned lesson row from the Schedule sheet.
    /// Matches by teacher_telegram, student_telegram, and the date+time of the lesson.
    /// Returns an error if no matching planned lesson is found.
    pub async fn delete_schedule_entry(
        &self,
        teacher: &TelegramName,
        student: &TelegramName,
        date: NaiveDate,
        time: NaiveTime,
    ) -> Result<()> {
        let target_dt = date.and_time(time).and_utc();

        let range = format!("{SHEET_SCHEDULE}!A:Z");
        let rows = self
            .get_values(&range)
            .await
            .context("Failed to get Schedule sheet data")?;

        if rows.is_empty() {
            return Err(anyhow::anyhow!("Schedule sheet is empty"));
        }

        let schema = SheetSchema::new(SHEET_SCHEDULE.to_string(), rows[0].clone());

        for (row_idx, row) in rows.iter().enumerate().skip(1) {
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }

            let row_teacher: Option<TelegramName> =
                TelegramName::try_from(schema.get_str(row, ScheduleEntry::TEACHER_TELEGRAM)).ok();
            let row_student: Option<TelegramName> =
                TelegramName::try_from(schema.get_str(row, ScheduleEntry::STUDENT_TELEGRAM)).ok();
            if row_teacher.as_ref() != Some(teacher) || row_student.as_ref() != Some(student) {
                continue;
            }

            let row_dt = Self::row_datetime(&schema, row);
            let Some(row_dt) = row_dt else { continue };
            if row_dt != target_dt {
                continue;
            }

            let status_str = schema.get_str(row, ScheduleEntry::STATUS);
            if LessonStatus::from_str(status_str).is_some() {
                return Err(anyhow::anyhow!("Lesson is not planned (status: {})", status_str));
            }

            // row_idx is 0-based (0 = header row in the sheet).
            self.delete_row(SHEET_SCHEDULE, row_idx).await?;
            return Ok(());
        }

        Err(anyhow::anyhow!(
            "No planned lesson found for {} ↔ {} at {}",
            teacher,
            student,
            target_dt
        ))
    }

    /// Updates the status cell of a lesson identified by teacher, student, and datetime.
    pub async fn update_schedule_status(
        &self,
        teacher: &TelegramName,
        student: &TelegramName,
        date: NaiveDate,
        time: NaiveTime,
        status: &crate::models::LessonStatus,
    ) -> Result<()> {
        let target_dt = date.and_time(time).and_utc();

        let range = format!("{SHEET_SCHEDULE}!A:Z");
        let rows = self
            .get_values(&range)
            .await
            .context("Failed to get Schedule sheet data")?;

        if rows.is_empty() {
            return Err(anyhow::anyhow!("Schedule sheet is empty"));
        }

        let schema = SheetSchema::new(SHEET_SCHEDULE.to_string(), rows[0].clone());

        let status_col_idx = schema
            .get_col(ScheduleEntry::STATUS)
            .ok_or_else(|| anyhow::anyhow!("Schedule sheet has no 'status' column"))?;
        let status_col_letter = col_index_to_letter(status_col_idx);

        for (row_idx, row) in rows.iter().enumerate().skip(1) {
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }

            let row_teacher: Option<TelegramName> =
                TelegramName::try_from(schema.get_str(row, ScheduleEntry::TEACHER_TELEGRAM)).ok();
            let row_student: Option<TelegramName> =
                TelegramName::try_from(schema.get_str(row, ScheduleEntry::STUDENT_TELEGRAM)).ok();
            if row_teacher.as_ref() != Some(teacher) || row_student.as_ref() != Some(student) {
                continue;
            }

            let row_dt = Self::row_datetime(&schema, row);
            let Some(row_dt) = row_dt else { continue };
            if row_dt != target_dt {
                continue;
            }

            let row_num = row_idx + 1;
            let cell_range = format!("{SHEET_SCHEDULE}!{status_col_letter}{row_num}");
            self.update_values(
                &cell_range,
                vec![vec![serde_json::json!(status.as_sheet_str())]],
            )
            .await
            .context("Failed to write lesson status")?;
            return Ok(());
        }

        Err(anyhow::anyhow!(
            "No lesson found for {} ↔ {} at {}",
            teacher,
            student,
            target_dt
        ))
    }

    /// Reconstructs the UTC datetime from a row using the same logic as `get_schedule`.
    fn row_datetime(schema: &SheetSchema, row: &[String]) -> Option<DateTime<Utc>> {
        let date_str = schema.get_str(row, "date");
        let time_str = schema.get_str(row, "time");
        if !date_str.is_empty() && !time_str.is_empty() {
            let combined = format!("{} {}", date_str.trim(), time_str.trim());
            DateTime::<Utc>::from_sheet_value(&combined).ok()
        } else {
            let combined = schema.get_str(row, ScheduleEntry::DATETIME).to_string();
            DateTime::<Utc>::from_sheet_value(&combined).ok()
        }
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
