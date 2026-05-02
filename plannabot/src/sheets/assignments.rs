//! Teacher ↔ student assignment data reader for the `Assignments` sheet tab.

use anyhow::{Context, Result};

use super::{SheetSchema, SheetsClient, SHEET_ASSIGNMENTS};
use crate::models::TeacherStudentAssignment;

impl SheetsClient {
    /// Reads all rows from the `Assignments` sheet and returns them as a
    /// `Vec<TeacherStudentAssignment>`.
    ///
    /// Normalisation: `'@'` prefix stripped, then lowercased.
    /// Rows where either `teacher_telegram` or `student_telegram` is empty are
    /// silently skipped.
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

            let teacher_telegram = schema
                .get_str(row, "teacher_telegram")
                .trim_start_matches('@')
                .to_lowercase();

            let student_telegram = schema
                .get_str(row, "student_telegram")
                .trim_start_matches('@')
                .to_lowercase();

            // Both fields are required.
            if teacher_telegram.is_empty() || student_telegram.is_empty() {
                continue;
            }

            assignments.push(TeacherStudentAssignment {
                teacher_telegram,
                student_telegram,
            });
        }

        Ok(assignments)
    }

    /// Returns all students assigned to the given teacher.
    ///
    /// `telegram_name` is normalised before filtering.
    pub async fn get_students_for_teacher(
        &self,
        telegram_name: &str,
    ) -> Result<Vec<TeacherStudentAssignment>> {
        let normalised = telegram_name.trim_start_matches('@').to_lowercase();
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
    /// `telegram_name` is normalised before filtering.
    pub async fn get_teachers_for_student(
        &self,
        telegram_name: &str,
    ) -> Result<Vec<TeacherStudentAssignment>> {
        let normalised = telegram_name.trim_start_matches('@').to_lowercase();
        let all = self.get_assignments().await?;
        Ok(all
            .into_iter()
            .filter(|a| a.student_telegram == normalised)
            .collect())
    }
}
