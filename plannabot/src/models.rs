use std::collections::HashMap;

use chrono::{DateTime, Utc};

/// A student registered in the system.
/// `telegram_name` is stored without '@', in lowercase.
#[derive(Debug, Clone, PartialEq)]
pub struct Student {
    pub telegram_name: String,
    pub name: String,
    pub timezone: String,
    pub currency: String,
    pub zoom_url: Option<String>,
    pub board_url: Option<String>,
    pub custom: HashMap<String, String>,
}

/// A teacher registered in the system.
/// `telegram_name` is stored without '@', in lowercase.
#[derive(Debug, Clone, PartialEq)]
pub struct Teacher {
    pub telegram_name: String,
    pub timezone: String,
    pub custom: HashMap<String, String>,
}

/// Status of a scheduled lesson.
#[derive(Debug, Clone, PartialEq)]
pub enum LessonStatus {
    Done,
    Cancelled,
}

impl LessonStatus {
    /// Parse from a string. Returns None for empty or unrecognized values (treated as "planned").
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "done" | "completed" => Some(LessonStatus::Done),
            "cancelled" | "canceled" => Some(LessonStatus::Cancelled),
            _ => None,
        }
    }
}

/// A single entry in the schedule.
/// `student_telegram` and `teacher_telegram` are stored without '@', in lowercase.
/// `status = None` means the lesson is planned (not yet done or cancelled).
#[derive(Debug, Clone)]
pub struct ScheduleEntry {
    pub student_telegram: String,
    pub teacher_telegram: String,
    pub datetime: DateTime<Utc>,
    pub duration_minutes: i64,
    pub cost: f64,
    pub status: Option<LessonStatus>,
    pub custom: HashMap<String, String>,
}

impl ScheduleEntry {
    /// Returns true if this lesson is planned (not done or cancelled).
    pub fn is_planned(&self) -> bool {
        self.status.is_none()
    }
}

/// A payment record for a student.
/// `student_telegram` is stored without '@', in lowercase.
#[derive(Debug, Clone)]
pub struct Payment {
    pub student_telegram: String,
    pub date: DateTime<Utc>,
    pub sum: f64,
    pub custom: HashMap<String, String>,
}

/// The role of a Telegram user in the system.
#[derive(Debug, Clone)]
pub enum UserRole {
    Student(Student),
    Teacher(Teacher),
}
