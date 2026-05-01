//! Teacher data reader for the `Teachers` sheet tab.

use std::collections::HashMap;

use anyhow::{Context, Result};

use super::{SheetSchema, SheetsClient, SHEET_TEACHERS, TEACHERS_COLS};
use crate::models::Teacher;

impl SheetsClient {
    /// Reads all rows from the `Teachers` sheet and returns a map from
    /// (normalised) Telegram username → [`Teacher`].
    ///
    /// Normalisation: `'@'` prefix stripped, then lowercased.
    /// Rows whose `telegram_name` cell is empty are silently skipped.
    /// Any column not in [`TEACHERS_COLS`] is collected into [`Teacher::custom`].
    pub async fn get_teachers(&self) -> Result<HashMap<String, Teacher>> {
        let range = format!("{SHEET_TEACHERS}!A:Z");
        let rows = self
            .get_values(&range)
            .await
            .context("Failed to get Teachers sheet data")?;

        if rows.is_empty() {
            return Ok(HashMap::new());
        }

        // First row is always the header.
        let schema = SheetSchema::new(SHEET_TEACHERS.to_string(), rows[0].clone());

        let mut teachers: HashMap<String, Teacher> = HashMap::new();

        for row in rows.iter().skip(1) {
            // Skip empty rows.
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }

            let telegram_name = schema
                .get_str(row, "telegram_name")
                .trim_start_matches('@')
                .to_lowercase();

            if telegram_name.is_empty() {
                continue;
            }

            let teacher = Teacher {
                telegram_name: telegram_name.clone(),
                timezone: schema.get_str(row, "timezone").to_string(),
                custom: schema.get_custom(row, TEACHERS_COLS),
            };

            teachers.insert(telegram_name, teacher);
        }

        Ok(teachers)
    }
}
