//! Student data reader for the `Students` sheet tab.

use std::collections::HashMap;

use anyhow::{Context, Result};
use chrono_tz::Tz;

use super::{SheetSchema, SheetsClient, SHEET_STUDENTS, STUDENTS_COLS};
use crate::models::{SheetParseError, Student, TelegramName};

impl SheetsClient {
    /// Reads all rows from the `Students` sheet and returns a map from
    /// (normalised) Telegram username → [`Student`], together with any
    /// [`SheetParseError`]s encountered while parsing rows.
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

        let schema = SheetSchema::new(SHEET_STUDENTS.to_string(), rows[0].clone());

        let mut students: HashMap<String, Student> = HashMap::new();
        let mut errors: Vec<SheetParseError> = Vec::new();

        for (row_idx, row) in rows.iter().enumerate().skip(1) {
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }

            let row_num = row_idx + 1;

            let Some(telegram_name) = schema.get_field::<Option<TelegramName>>(row, row_num, Student::TELEGRAM_NAME, &mut errors) else {
                continue;
            };

            let student = Student {
                timezone: schema.get_field::<Tz>(row, row_num, Student::TIMEZONE, &mut errors),
                name: schema.get_field(row, row_num, Student::NAME, &mut errors),
                currency: schema.get_field(row, row_num, Student::CURRENCY, &mut errors),
                zoom_url: schema.get_field(row, row_num, Student::ZOOM_URL, &mut errors),
                board_url: schema.get_field(row, row_num, Student::BOARD_URL, &mut errors),
                custom: schema.get_custom(row, STUDENTS_COLS),
                telegram_name: telegram_name.clone(),
            };

            students.insert(telegram_name.as_str().to_string(), student);
        }

        Ok((students, errors))
    }
}
