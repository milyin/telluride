//! Teacher data reader for the `Teachers` sheet tab.

use std::collections::HashMap;

use anyhow::{Context, Result};

use super::{
    parse_telegram_name, parse_timezone_or_utc, SheetSchema, SheetsClient, SHEET_TEACHERS,
    TEACHERS_COLS,
};
use crate::models::{SheetParseError, Teacher};

impl SheetsClient {
    /// Reads all rows from the `Teachers` sheet and returns a map from
    /// (normalised) Telegram username → [`Teacher`], together with any
    /// [`SheetParseError`]s encountered while parsing rows.
    ///
    /// Normalisation: `'@'` prefix stripped, then lowercased.
    /// Rows whose `telegram_name` cell is empty are silently skipped; rows
    /// with a non-empty but invalid value are logged at `ERROR` level and
    /// collected as errors.
    /// Any column not in [`TEACHERS_COLS`] is collected into [`Teacher::custom`].
    pub async fn get_teachers(
        &self,
    ) -> Result<(HashMap<String, Teacher>, Vec<SheetParseError>)> {
        let range = format!("{SHEET_TEACHERS}!A:Z");
        let rows = self
            .get_values(&range)
            .await
            .context("Failed to get Teachers sheet data")?;

        if rows.is_empty() {
            return Ok((HashMap::new(), vec![]));
        }

        // First row is always the header.
        let schema = SheetSchema::new(SHEET_TEACHERS.to_string(), rows[0].clone());

        let mut teachers: HashMap<String, Teacher> = HashMap::new();
        let mut errors: Vec<SheetParseError> = Vec::new();

        for (row_idx, row) in rows.iter().enumerate().skip(1) {
            // Skip empty rows.
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }

            let Some(telegram_name) = parse_telegram_name(
                SHEET_TEACHERS,
                row_idx + 1,
                Teacher::TELEGRAM_NAME,
                schema.get_str(row, Teacher::TELEGRAM_NAME),
                &mut errors,
                false,
            ) else {
                continue;
            };

            let timezone = parse_timezone_or_utc(
                SHEET_TEACHERS,
                row_idx + 1,
                Teacher::TIMEZONE,
                schema.get_str(row, Teacher::TIMEZONE),
                &mut errors,
            );

            let teacher = Teacher {
                telegram_name: telegram_name.clone(),
                timezone,
                custom: schema.get_custom(row, TEACHERS_COLS),
            };

            teachers.insert(telegram_name.as_str().to_string(), teacher);
        }

        Ok((teachers, errors))
    }
}
