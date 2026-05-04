use std::{fmt, str::FromStr};

use anyhow::bail;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc, Weekday};
use chrono_tz::Tz;
use std::convert::TryFrom;

/// A normalised Telegram username.
///
/// Stored without `'@'`, in lowercase.  Constructed via [`TryFrom<&str>`],
/// which strips a leading `'@'`, lower-cases the result, and then enforces
/// the following Telegram username rules:
///
/// * 5–32 characters long.
/// * Only ASCII letters (`a-z`), digits (`0-9`), and underscores (`_`).
/// * Must start with a letter (not a digit or underscore).
/// * Cannot end with an underscore.
/// * No consecutive underscores.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TelegramName(String);

impl FromStr for TelegramName {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        // Normalise: strip leading '@' and lowercase.
        let s = raw.trim_start_matches('@').to_lowercase();

        // --- Length ---
        if s.len() < 5 || s.len() > 32 {
            bail!(
                "Telegram username {:?} must be 5–32 characters long (got {})",
                s,
                s.len()
            );
        }

        // --- Character set ---
        if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            bail!(
                "Telegram username {:?} contains invalid characters \
                 (only a-z, 0-9, _ allowed)",
                s
            );
        }

        // --- Must start with a letter ---
        let first = s.chars().next().unwrap();
        if !first.is_ascii_alphabetic() {
            bail!(
                "Telegram username {:?} must start with a letter, not {:?}",
                s,
                first
            );
        }

        // --- Must not end with an underscore ---
        if s.ends_with('_') {
            bail!("Telegram username {:?} must not end with an underscore", s);
        }

        // --- No consecutive underscores ---
        if s.contains("__") {
            bail!(
                "Telegram username {:?} must not contain consecutive underscores",
                s
            );
        }

        Ok(TelegramName(s))
    }
}

impl fmt::Display for TelegramName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.0)
    }
}

impl TryFrom<&str> for TelegramName {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl TelegramName {
    /// Returns the inner username string (without `'@'`, in lowercase).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

macro_rules! sheet_struct {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $(
                $(#[$field_meta:meta])*
                $field_vis:vis $field:ident: $ty:ty
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis struct $name {
            $(
                $(#[$field_meta])*
                $field_vis $field: $ty,
            )*
            pub custom: std::collections::HashMap<String, String>,
        }

        impl $name {
            pub const SHEET_COLS: &'static [&'static str] = &[
                $(
                    stringify!($field),
                )*
            ];

            paste::paste! {
                $(
                    pub const [<$field:upper>]: &'static str = stringify!($field);
                )*
            }
        }
    };
}

sheet_struct! {
    /// A student registered in the system.
    #[derive(Debug, Clone, PartialEq)]
    pub struct Student {
        pub telegram_name: TelegramName,
        pub name: String,
        pub timezone: Tz,
        pub currency: String,
        pub zoom_url: Option<String>,
        pub board_url: Option<String>,
    }
}

sheet_struct! {
    /// A teacher registered in the system.
    #[derive(Debug, Clone, PartialEq)]
    pub struct Teacher {
        pub telegram_name: TelegramName,
        pub name: String,
        pub timezone: Tz,
        pub admin: bool,
    }
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

sheet_struct! {
    /// A single entry in the schedule.
    /// `status = None` means the lesson is planned (not yet done or cancelled).
    #[derive(Debug, Clone)]
    pub struct ScheduleEntry {
        pub student_telegram: TelegramName,
        pub teacher_telegram: TelegramName,
        pub datetime: DateTime<Utc>,
        pub duration_minutes: i64,
        pub cost: f64,
        pub status: Option<LessonStatus>,
    }
}

impl ScheduleEntry {
    /// Returns true if this lesson is planned (not done or cancelled).
    pub fn is_planned(&self) -> bool {
        self.status.is_none()
    }
}

sheet_struct! {
    /// A payment record for a student.
    #[derive(Debug, Clone)]
    pub struct Payment {
        pub student_telegram: TelegramName,
        pub date: DateTime<Utc>,
        pub sum: f64,
    }
}

sheet_struct! {
    /// A teacher ↔ student pairing row.
    /// Multiple students can be assigned to a single teacher.
    #[derive(Debug, Clone, PartialEq)]
    pub struct TeacherStudentPairing {
        pub teacher_telegram: TelegramName,
        pub student_telegram: TelegramName,
        pub cost: i64,
    }
}

sheet_struct! {
    /// A teacher's working time window for a specific day-of-week or calendar date.
    ///
    /// Rows with `date` set override rows with `day_of_week` for that specific date.
    #[derive(Debug, Clone)]
    pub struct Worktime {
        pub teacher_telegram: TelegramName,
        pub day_of_week: Option<Weekday>,
        pub date: Option<NaiveDate>,
        pub start_time: NaiveTime,
        pub end_time: NaiveTime,
    }
}

/// Persistent DB identity of a Telegram user. Only ever `Student` or `Teacher`.
/// Returned by [`BotState::get_role`].
#[derive(Debug, Clone)]
pub enum UserRole {
    Student(Student),
    Teacher(Teacher),
}

impl UserRole {
    pub fn name(&self) -> &str {
        match self {
            UserRole::Student(s) => &s.name,
            UserRole::Teacher(t) => &t.name,
        }
    }
}

/// Session-aware effective role of a Telegram user.
/// Returned by [`BotState::get_effective_role`].
#[derive(Debug, Clone)]
pub enum UserEffectiveRole {
    Student(Student),
    Teacher(Teacher),
    /// Teacher currently in admin mode.
    Admin(Teacher),
    /// Teacher currently impersonating a student (telegram name without '@').
    Impersonate(Teacher, String),
}

impl UserEffectiveRole {
    pub fn name(&self) -> &str {
        match self {
            UserEffectiveRole::Student(s) => &s.name,
            UserEffectiveRole::Teacher(t) => &t.name,
            UserEffectiveRole::Admin(t) => &t.name,
            UserEffectiveRole::Impersonate(t, _) => &t.name,
        }
    }
}

impl From<UserRole> for UserEffectiveRole {
    fn from(role: UserRole) -> Self {
        match role {
            UserRole::Student(s) => UserEffectiveRole::Student(s),
            UserRole::Teacher(t) => UserEffectiveRole::Teacher(t),
        }
    }
}

/// A parse error encountered while reading a row from a Google Sheet.
///
/// Collected during table refresh and reported to teachers as a
/// Telegram message.  All indices are **1-based** (matching how Google Sheets
/// labels rows to users).
#[derive(Debug, Clone)]
pub struct SheetParseError {
    /// Name of the sheet tab (e.g. `"Schedule"`).
    pub sheet: String,
    /// 1-based row number where the error occurred (row 1 = header).
    pub row: usize,
    /// Column / field name that could not be parsed (e.g. `"student_telegram"`).
    pub column: String,
    /// Human-readable description of what went wrong.
    pub message: String,
}

impl fmt::Display for SheetParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Sheet '{}', row {}, column '{}': {}",
            self.sheet, self.row, self.column, self.message
        )
    }
}
