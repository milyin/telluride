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

            pairings.push(TeacherStudentPairing {
                teacher_telegram,
                student_telegram,
                cost,
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
