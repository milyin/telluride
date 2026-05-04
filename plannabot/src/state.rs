use crate::models::{
    SheetParseError, Student, Teacher, TeacherStudentPairing, TelegramName, UserEffectiveRole,
    UserRole,
};
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
    pairings: Arc<RwLock<Vec<TeacherStudentPairing>>>,

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

    // --- Admin mode ----------------------------------------------------------
    /// The set of chat IDs currently in admin mode.  Entries are inserted by
    /// `/admin` and removed by `/quit`.
    admin_modes: RwLock<HashSet<ChatId>>,

    // --- Parse Error Reporting -----------------------------------------------
    /// Parse errors from the most recent reload.
    last_errors: RwLock<Vec<SheetParseError>>,

    /// The set of teacher TelegramNames who have ALREADY been sent the `last_errors`.
    /// This is cleared whenever `last_errors` is updated.
    notified_teachers: RwLock<HashSet<TelegramName>>,

    /// The time of the last successful data reload from the spreadsheet.
    last_reload: Mutex<Option<DateTime<Utc>>>,
}

pub struct BotStats {
    pub students_count: usize,
    pub teachers_count: usize,
    pub last_reload: Option<DateTime<Utc>>,
    pub last_modified: Option<DateTime<Utc>>,
    pub refresh_delay: Duration,
    pub spreadsheet_id: String,
}

impl BotState {
    pub fn new(sheets: Arc<SheetsClient>) -> Self {
        Self {
            sheets,
            students: Arc::new(RwLock::new(HashMap::new())),
            teachers: Arc::new(RwLock::new(HashMap::new())),
            pairings: Arc::new(RwLock::new(Vec::new())),
            last_modified: Mutex::new(None),
            // Subtract CHECK_INTERVAL so the very first command fires a check.
            last_checked: Mutex::new(Instant::now() - CHECK_INTERVAL),
            impersonations: RwLock::new(HashMap::new()),
            admin_modes: RwLock::new(HashSet::new()),
            last_errors: RwLock::new(Vec::new()),
            notified_teachers: RwLock::new(HashSet::new()),
            last_reload: Mutex::new(None),
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
        self.send_error_report(bot, chat_id, &errors).await;

        // Mark as notified
        notified.insert(telegram_name.clone());
    }

    /// Resolves the effective role of a user for the given chat session.
    ///
    /// Combines the persistent identity from [`get_role`] with the current
    /// session mode (admin or impersonation) stored by `chat_id`:
    /// - A teacher in admin mode → `UserEffectiveRole::Admin`
    /// - A teacher in impersonation mode → `UserEffectiveRole::Impersonate`
    /// - Otherwise → `UserEffectiveRole::Teacher` or `UserEffectiveRole::Student` unchanged
    pub async fn get_effective_role(
        &self,
        telegram_name: &str,
        chat_id: ChatId,
    ) -> Option<UserEffectiveRole> {
        match self.get_role(telegram_name).await? {
            UserRole::Teacher(teacher) => {
                if self.is_in_admin_mode(chat_id).await {
                    Some(UserEffectiveRole::Admin(teacher))
                } else if let Some(student_name) = self.get_impersonation(chat_id).await {
                    Some(UserEffectiveRole::Impersonate(teacher, student_name))
                } else {
                    Some(UserEffectiveRole::Teacher(teacher))
                }
            }
            UserRole::Student(student) => Some(UserEffectiveRole::Student(student)),
        }
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

    /// Returns a sorted list of all registered teacher telegram names (without '@', lowercase).
    pub async fn get_teacher_names(&self) -> Vec<String> {
        let teachers = self.teachers.read().await;
        let mut names: Vec<String> = teachers.keys().cloned().collect();
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

    /// Returns all pairings where the given teacher is the teacher.
    pub async fn get_pairings_for_teacher(
        &self,
        telegram_name: &str,
    ) -> Vec<TeacherStudentPairing> {
        let Ok(normalised) = TelegramName::try_from(telegram_name) else {
            return vec![];
        };
        let pairings = self.pairings.read().await;
        pairings
            .iter()
            .filter(|a| a.teacher_telegram == normalised)
            .cloned()
            .collect()
    }

    /// Returns all pairings where the given student is the student.
    pub async fn get_pairings_for_student(
        &self,
        telegram_name: &str,
    ) -> Vec<TeacherStudentPairing> {
        let Ok(normalised) = TelegramName::try_from(telegram_name) else {
            return vec![];
        };
        let pairings = self.pairings.read().await;
        pairings
            .iter()
            .filter(|a| a.student_telegram == normalised)
            .cloned()
            .collect()
    }

    /// Returns global statistics about the bot's state.
    pub async fn get_stats(&self) -> BotStats {
        BotStats {
            students_count: self.students.read().await.len(),
            teachers_count: self.teachers.read().await.len(),
            last_reload: *self.last_reload.lock().unwrap(),
            last_modified: *self.last_modified.lock().unwrap(),
            refresh_delay: CHECK_INTERVAL,
            spreadsheet_id: self.sheets.get_spreadsheet_id().to_string(),
        }
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
    // Admin mode API
    // -----------------------------------------------------------------------

    /// Puts `chat_id` into admin mode.
    pub async fn enter_admin_mode(&self, chat_id: ChatId) {
        self.admin_modes.write().await.insert(chat_id);
    }

    /// Returns `true` if `chat_id` is currently in admin mode.
    pub async fn is_in_admin_mode(&self, chat_id: ChatId) -> bool {
        self.admin_modes.read().await.contains(&chat_id)
    }

    /// Removes `chat_id` from admin mode.
    pub async fn exit_admin_mode(&self, chat_id: ChatId) {
        self.admin_modes.write().await.remove(&chat_id);
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
        let (pairings, pairing_errors) = self.sheets.get_pairings().await?;
        let modified_time = self.sheets.get_spreadsheet_modified_time().await?;

        errors.extend(teacher_errors);
        errors.extend(pairing_errors);

        // Also reload schedule and worktime to surface parse errors (data is read
        // on demand, but we still want to report failures at reload time).
        let (_, schedule_errors) = self.sheets.get_schedule().await?;
        errors.extend(schedule_errors);
        let (_, worktime_errors) = self.sheets.get_worktime().await?;
        errors.extend(worktime_errors);

        let (ns, nt, na) = (students.len(), teachers.len(), pairings.len());
        *self.students.write().await = students;
        *self.teachers.write().await = teachers;
        *self.pairings.write().await = pairings;
        *self.last_errors.write().await = errors.clone();
        self.notified_teachers.write().await.clear();
        *self.last_modified.lock().unwrap() = Some(modified_time);
        *self.last_reload.lock().unwrap() = Some(Utc::now());

        if errors.is_empty() {
            log::info!("Data reloaded: {ns} students, {nt} teachers, {na} pairings.");
        } else {
            log::error!(
                "Data reloaded: {ns} students, {nt} teachers, {na} pairings \
                 — {} parse error(s).",
                errors.len()
            );
        }

        Ok(errors)
    }

    /// Sends the error report to `chat_id` without pinning it.
    ///
    /// All Telegram API errors are logged but do not propagate.
    async fn send_error_report(&self, bot: &Bot, chat_id: ChatId, errors: &[SheetParseError]) {
        let sheet_id = self.sheets.get_spreadsheet_id();
        let url = format!("https://docs.google.com/spreadsheets/d/{}/edit", sheet_id);

        // Build the error message text.
        let header = format!(
            "⚠️ Spreadsheet parse errors ({} row(s) skipped):\n\n",
            errors.len()
        );
        let body: String = errors.iter().map(|e| format!("• {}\n", e)).collect();
        let footer = format!("\nLink to spreadsheet: {}", url);
        let text = format!("{}{}{}", header, body, footer);

        // Send the message.
        if let Err(e) = bot.send_message(chat_id, &text).await {
            log::error!("Failed to send parse-error report to chat {chat_id}: {e}");
        }
    }
}
