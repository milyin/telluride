use crate::models::{SheetParseError, Student, Teacher, TeacherStudentAssignment, TelegramName, UserRole};
use crate::sheets::SheetsClient;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use teloxide::prelude::*;
use teloxide::types::ChatId;
use tokio::sync::RwLock;

/// Minimum time between consecutive Drive API modification-time checks.
/// Within this window the cached data is assumed to still be valid.
const CHECK_INTERVAL: Duration = Duration::from_secs(15);

pub struct BotState {
    pub sheets: Arc<SheetsClient>,

    // --- Cached sheet data ---------------------------------------------------
    students: Arc<RwLock<HashMap<String, Student>>>,
    teachers: Arc<RwLock<HashMap<String, Teacher>>>,
    assignments: Arc<RwLock<Vec<TeacherStudentAssignment>>>,

    // --- Staleness tracking --------------------------------------------------
    /// The `modifiedTime` value returned by the Drive API the last time we
    /// successfully loaded data.  `None` means we have not yet confirmed the
    /// modification time (forces a load on the first check).
    last_modified: Mutex<Option<DateTime<Utc>>>,
    /// Instant of the most recent modification-time check (successful or not).
    /// Starts at `Instant::now() - CHECK_INTERVAL` so the very first command
    /// triggers an immediate check.
    last_checked: Mutex<Instant>,

    // --- Impersonation -------------------------------------------------------
    /// Maps a teacher's ChatId to the telegram name of the student they are
    /// currently impersonating.  Entries are inserted by `/impersonate` and
    /// removed by `/quit`.
    impersonations: RwLock<HashMap<ChatId, String>>,

    // --- Parse Error Reporting -----------------------------------------------
    /// Parse errors from the most recent reload.
    last_errors: RwLock<Vec<SheetParseError>>,

    /// The set of teacher TelegramNames who have ALREADY been sent the `last_errors`.
    /// This is cleared whenever `last_errors` is updated.
    notified_teachers: RwLock<HashSet<TelegramName>>,
}

impl BotState {
    pub fn new(sheets: Arc<SheetsClient>) -> Self {
        Self {
            sheets,
            students: Arc::new(RwLock::new(HashMap::new())),
            teachers: Arc::new(RwLock::new(HashMap::new())),
            assignments: Arc::new(RwLock::new(Vec::new())),
            last_modified: Mutex::new(None),
            // Subtract CHECK_INTERVAL so the very first command fires a check.
            last_checked: Mutex::new(Instant::now() - CHECK_INTERVAL),
            impersonations: RwLock::new(HashMap::new()),
            last_errors: RwLock::new(Vec::new()),
            notified_teachers: RwLock::new(HashSet::new()),
        }
    }

    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Unconditionally re-reads all tables from Google Sheets.
    ///
    /// Returns any [`SheetParseError`]s encountered during parsing.
    /// Called once at startup; use [`refresh_if_needed`] during normal
    /// operation to benefit from caching.
    pub async fn refresh(&self) -> Result<Vec<SheetParseError>> {
        self.do_reload().await
    }

    /// Checks whether the spreadsheet has been modified since the last load
    /// and, if so, reloads the cached data.
    ///
    /// The Drive API is queried **at most once per [`CHECK_INTERVAL`]**
    /// (currently 15 s) to avoid excessive API usage.  If the Drive API call
    /// fails, a warning is logged and the existing cache is kept — the bot
    /// continues serving stale-but-valid data rather than crashing.
    ///
    /// Returns `true` if data was actually reloaded.
    pub async fn refresh_if_needed(&self) -> bool {
        // --- Throttle: bail out if we checked recently ----------------------
        {
            let last_checked = self.last_checked.lock().unwrap();
            if last_checked.elapsed() < CHECK_INTERVAL {
                return false;
            }
        }

        // --- Hit the Drive API to get the current modification time ---------
        let current_modified = match self.sheets.get_spreadsheet_modified_time().await {
            Ok(t) => t,
            Err(e) => {
                log::warn!(
                    "Could not check spreadsheet modification time (using cached data): {}",
                    e
                );
                // Update last_checked so we don't hammer the API on every command
                // even when it keeps failing.
                *self.last_checked.lock().unwrap() = Instant::now();
                return false;
            }
        };

        // Record the successful check time.
        *self.last_checked.lock().unwrap() = Instant::now();

        // --- Compare with what we loaded last time --------------------------
        let needs_reload = {
            let last_modified = self.last_modified.lock().unwrap();
            *last_modified != Some(current_modified)
        };

        if !needs_reload {
            log::debug!("Spreadsheet unchanged (modifiedTime = {current_modified}).");
            return false;
        }

        // --- Reload ---------------------------------------------------------
        log::info!(
            "Spreadsheet modified (new modifiedTime = {}), reloading data…",
            current_modified
        );
        match self.do_reload().await {
            Ok(_) => true,
            Err(e) => {
                log::error!("Failed to reload data from spreadsheet: {}", e);
                false
            }
        }
    }

    /// Sends the most recent parse errors to the given teacher if they haven't
    /// already received them for the current spreadsheet version.
    pub async fn try_send_errors_to_teacher(
        &self,
        bot: &Bot,
        chat_id: ChatId,
        telegram_name: &TelegramName,
    ) {
        let errors = self.last_errors.read().await;
        if errors.is_empty() {
            return;
        }

        let mut notified = self.notified_teachers.write().await;
        if notified.contains(telegram_name) {
            return;
        }

        // Send the errors
        Self::send_error_report(bot, chat_id, &errors).await;

        // Mark as notified
        notified.insert(telegram_name.clone());
    }

    /// Looks up a user by Telegram name (normalised via [`TelegramName`]).
    ///
    /// If a user is both a teacher and a student, they are treated as a teacher.
    pub async fn get_role(&self, telegram_name: &str) -> Option<UserRole> {
        let normalised = TelegramName::try_from(telegram_name).ok()?;
        let key = normalised.as_str();

        {
            let teachers = self.teachers.read().await;
            if let Some(teacher) = teachers.get(key) {
                return Some(UserRole::Teacher(teacher.clone()));
            }
        }

        {
            let students = self.students.read().await;
            if let Some(student) = students.get(key) {
                return Some(UserRole::Student(student.clone()));
            }
        }

        None
    }

    /// Checks if a user is both a teacher and a student.
    /// Both must be present in the respective tables.
    pub async fn is_both_teacher_and_student(&self, telegram_name: &str) -> bool {
        let Ok(normalised) = TelegramName::try_from(telegram_name) else {
            return false;
        };
        let key = normalised.as_str();

        let is_teacher = {
            let teachers = self.teachers.read().await;
            teachers.contains_key(key)
        };

        let is_student = {
            let students = self.students.read().await;
            students.contains_key(key)
        };

        is_teacher && is_student
    }

    /// Returns a sorted list of all registered student telegram names (without '@', lowercase).
    pub async fn get_student_names(&self) -> Vec<String> {
        let students = self.students.read().await;
        let mut names: Vec<String> = students.keys().cloned().collect();
        names.sort();
        names
    }

    /// Gets a student by telegram name directly, bypassing the role-based lookup.
    /// This is useful for impersonating dual-role users.
    /// Returns the student if they are registered, regardless of whether they're also a teacher.
    pub async fn get_student(&self, telegram_name: &str) -> Option<crate::models::Student> {
        let normalised = TelegramName::try_from(telegram_name).ok()?;
        let students = self.students.read().await;
        students.get(normalised.as_str()).cloned()
    }

    /// Returns all assignments where the given teacher is the teacher.
    pub async fn get_assignments_for_teacher(
        &self,
        telegram_name: &str,
    ) -> Vec<TeacherStudentAssignment> {
        let Ok(normalised) = TelegramName::try_from(telegram_name) else {
            return vec![];
        };
        let assignments = self.assignments.read().await;
        assignments
            .iter()
            .filter(|a| a.teacher_telegram == normalised)
            .cloned()
            .collect()
    }

    /// Returns all assignments where the given student is the student.
    pub async fn get_assignments_for_student(
        &self,
        telegram_name: &str,
    ) -> Vec<TeacherStudentAssignment> {
        let Ok(normalised) = TelegramName::try_from(telegram_name) else {
            return vec![];
        };
        let assignments = self.assignments.read().await;
        assignments
            .iter()
            .filter(|a| a.student_telegram == normalised)
            .cloned()
            .collect()
    }

    // -----------------------------------------------------------------------
    // Impersonation API
    // -----------------------------------------------------------------------

    /// Records that the teacher chatting in `chat_id` is now impersonating
    /// `student_name` (without '@', lowercase).
    pub async fn impersonate(&self, chat_id: ChatId, student_name: String) {
        self.impersonations
            .write()
            .await
            .insert(chat_id, student_name);
    }

    /// Returns the telegram name of the student being impersonated in this
    /// chat, or `None` if the chat is not in impersonation mode.
    pub async fn get_impersonation(&self, chat_id: ChatId) -> Option<String> {
        self.impersonations.read().await.get(&chat_id).cloned()
    }

    /// Removes the impersonation entry for `chat_id`, returning the teacher to
    /// normal mode.
    pub async fn clear_impersonation(&self, chat_id: ChatId) {
        self.impersonations.write().await.remove(&chat_id);
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Reads all four tables from Google Sheets and atomically replaces the caches.
    /// Also fetches and caches the current modification time.
    ///
    /// Returns all parse errors collected across all sheets.
    async fn do_reload(&self) -> Result<Vec<SheetParseError>> {
        let (students, mut errors) = self.sheets.get_students().await?;
        let (teachers, teacher_errors) = self.sheets.get_teachers().await?;
        let (assignments, assignment_errors) = self.sheets.get_assignments().await?;
        let modified_time = self.sheets.get_spreadsheet_modified_time().await?;

        errors.extend(teacher_errors);
        errors.extend(assignment_errors);

        // Also reload the schedule to surface its errors (the cached data itself
        // is read on demand, but we still want to report parse failures).
        let (_, schedule_errors) = self.sheets.get_schedule().await?;
        errors.extend(schedule_errors);

        let (ns, nt, na) = (students.len(), teachers.len(), assignments.len());
        *self.students.write().await = students;
        *self.teachers.write().await = teachers;
        *self.assignments.write().await = assignments;
        *self.last_errors.write().await = errors.clone();
        self.notified_teachers.write().await.clear();
        *self.last_modified.lock().unwrap() = Some(modified_time);

        if errors.is_empty() {
            log::info!("Data reloaded: {ns} students, {nt} teachers, {na} assignments.");
        } else {
            log::error!(
                "Data reloaded: {ns} students, {nt} teachers, {na} assignments \
                 — {} parse error(s).",
                errors.len()
            );
        }

        Ok(errors)
    }

    /// Sends the error report to `chat_id` without pinning it.
    ///
    /// All Telegram API errors are logged but do not propagate.
    async fn send_error_report(bot: &Bot, chat_id: ChatId, errors: &[SheetParseError]) {
        // Build the error message text.
        let header = format!(
            "⚠️ Spreadsheet parse errors ({} row(s) skipped):\n\n",
            errors.len()
        );
        let body: String = errors
            .iter()
            .map(|e| format!("• {}\n", e))
            .collect();
        let text = format!("{}{}", header, body);

        // Send the message.
        if let Err(e) = bot.send_message(chat_id, &text).await {
            log::error!("Failed to send parse-error report to chat {chat_id}: {e}");
        }
    }
}
