//! Teacher ↔ student assignment data reader for the `Assignments` sheet tab.

use anyhow::{Context, Result};

use super::{SheetSchema, SheetsClient, SHEET_ASSIGNMENTS};
use crate::models::{TeacherStudentAssignment, TelegramName};

impl SheetsClient {
    /// Reads all rows from the `Assignments` sheet and returns them as a
    /// `Vec<TeacherStudentAssignment>`.
    ///
    /// Rows where either `teacher_telegram` or `student_telegram` is empty or
    /// invalid are silently skipped.
    pub async fn get_assignments(&self) -> Result<Vec<TeacherStudentAssignment>> {
        let range = format!("{SHEET_ASSIGNMENTS}!A:Z");
        let rows = self
            .get_values(&range)
            .await
            .context("Failed to get Assignments sheet data")?;

        if rows.is_empty() {
            return Ok(vec![]);
        }

        // First row is always the header.
        let schema = SheetSchema::new(SHEET_ASSIGNMENTS.to_string(), rows[0].clone());

        let mut assignments: Vec<TeacherStudentAssignment> = Vec::new();

        for row in rows.iter().skip(1) {
            // Skip empty rows.
            if row.is_empty() || row.iter().all(|c| c.is_empty()) {
                continue;
            }

            let raw_teacher = schema.get_str(row, "teacher_telegram");
            let teacher_telegram = match TelegramName::try_from(raw_teacher) {
                Ok(n) => n,
                Err(e) => {
                    if !raw_teacher.trim().is_empty() {
                        log::warn!("Assignments sheet — invalid teacher_telegram {:?}: {}", raw_teacher, e);
                    }
                    continue;
                }
            };

            let raw_student = schema.get_str(row, "student_telegram");
            let student_telegram = match TelegramName::try_from(raw_student) {
                Ok(n) => n,
                Err(e) => {
                    if !raw_student.trim().is_empty() {
                        log::warn!("Assignments sheet — invalid student_telegram {:?}: {}", raw_student, e);
                    }
                    continue;
                }
            };

            assignments.push(TeacherStudentAssignment {
                teacher_telegram,
                student_telegram,
            });
        }

        Ok(assignments)
    }

    /// Returns all students assigned to the given teacher.
    ///
    /// `telegram_name` is normalised and validated; an empty list is returned
    /// for invalid names.
    pub async fn get_students_for_teacher(
        &self,
        telegram_name: &str,
    ) -> Result<Vec<TeacherStudentAssignment>> {
        let Ok(normalised) = TelegramName::try_from(telegram_name) else {
            return Ok(vec![]);
        };
        let all = self.get_assignments().await?;
        Ok(all
            .into_iter()
            .filter(|a| a.teacher_telegram == normalised)
            .collect())
    }

    /// Returns the teacher assigned to the given student, if any.
    ///
    /// When multiple rows for the same student exist (misconfigured sheet),
    /// all matching rows are returned so the caller can decide how to handle it.
    ///
    /// `telegram_name` is normalised and validated; an empty list is returned
    /// for invalid names.
    pub async fn get_teachers_for_student(
        &self,
        telegram_name: &str,
    ) -> Result<Vec<TeacherStudentAssignment>> {
        let Ok(normalised) = TelegramName::try_from(telegram_name) else {
            return Ok(vec![]);
        };
        let all = self.get_assignments().await?;
        Ok(all
            .into_iter()
            .filter(|a| a.student_telegram == normalised)
            .collect())
    }
}
