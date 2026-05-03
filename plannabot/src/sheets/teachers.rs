//! Teacher data reader for the `Teachers` sheet tab.

use std::collections::HashMap;

use anyhow::{Context, Result};
use chrono_tz::Tz;

use super::{SheetSchema, SheetsClient, SHEET_TEACHERS, TEACHERS_COLS};
use crate::models::{SheetParseError, Teacher, TelegramName};

impl SheetsClient {
    /// Reads all rows from the `Teachers` sheet and returns a map from
    /// (normalised) Telegram username → [`Teacher`], together with any
    /// [`SheetParseError`]s encountered while parsing rows.
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

        let schema = SheetSchema::new(SHEET_TEACHERS.to_string(), rows[0].clone());

        let mut teachers: HashMap<String, Teacher> = HashMap::new();
        let mut errors: Vec<SheetParseError> = Vec::new();

        for (row_idx, row) in rows.iter().enumerate().skip(1) {
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }

            let row_num = row_idx + 1;

            let Some(telegram_name) = schema.get_field::<Option<TelegramName>>(row, row_num, Teacher::TELEGRAM_NAME, &mut errors) else {
                continue;
            };

            let teacher = Teacher {
                timezone: schema.get_field::<Tz>(row, row_num, Teacher::TIMEZONE, &mut errors),
                admin: schema.get_field::<bool>(row, row_num, Teacher::ADMIN, &mut errors),
                custom: schema.get_custom(row, TEACHERS_COLS),
                telegram_name: telegram_name.clone(),
            };

            teachers.insert(telegram_name.as_str().to_string(), teacher);
        }

        Ok((teachers, errors))
    }
}
