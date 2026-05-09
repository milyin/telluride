//! Teacher ↔ student pairing data reader for the `Pairings` sheet tab.

use anyhow::{Context, Result};

use super::{SHEET_PAIRINGS, SheetSchema, SheetsClient};
use crate::models::{SheetParseError, TeacherStudentPairing, TelegramName};

impl SheetsClient {
    /// Reads all rows from the `Pairings` sheet and returns them as a
    /// `Vec<TeacherStudentPairing>`, together with any [`SheetParseError`]s
    /// encountered while parsing rows.
    pub async fn get_pairings(&self) -> Result<(Vec<TeacherStudentPairing>, Vec<SheetParseError>)> {
        let range = format!("{SHEET_PAIRINGS}!A:Z");
        let rows = self
            .get_values(&range)
            .await
            .context("Failed to get Pairings sheet data")?;

        if rows.is_empty() {
            return Ok((vec![], vec![]));
        }

        let schema = SheetSchema::new(SHEET_PAIRINGS.to_string(), rows[0].clone());

        let mut pairings: Vec<TeacherStudentPairing> = Vec::new();
        let mut errors: Vec<SheetParseError> = Vec::new();

        for (row_idx, row) in rows.iter().enumerate().skip(1) {
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }

            let row_num = row_idx + 1;

            let Some(teacher_telegram) = schema.get_field::<Option<TelegramName>>(
                row,
                row_num,
                TeacherStudentPairing::TEACHER_TELEGRAM,
                &mut errors,
            ) else {
                continue;
            };
            let Some(student_telegram) = schema.get_field::<Option<TelegramName>>(
                row,
                row_num,
                TeacherStudentPairing::STUDENT_TELEGRAM,
                &mut errors,
            ) else {
                continue;
            };
            let cost =
                schema.get_field::<i64>(row, row_num, TeacherStudentPairing::COST, &mut errors);
            let duration_minutes: u64 = schema
                .get_str(row, TeacherStudentPairing::DURATION_MINUTES)
                .parse()
                .unwrap_or(60);

            pairings.push(TeacherStudentPairing {
                teacher_telegram,
                student_telegram,
                cost,
                duration_minutes,
                custom: schema.get_custom(row, super::PAIRINGS_COLS),
            });
        }

        Ok((pairings, errors))
    }

    /// Returns all students assigned to the given teacher.
    pub async fn get_students_for_teacher(
        &self,
        telegram_name: &str,
    ) -> Result<Vec<TeacherStudentPairing>> {
        let Ok(normalised) = TelegramName::try_from(telegram_name) else {
            return Ok(vec![]);
        };
        let (all, _) = self.get_pairings().await?;
        Ok(all
            .into_iter()
            .filter(|a| a.teacher_telegram == normalised)
            .collect())
    }

    /// Appends a new pairing row to the `Pairings` sheet.
    pub async fn add_pairing(
        &self,
        teacher: &TelegramName,
        student: &TelegramName,
        duration_minutes: u64,
        cost: i64,
    ) -> Result<()> {
        let header_range = format!("{SHEET_PAIRINGS}!1:1");
        let rows = self
            .get_values(&header_range)
            .await
            .context("Failed to read Pairings header row")?;
        let headers = rows.into_iter().next().unwrap_or_default();

        let mut values: std::collections::HashMap<&str, String> = std::collections::HashMap::new();
        values.insert(TeacherStudentPairing::TEACHER_TELEGRAM, teacher.as_str().to_string());
        values.insert(TeacherStudentPairing::STUDENT_TELEGRAM, student.as_str().to_string());
        values.insert(TeacherStudentPairing::COST, cost.to_string());
        values.insert(TeacherStudentPairing::DURATION_MINUTES, duration_minutes.to_string());

        let row: Vec<serde_json::Value> = headers
            .iter()
            .map(|h| serde_json::Value::String(values.get(h.as_str()).cloned().unwrap_or_default()))
            .collect();

        self.append_row(SHEET_PAIRINGS, row).await
    }

    /// Updates the `cost` and `duration_minutes` of an existing pairing row.
    pub async fn update_pairing(
        &self,
        teacher: &TelegramName,
        student: &TelegramName,
        duration_minutes: u64,
        cost: i64,
    ) -> Result<()> {
        let range = format!("{SHEET_PAIRINGS}!A:Z");
        let rows = self
            .get_values(&range)
            .await
            .context("Failed to get Pairings sheet data")?;

        if rows.is_empty() {
            return Err(anyhow::anyhow!("Pairings sheet is empty"));
        }

        let schema = SheetSchema::new(SHEET_PAIRINGS.to_string(), rows[0].clone());

        for (row_idx, row) in rows.iter().enumerate().skip(1) {
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }
            let row_teacher = TelegramName::try_from(schema.get_str(row, TeacherStudentPairing::TEACHER_TELEGRAM)).ok();
            let row_student = TelegramName::try_from(schema.get_str(row, TeacherStudentPairing::STUDENT_TELEGRAM)).ok();
            if row_teacher.as_ref() != Some(teacher) || row_student.as_ref() != Some(student) {
                continue;
            }

            let mut updated_row: Vec<String> = row.clone();
            let needed_len = schema.headers.len();
            if updated_row.len() < needed_len {
                updated_row.resize(needed_len, String::new());
            }

            if let Some(idx) = schema.get_col(TeacherStudentPairing::COST) {
                updated_row[idx] = cost.to_string();
            }
            if let Some(idx) = schema.get_col(TeacherStudentPairing::DURATION_MINUTES) {
                updated_row[idx] = duration_minutes.to_string();
            }

            let sheet_row = row_idx + 1;
            let last_col = super::col_index_to_letter(updated_row.len().saturating_sub(1));
            let update_range = format!("{SHEET_PAIRINGS}!A{sheet_row}:{last_col}{sheet_row}");
            let values: Vec<Vec<serde_json::Value>> = vec![updated_row
                .into_iter()
                .map(serde_json::Value::String)
                .collect()];
            self.update_values(&update_range, values).await?;
            return Ok(());
        }

        Err(anyhow::anyhow!("Pairing {teacher} ↔ {student} not found in sheet"))
    }

    /// Deletes the pairing row for the given teacher ↔ student pair.
    pub async fn delete_pairing(
        &self,
        teacher: &TelegramName,
        student: &TelegramName,
    ) -> Result<()> {
        let range = format!("{SHEET_PAIRINGS}!A:Z");
        let rows = self
            .get_values(&range)
            .await
            .context("Failed to get Pairings sheet data")?;

        if rows.is_empty() {
            return Err(anyhow::anyhow!("Pairings sheet is empty"));
        }

        let schema = SheetSchema::new(SHEET_PAIRINGS.to_string(), rows[0].clone());

        for (row_idx, row) in rows.iter().enumerate().skip(1) {
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }
            let row_teacher = TelegramName::try_from(schema.get_str(row, TeacherStudentPairing::TEACHER_TELEGRAM)).ok();
            let row_student = TelegramName::try_from(schema.get_str(row, TeacherStudentPairing::STUDENT_TELEGRAM)).ok();
            if row_teacher.as_ref() == Some(teacher) && row_student.as_ref() == Some(student) {
                self.delete_row(SHEET_PAIRINGS, row_idx).await?;
                return Ok(());
            }
        }

        Err(anyhow::anyhow!("Pairing {teacher} ↔ {student} not found in sheet"))
    }

    /// Returns the teacher(s) assigned to the given student, if any.
    pub async fn get_teachers_for_student(
        &self,
        telegram_name: &str,
    ) -> Result<Vec<TeacherStudentPairing>> {
        let Ok(normalised) = TelegramName::try_from(telegram_name) else {
            return Ok(vec![]);
        };
        let (all, _) = self.get_pairings().await?;
        Ok(all
            .into_iter()
            .filter(|a| a.student_telegram == normalised)
            .collect())
    }
}
