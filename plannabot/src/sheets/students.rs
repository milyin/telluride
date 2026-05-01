//! Student data reader for the `Students` sheet tab.

use std::collections::HashMap;

use anyhow::{Context, Result};

use super::{SheetSchema, SheetsClient, SHEET_STUDENTS, STUDENTS_COLS};
use crate::models::Student;

impl SheetsClient {
    /// Reads all rows from the `Students` sheet and returns a map from
    /// (normalised) Telegram username → [`Student`].
    ///
    /// Normalisation: `'@'` prefix stripped, then lowercased.
    /// Rows whose `telegram_name` cell is empty are silently skipped.
    /// Any column not in [`STUDENTS_COLS`] is collected into [`Student::custom`].
    pub async fn get_students(&self) -> Result<HashMap<String, Student>> {
        let range = format!("{SHEET_STUDENTS}!A:Z");
        let rows = self
            .get_values(&range)
            .await
            .context("Failed to get Students sheet data")?;

        if rows.is_empty() {
            return Ok(HashMap::new());
        }

        // First row is always the header.
        let schema = SheetSchema::new(SHEET_STUDENTS.to_string(), rows[0].clone());

        let mut students: HashMap<String, Student> = HashMap::new();

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

            let student = Student {
                telegram_name: telegram_name.clone(),
                name: schema.get_str(row, "name").to_string(),
                timezone: schema.get_str(row, "timezone").to_string(),
                currency: schema.get_str(row, "currency").to_string(),
                zoom_url: schema.get_optional(row, "zoom_url").map(|s| s.to_string()),
                board_url: schema.get_optional(row, "board_url").map(|s| s.to_string()),
                custom: schema.get_custom(row, STUDENTS_COLS),
            };

            students.insert(telegram_name, student);
        }

        Ok(students)
    }
}
