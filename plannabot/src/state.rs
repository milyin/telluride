use crate::models::{SheetParseError, Student, Teacher, TeacherStudentAssignment, TelegramName, UserRole};
use crate::sheets::SheetsClient;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::{
    collections::HashMap,
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

    // --- Teacher chat registration -------------------------------------------
    /// Maps a teacher's normalised TelegramName to their ChatId.
    ///
    /// Populated whenever a known teacher sends any message.  Used to send and
    /// pin parse-error reports after a spreadsheet refresh.  Teachers who have
    /// never messaged the bot are deferred: once they do appear their pending
    /// errors are sent at that point.
    teacher_chat_ids: RwLock<HashMap<TelegramName, ChatId>>,

    /// Parse errors collected during the most recent reload that have not yet
    /// been sent to all teachers.  Keyed by teacher TelegramName so that when
    /// a previously-unknown teacher appears we can deliver their report.
    ///
    /// Errors are cleared once every teacher has been notified (or the next
    /// reload overwrites them).
    pending_errors: RwLock<Vec<SheetParseError>>,
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
            teacher_chat_ids: RwLock::new(HashMap::new()),
            pending_errors: RwLock::new(Vec::new()),
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

    /// Checks if we already have a ChatId for the given teacher.
    pub async fn is_teacher_chat_known(&self, telegram_name: &TelegramName) -> bool {
        let map = self.teacher_chat_ids.read().await;
        map.contains_key(telegram_name)
    }

    /// Records that the teacher with `telegram_name` is chatting from `chat_id`.
    ///
    /// Should be called whenever a known teacher sends any message.  If there
    /// are pending parse errors that haven't been delivered to this teacher yet,
    /// the caller is responsible for calling [`report_parse_errors`] afterwards.
    pub async fn register_teacher_chat(&self, chat_id: ChatId, telegram_name: &TelegramName) {
        let mut map = self.teacher_chat_ids.write().await;
        map.insert(telegram_name.clone(), chat_id);
    }

    /// Sends pending parse errors to all known teacher chats and pins the
    /// message.  If a teacher's chat already has a pinned message the report
    /// for that chat is **skipped**.  Any teacher whose ChatId is not yet known
    /// is skipped; the errors remain in `pending_errors` so they can be
    /// delivered when that teacher next appears (see [`deliver_pending_to`]).
    pub async fn report_parse_errors(&self, bot: &Bot) {
        let pending = self.pending_errors.read().await;
        if pending.is_empty() {
            return;
        }

        let chat_ids = self.teacher_chat_ids.read().await;
        for (_name, &chat_id) in chat_ids.iter() {
            Self::try_send_error_pin(bot, chat_id, &pending).await;
        }
    }

    /// Delivers any pending parse errors to a single teacher chat that has just
    /// become known.  Should be called immediately after [`register_teacher_chat`]
    /// so that teachers who appear after a failed reload still get their report.
    pub async fn deliver_pending_to(&self, bot: &Bot, chat_id: ChatId) {
        let pending = self.pending_errors.read().await;
        if pending.is_empty() {
            return;
        }
        Self::try_send_error_pin(bot, chat_id, &pending).await;
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
        *self.pending_errors.write().await = errors.clone();
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

    /// Attempts to send the error report to `chat_id` and pin it.
    ///
    /// Skipped silently if the chat already has a pinned message.
    /// All Telegram API errors are logged but do not propagate.
    async fn try_send_error_pin(bot: &Bot, chat_id: ChatId, errors: &[SheetParseError]) {
        // Check for an existing pinned message by fetching chat info.
        match bot.get_chat(chat_id).await {
            Ok(chat) => {
                if chat.pinned_message.is_some() {
                    log::debug!(
                        "Skipping error report to chat {chat_id}: \
                         a message is already pinned."
                    );
                    return;
                }
            }
            Err(e) => {
                log::warn!("Could not get chat info for {chat_id}: {e}");
                return;
            }
        }

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
        let msg = match bot.send_message(chat_id, &text).await {
            Ok(m) => m,
            Err(e) => {
                log::error!("Failed to send parse-error report to chat {chat_id}: {e}");
                return;
            }
        };

        // Pin it.
        if let Err(e) = bot
            .pin_chat_message(chat_id, msg.id)
            .disable_notification(true)
            .await
        {
            log::error!("Failed to pin parse-error report in chat {chat_id}: {e}");
        }
    }
}
