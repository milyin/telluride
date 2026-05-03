//! Student data reader for the `Students` sheet tab.

use std::collections::HashMap;

use anyhow::{Context, Result};

use super::{
    parse_telegram_name, parse_timezone_or_utc, SheetSchema, SheetsClient, SHEET_STUDENTS,
    STUDENTS_COLS,
};
use crate::models::{SheetParseError, Student};

impl SheetsClient {
    /// Reads all rows from the `Students` sheet and returns a map from
    /// (normalised) Telegram username → [`Student`], together with any
    /// [`SheetParseError`]s encountered while parsing rows.
    ///
    /// Rows whose `telegram_name` cell is empty or contains an invalid username
    /// are skipped; the error is logged at `ERROR` level and collected.
    /// Any column not in [`STUDENTS_COLS`] is collected into [`Student::custom`].
    pub async fn get_students(
        &self,
    ) -> Result<(HashMap<String, Student>, Vec<SheetParseError>)> {
        let range = format!("{SHEET_STUDENTS}!A:Z");
        let rows = self
            .get_values(&range)
            .await
            .context("Failed to get Students sheet data")?;

        if rows.is_empty() {
            return Ok((HashMap::new(), vec![]));
        }

        // First row is always the header.
        let schema = SheetSchema::new(SHEET_STUDENTS.to_string(), rows[0].clone());

        let mut students: HashMap<String, Student> = HashMap::new();
        let mut errors: Vec<SheetParseError> = Vec::new();

        for (row_idx, row) in rows.iter().enumerate().skip(1) {
            // Skip empty rows.
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }

            let Some(telegram_name) = parse_telegram_name(
                SHEET_STUDENTS,
                row_idx + 1,
                Student::TELEGRAM_NAME,
                schema.get_str(row, Student::TELEGRAM_NAME),
                &mut errors,
                false,
            ) else {
                continue;
            };

            let timezone = parse_timezone_or_utc(
                SHEET_STUDENTS,
                row_idx + 1,
                Student::TIMEZONE,
                schema.get_str(row, Student::TIMEZONE),
                &mut errors,
            );

            let student = Student {
                telegram_name: telegram_name.clone(),
                name: schema.get_str(row, Student::NAME).to_string(),
                timezone,
                currency: schema.get_str(row, Student::CURRENCY).to_string(),
                zoom_url: schema.get_optional(row, Student::ZOOM_URL).map(|s| s.to_string()),
                board_url: schema.get_optional(row, Student::BOARD_URL).map(|s| s.to_string()),
                custom: schema.get_custom(row, STUDENTS_COLS),
            };

            students.insert(telegram_name.as_str().to_string(), student);
        }

        Ok((students, errors))
    }
}

