//! Schedule data reader for the `Schedule` sheet tab.

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};

use super::{SheetSchema, SheetsClient, SCHEDULE_COLS, SHEET_SCHEDULE};
use crate::models::{LessonStatus, ScheduleEntry, SheetParseError, TelegramName};

// ---------------------------------------------------------------------------
// Datetime parsing
// ---------------------------------------------------------------------------

/// Tries to parse a datetime string in several common formats.
///
/// Formats attempted (in order):
/// 1. RFC 3339 / ISO 8601 with timezone  (`2024-05-01T14:30:00+03:00`)
/// 2. `YYYY-MM-DD HH:MM:SS`              (treated as UTC)
/// 3. `YYYY-MM-DD HH:MM`                 (treated as UTC)
/// 4. `DD/MM/YYYY HH:MM`                 (treated as UTC)
///
/// Returns `None` if none of the formats match.
fn parse_datetime(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();

    // 1. RFC 3339 (includes timezone offset)
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }

    // 2. "YYYY-MM-DD HH:MM:SS" — naive, treat as UTC
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Some(ndt.and_utc());
    }

    // 3. "YYYY-MM-DD HH:MM" — naive, treat as UTC
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M") {
        return Some(ndt.and_utc());
    }

    // 4. "DD/MM/YYYY HH:MM" — naive, treat as UTC
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%d/%m/%Y %H:%M") {
        return Some(ndt.and_utc());
    }

    None
}

/// Tries to combine separate date and time strings into a DateTime.
///
/// Date formats attempted (in order):
/// 1. `YYYY-MM-DD`
/// 2. `DD/MM/YYYY`
///
/// Time format: `HH:MM` or `HH:MM:SS`
///
/// Returns `None` if date or time cannot be parsed.
fn parse_date_time(date_str: &str, time_str: &str) -> Option<DateTime<Utc>> {
    let date_str = date_str.trim();
    let time_str = time_str.trim();

    // Try to parse date in YYYY-MM-DD format
    if let Ok(ndt) =
        NaiveDateTime::parse_from_str(&format!("{} {}", date_str, time_str), "%Y-%m-%d %H:%M:%S")
    {
        return Some(ndt.and_utc());
    }

    if let Ok(ndt) =
        NaiveDateTime::parse_from_str(&format!("{} {}", date_str, time_str), "%Y-%m-%d %H:%M")
    {
        return Some(ndt.and_utc());
    }

    // Try to parse date in DD/MM/YYYY format
    if let Ok(ndt) =
        NaiveDateTime::parse_from_str(&format!("{} {}", date_str, time_str), "%d/%m/%Y %H:%M:%S")
    {
        return Some(ndt.and_utc());
    }

    if let Ok(ndt) =
        NaiveDateTime::parse_from_str(&format!("{} {}", date_str, time_str), "%d/%m/%Y %H:%M")
    {
        return Some(ndt.and_utc());
    }

    None
}

// ---------------------------------------------------------------------------
// Schedule readers
// ---------------------------------------------------------------------------

impl SheetsClient {
    /// Reads **all** rows from the `Schedule` sheet and returns them as a
    /// `Vec<ScheduleEntry>`, together with any [`SheetParseError`]s encountered
    /// while parsing rows.
    ///
    /// Rows with an unparseable `date` or `time` value are skipped with an
    /// error.  Telegram names are normalised (strip `'@'`, lowercase).
    /// Cost values accept either `.` or `,` as the decimal separator.
    /// `duration_minutes` defaults to `60` when the cell is empty or
    /// unparseable.
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

        // First row is always the header.
        let schema = SheetSchema::new(SHEET_SCHEDULE.to_string(), rows[0].clone());

        let mut entries: Vec<ScheduleEntry> = Vec::new();
        let mut errors: Vec<SheetParseError> = Vec::new();

        for (row_idx, row) in rows.iter().enumerate().skip(1) {
            // Skip empty rows (1-based sheet row number = row_idx + 1).
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }

            // --- datetime -------------------------------------------------------
            // Try to parse separate date and time columns first
            let date_str = schema.get_str(row, "date");
            let time_str = schema.get_str(row, "time");

            let datetime = if !date_str.is_empty() && !time_str.is_empty() {
                match parse_date_time(date_str, time_str) {
                    Some(dt) => dt,
                    None => {
                        let err = SheetParseError {
                            sheet: SHEET_SCHEDULE.to_string(),
                            row: row_idx + 1,
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
                // Fall back to parsing a combined datetime column if it exists
                let datetime_str = schema.get_str(row, "datetime");
                match parse_datetime(datetime_str) {
                    Some(dt) => dt,
                    None => {
                        let err = SheetParseError {
                            sheet: SHEET_SCHEDULE.to_string(),
                            row: row_idx + 1,
                            column: "datetime".to_string(),
                            message: format!("cannot parse datetime '{}'", datetime_str),
                        };
                        log::error!("{err}");
                        errors.push(err);
                        continue;
                    }
                }
            };

            // --- telegram names -------------------------------------------------
            let raw_student = schema.get_str(row, "student_telegram");
            let student_telegram = match TelegramName::try_from(raw_student) {
                Ok(n) => n,
                Err(e) => {
                    let err = SheetParseError {
                        sheet: SHEET_SCHEDULE.to_string(),
                        row: row_idx + 1,
                        column: "student_telegram".to_string(),
                        message: e.to_string(),
                    };
                    log::error!("{err}");
                    errors.push(err);
                    continue;
                }
            };

            let raw_teacher = schema.get_str(row, "teacher_telegram");
            let teacher_telegram = match TelegramName::try_from(raw_teacher) {
                Ok(n) => n,
                Err(e) => {
                    let err = SheetParseError {
                        sheet: SHEET_SCHEDULE.to_string(),
                        row: row_idx + 1,
                        column: "teacher_telegram".to_string(),
                        message: e.to_string(),
                    };
                    log::error!("{err}");
                    errors.push(err);
                    continue;
                }
            };

            // --- numeric fields -------------------------------------------------
            let cost_str = schema.get_str(row, "cost").replace(',', ".");
            let cost: f64 = cost_str.parse().unwrap_or(0.0);

            let duration_str = schema.get_str(row, "duration_minutes");
            let duration_minutes: i64 = duration_str.parse().unwrap_or(60);

            // --- status ---------------------------------------------------------
            let status: Option<LessonStatus> =
                LessonStatus::from_str(schema.get_str(row, "status"));

            // --- custom columns -------------------------------------------------
            let custom = schema.get_custom(row, SCHEDULE_COLS);

            entries.push(ScheduleEntry {
                student_telegram,
                teacher_telegram,
                datetime,
                duration_minutes,
                cost,
                status,
                custom,
            });
        }

        Ok((entries, errors))
    }

    /// Returns only the schedule entries for the given student.
    ///
    /// `telegram_name` is normalised and validated; an empty list is returned
    /// for invalid names.
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

    /// Returns only the schedule entries for the given teacher.
    ///
    /// `telegram_name` is normalised and validated; an empty list is returned
    /// for invalid names.
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
