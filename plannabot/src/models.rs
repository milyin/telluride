use std::fmt;

use chrono::{DateTime, NaiveDate, NaiveTime, Utc, Weekday};
use chrono_tz::Tz;

pub use crate::types::{TelegramName, TimePeriod};

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
        pub currency: Option<&'static rusty_money::iso::Currency>,
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
/// `None` in the sheet field means the lesson is planned (future) or passed (past), depending on datetime.
#[derive(Debug, Clone, PartialEq)]
pub enum LessonStatus {
    Absent,
    Cancelled,
}

impl LessonStatus {
    /// Parse from a sheet string. Returns `None` for empty or unrecognised values (= auto planned/passed).
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "absent" => Some(LessonStatus::Absent),
            "cancelled" => Some(LessonStatus::Cancelled),
            _ => None,
        }
    }

    /// The string written to the sheet for this status.
    pub fn as_sheet_str(&self) -> &'static str {
        match self {
            LessonStatus::Absent => "absent",
            LessonStatus::Cancelled => "cancelled",
        }
    }
}

impl fmt::Display for LessonStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_sheet_str())
    }
}

impl std::str::FromStr for LessonStatus {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> anyhow::Result<Self> {
        LessonStatus::from_str(s)
            .ok_or_else(|| anyhow::anyhow!("unknown lesson status: '{}'", s))
    }
}

sheet_struct! {
    /// A single entry in the schedule.
    /// `status = None` means the lesson is planned.
    #[derive(Debug, Clone)]
    pub struct ScheduleEntry {
        pub student_telegram: TelegramName,
        pub teacher_telegram: TelegramName,
        pub datetime: DateTime<Utc>,
        pub duration_minutes: u64,
        pub cost: i64,
        pub status: Option<LessonStatus>,
    }
}

impl ScheduleEntry {
    /// Returns `true` if the lesson has no status set (= planned).
    pub fn is_planned(&self) -> bool {
        self.status.is_none()
    }

    pub fn time_period(&self) -> TimePeriod {
        TimePeriod::new(
            self.datetime,
            chrono::Duration::minutes(self.duration_minutes as i64),
        )
    }
}

sheet_struct! {
    /// A payment record for a student.
    #[derive(Debug, Clone)]
    pub struct Payment {
        pub student_telegram: TelegramName,
        pub date: DateTime<Utc>,
        pub sum: i64,
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
        pub duration_minutes: u64,
        pub zoom_url: Option<String>,
        pub board_url: Option<String>,
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

impl Worktime {
    pub fn time_period(&self, date: NaiveDate) -> TimePeriod {
        let start = date.and_time(self.start_time).and_utc();
        let end = date.and_time(self.end_time).and_utc();
        TimePeriod::new(start, end - start)
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
    /// Teacher currently impersonating a student.
    ImpersonateStudent(Teacher, TelegramName),
    /// Teacher (admin) currently impersonating another teacher.
    ImpersonateTeacher(Teacher, TelegramName),
}

impl UserEffectiveRole {
    pub fn name(&self) -> &str {
        match self {
            UserEffectiveRole::Student(s) => &s.name,
            UserEffectiveRole::Teacher(t) => &t.name,
            UserEffectiveRole::Admin(t) => &t.name,
            UserEffectiveRole::ImpersonateStudent(t, _) => &t.name,
            UserEffectiveRole::ImpersonateTeacher(t, _) => &t.name,
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
