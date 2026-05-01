use crate::models::{Student, Teacher, UserRole};
use crate::sheets::SheetsClient;
use anyhow::Result;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

pub struct BotState {
    pub sheets: Arc<SheetsClient>,
    students: Arc<RwLock<HashMap<String, Student>>>,
    teachers: Arc<RwLock<HashMap<String, Teacher>>>,
}

impl BotState {
    pub fn new(sheets: Arc<SheetsClient>) -> Self {
        Self {
            sheets,
            students: Arc::new(RwLock::new(HashMap::new())),
            teachers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Re-reads students and teachers from Google Sheets and caches them.
    pub async fn refresh(&self) -> Result<()> {
        let students = self.sheets.get_students().await?;
        let teachers = self.sheets.get_teachers().await?;
        let student_count = students.len();
        let teacher_count = teachers.len();
        *self.students.write().await = students;
        *self.teachers.write().await = teachers;
        log::info!(
            "Refreshed: {} students, {} teachers",
            student_count,
            teacher_count
        );
        Ok(())
    }

    /// Looks up a user by Telegram name (normalized: no '@', lowercase).
    /// Checks students first, then teachers.
    pub async fn get_role(&self, telegram_name: &str) -> Option<UserRole> {
        let normalized = telegram_name.trim_start_matches('@').to_lowercase();

        {
            let students = self.students.read().await;
            if let Some(student) = students.get(&normalized) {
                return Some(UserRole::Student(student.clone()));
            }
        }

        {
            let teachers = self.teachers.read().await;
            if let Some(teacher) = teachers.get(&normalized) {
                return Some(UserRole::Teacher(teacher.clone()));
            }
        }

        None
    }
}
