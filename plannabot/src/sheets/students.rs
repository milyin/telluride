//! Student data reader and writer for the `Students` sheet tab.

use std::collections::HashMap;

use anyhow::{Context, Result};
use chrono_tz::Tz;

use super::{SheetSchema, SheetsClient, SHEET_STUDENTS, STUDENTS_COLS, col_index_to_letter};
use crate::models::{SheetParseError, Student, TelegramName};
use crate::types::Duration;

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
                chat_id: schema.get_field(row, row_num, Student::CHAT_ID, &mut errors),
                notification_delay: schema.get_field(row, row_num, Student::NOTIFICATION_DELAY, &mut errors),
                custom: schema.get_custom(row, STUDENTS_COLS),
                telegram_name: telegram_name.clone(),
            };

            students.insert(telegram_name.as_str().to_string(), student);
        }

        Ok((students, errors))
    }

    /// Appends a new student row to the `Students` sheet.
    pub async fn add_student(
        &self,
        telegram_name: &TelegramName,
        tz: &str,
        currency: &str,
        name: &str,
    ) -> Result<()> {
        let header_range = format!("{SHEET_STUDENTS}!1:1");
        let rows = self
            .get_values(&header_range)
            .await
            .context("Failed to read Students header row")?;
        let headers = rows.into_iter().next().unwrap_or_default();

        let mut values: HashMap<&str, String> = HashMap::new();
        values.insert(Student::TELEGRAM_NAME, telegram_name.as_str().to_string());
        values.insert(Student::NAME, name.to_string());
        values.insert(Student::TIMEZONE, tz.to_string());
        values.insert(Student::CURRENCY, currency.to_string());

        let row: Vec<serde_json::Value> = headers
            .iter()
            .map(|h| serde_json::Value::String(values.get(h.as_str()).cloned().unwrap_or_default()))
            .collect();

        self.append_row(SHEET_STUDENTS, row).await
    }

    /// Deletes the student row identified by `telegram_name`.
    pub async fn delete_student(&self, telegram_name: &TelegramName) -> Result<()> {
        let range = format!("{SHEET_STUDENTS}!A:Z");
        let rows = self
            .get_values(&range)
            .await
            .context("Failed to get Students sheet data")?;

        if rows.is_empty() {
            return Err(anyhow::anyhow!("Students sheet is empty"));
        }

        let schema = SheetSchema::new(SHEET_STUDENTS.to_string(), rows[0].clone());

        for (row_idx, row) in rows.iter().enumerate().skip(1) {
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }
            let row_name = TelegramName::try_from(schema.get_str(row, Student::TELEGRAM_NAME)).ok();
            if row_name.as_ref() == Some(telegram_name) {
                self.delete_row(SHEET_STUDENTS, row_idx).await?;
                return Ok(());
            }
        }

        Err(anyhow::anyhow!("Student {} not found in sheet", telegram_name))
    }

    /// Updates the timezone, currency, and name fields of an existing student row.
    pub async fn update_student(
        &self,
        telegram_name: &TelegramName,
        tz: &str,
        currency: &str,
        name: &str,
    ) -> Result<()> {
        let range = format!("{SHEET_STUDENTS}!A:Z");
        let rows = self
            .get_values(&range)
            .await
            .context("Failed to get Students sheet data")?;

        if rows.is_empty() {
            return Err(anyhow::anyhow!("Students sheet is empty"));
        }

        let schema = SheetSchema::new(SHEET_STUDENTS.to_string(), rows[0].clone());

        for (row_idx, row) in rows.iter().enumerate().skip(1) {
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }
            let row_name = TelegramName::try_from(schema.get_str(row, Student::TELEGRAM_NAME)).ok();
            if row_name.as_ref() != Some(telegram_name) {
                continue;
            }

            // Build a full row with updated fields
            let mut updated_row: Vec<String> = row.clone();
            // Ensure row is long enough to cover all known columns
            let needed_len = schema.headers.len();
            if updated_row.len() < needed_len {
                updated_row.resize(needed_len, String::new());
            }

            if let Some(idx) = schema.get_col(Student::TIMEZONE) {
                updated_row[idx] = tz.to_string();
            }
            if let Some(idx) = schema.get_col(Student::CURRENCY) {
                updated_row[idx] = currency.to_string();
            }
            if let Some(idx) = schema.get_col(Student::NAME) {
                updated_row[idx] = name.to_string();
            }

            let sheet_row = row_idx + 1; // 1-based row number
            let last_col = col_index_to_letter(updated_row.len().saturating_sub(1));
            let update_range = format!("{SHEET_STUDENTS}!A{sheet_row}:{last_col}{sheet_row}");
            let values: Vec<Vec<serde_json::Value>> = vec![updated_row
                .into_iter()
                .map(serde_json::Value::String)
                .collect()];
            self.update_values(&update_range, values).await?;
            return Ok(());
        }

        Err(anyhow::anyhow!("Student {} not found in sheet", telegram_name))
    }

    /// Updates the `chat_id` column for an existing student row.
    pub async fn update_student_chat_id(
        &self,
        telegram_name: &TelegramName,
        chat_id: i64,
    ) -> Result<()> {
        self.update_student_single_column(telegram_name, Student::CHAT_ID, chat_id.to_string()).await
    }

    /// Updates the `notification_delay` column for an existing student row.
    /// Pass `None` to clear the delay (disables notifications).
    pub async fn update_student_notification_delay(
        &self,
        telegram_name: &TelegramName,
        delay: Option<&Duration>,
    ) -> Result<()> {
        let value = delay.map(|d| d.to_string()).unwrap_or_default();
        self.update_student_single_column(telegram_name, Student::NOTIFICATION_DELAY, value).await
    }

    async fn update_student_single_column(
        &self,
        telegram_name: &TelegramName,
        col_name: &str,
        value: String,
    ) -> Result<()> {
        let range = format!("{SHEET_STUDENTS}!A:Z");
        let rows = self
            .get_values(&range)
            .await
            .context("Failed to get Students sheet data")?;

        if rows.is_empty() {
            return Err(anyhow::anyhow!("Students sheet is empty"));
        }

        let schema = SheetSchema::new(SHEET_STUDENTS.to_string(), rows[0].clone());

        for (row_idx, row) in rows.iter().enumerate().skip(1) {
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }
            let row_name = TelegramName::try_from(schema.get_str(row, Student::TELEGRAM_NAME)).ok();
            if row_name.as_ref() != Some(telegram_name) {
                continue;
            }

            let mut updated_row: Vec<String> = row.clone();
            let needed_len = schema.headers.len();
            if updated_row.len() < needed_len {
                updated_row.resize(needed_len, String::new());
            }

            if let Some(idx) = schema.get_col(col_name) {
                updated_row[idx] = value;
            }

            let sheet_row = row_idx + 1;
            let last_col = col_index_to_letter(updated_row.len().saturating_sub(1));
            let update_range = format!("{SHEET_STUDENTS}!A{sheet_row}:{last_col}{sheet_row}");
            let values: Vec<Vec<serde_json::Value>> = vec![updated_row
                .into_iter()
                .map(serde_json::Value::String)
                .collect()];
            self.update_values(&update_range, values).await?;
            return Ok(());
        }

        Err(anyhow::anyhow!("Student {} not found in sheet", telegram_name))
    }
}
