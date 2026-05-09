//! Teacher data reader and writer for the `Teachers` sheet tab.

use std::collections::HashMap;

use anyhow::{Context, Result};
use chrono_tz::Tz;

use super::{SHEET_TEACHERS, SheetSchema, SheetsClient, TEACHERS_COLS, col_index_to_letter};
use crate::models::{SheetParseError, Teacher, TelegramName};

impl SheetsClient {
    /// Reads all rows from the `Teachers` sheet and returns a map from
    /// (normalised) Telegram username → [`Teacher`], together with any
    /// [`SheetParseError`]s encountered while parsing rows.
    pub async fn get_teachers(&self) -> Result<(HashMap<String, Teacher>, Vec<SheetParseError>)> {
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

            let Some(telegram_name) = schema.get_field::<Option<TelegramName>>(
                row,
                row_num,
                Teacher::TELEGRAM_NAME,
                &mut errors,
            ) else {
                continue;
            };

            let teacher = Teacher {
                timezone: schema.get_field::<Tz>(row, row_num, Teacher::TIMEZONE, &mut errors),
                admin: schema.get_field::<bool>(row, row_num, Teacher::ADMIN, &mut errors),
                chat_id: schema.get_field::<i64>(row, row_num, Teacher::CHAT_ID, &mut errors),
                custom: schema.get_custom(row, TEACHERS_COLS),
                telegram_name: telegram_name.clone(),
                name: schema.get_field::<String>(row, row_num, Teacher::NAME, &mut errors),
            };

            teachers.insert(telegram_name.as_str().to_string(), teacher);
        }

        Ok((teachers, errors))
    }

    /// Appends a new teacher row to the `Teachers` sheet.
    pub async fn add_teacher(
        &self,
        telegram_name: &TelegramName,
        tz: &str,
        name: &str,
    ) -> Result<()> {
        let header_range = format!("{SHEET_TEACHERS}!1:1");
        let rows = self
            .get_values(&header_range)
            .await
            .context("Failed to read Teachers header row")?;
        let headers = rows.into_iter().next().unwrap_or_default();

        let mut values: HashMap<&str, String> = HashMap::new();
        values.insert(Teacher::TELEGRAM_NAME, telegram_name.as_str().to_string());
        values.insert(Teacher::NAME, name.to_string());
        values.insert(Teacher::TIMEZONE, tz.to_string());
        // Teacher::ADMIN defaults to empty string = false

        let row: Vec<serde_json::Value> = headers
            .iter()
            .map(|h| serde_json::Value::String(values.get(h.as_str()).cloned().unwrap_or_default()))
            .collect();

        self.append_row(SHEET_TEACHERS, row).await
    }

    /// Deletes the teacher row identified by `telegram_name`.
    pub async fn delete_teacher(&self, telegram_name: &TelegramName) -> Result<()> {
        let range = format!("{SHEET_TEACHERS}!A:Z");
        let rows = self
            .get_values(&range)
            .await
            .context("Failed to get Teachers sheet data")?;

        if rows.is_empty() {
            return Err(anyhow::anyhow!("Teachers sheet is empty"));
        }

        let schema = SheetSchema::new(SHEET_TEACHERS.to_string(), rows[0].clone());

        for (row_idx, row) in rows.iter().enumerate().skip(1) {
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }
            let row_name = TelegramName::try_from(schema.get_str(row, Teacher::TELEGRAM_NAME)).ok();
            if row_name.as_ref() == Some(telegram_name) {
                self.delete_row(SHEET_TEACHERS, row_idx).await?;
                return Ok(());
            }
        }

        Err(anyhow::anyhow!("Teacher {} not found in sheet", telegram_name))
    }

    /// Updates the timezone and name fields of an existing teacher row.
    pub async fn update_teacher(
        &self,
        telegram_name: &TelegramName,
        tz: &str,
        name: &str,
    ) -> Result<()> {
        let range = format!("{SHEET_TEACHERS}!A:Z");
        let rows = self
            .get_values(&range)
            .await
            .context("Failed to get Teachers sheet data")?;

        if rows.is_empty() {
            return Err(anyhow::anyhow!("Teachers sheet is empty"));
        }

        let schema = SheetSchema::new(SHEET_TEACHERS.to_string(), rows[0].clone());

        for (row_idx, row) in rows.iter().enumerate().skip(1) {
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }
            let row_name = TelegramName::try_from(schema.get_str(row, Teacher::TELEGRAM_NAME)).ok();
            if row_name.as_ref() != Some(telegram_name) {
                continue;
            }

            let mut updated_row: Vec<String> = row.clone();
            let needed_len = schema.headers.len();
            if updated_row.len() < needed_len {
                updated_row.resize(needed_len, String::new());
            }

            if let Some(idx) = schema.get_col(Teacher::TIMEZONE) {
                updated_row[idx] = tz.to_string();
            }
            if let Some(idx) = schema.get_col(Teacher::NAME) {
                updated_row[idx] = name.to_string();
            }

            let sheet_row = row_idx + 1;
            let last_col = col_index_to_letter(updated_row.len().saturating_sub(1));
            let update_range = format!("{SHEET_TEACHERS}!A{sheet_row}:{last_col}{sheet_row}");
            let values: Vec<Vec<serde_json::Value>> = vec![updated_row
                .into_iter()
                .map(serde_json::Value::String)
                .collect()];
            self.update_values(&update_range, values).await?;
            return Ok(());
        }

        Err(anyhow::anyhow!("Teacher {} not found in sheet", telegram_name))
    }

    /// Updates the `chat_id` column for an existing teacher row.
    pub async fn update_teacher_chat_id(
        &self,
        telegram_name: &TelegramName,
        chat_id: i64,
    ) -> Result<()> {
        let range = format!("{SHEET_TEACHERS}!A:Z");
        let rows = self
            .get_values(&range)
            .await
            .context("Failed to get Teachers sheet data")?;

        if rows.is_empty() {
            return Err(anyhow::anyhow!("Teachers sheet is empty"));
        }

        let schema = SheetSchema::new(SHEET_TEACHERS.to_string(), rows[0].clone());

        for (row_idx, row) in rows.iter().enumerate().skip(1) {
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }
            let row_name = TelegramName::try_from(schema.get_str(row, Teacher::TELEGRAM_NAME)).ok();
            if row_name.as_ref() != Some(telegram_name) {
                continue;
            }

            let mut updated_row: Vec<String> = row.clone();
            let needed_len = schema.headers.len();
            if updated_row.len() < needed_len {
                updated_row.resize(needed_len, String::new());
            }

            if let Some(idx) = schema.get_col(Teacher::CHAT_ID) {
                updated_row[idx] = chat_id.to_string();
            }

            let sheet_row = row_idx + 1;
            let last_col = col_index_to_letter(updated_row.len().saturating_sub(1));
            let update_range = format!("{SHEET_TEACHERS}!A{sheet_row}:{last_col}{sheet_row}");
            let values: Vec<Vec<serde_json::Value>> = vec![updated_row
                .into_iter()
                .map(serde_json::Value::String)
                .collect()];
            self.update_values(&update_range, values).await?;
            return Ok(());
        }

        Err(anyhow::anyhow!("Teacher {} not found in sheet", telegram_name))
    }
}
