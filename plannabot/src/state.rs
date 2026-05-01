use crate::models::{Student, Teacher, UserRole};
use crate::sheets::SheetsClient;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

/// Minimum time between consecutive Drive API modification-time checks.
/// Within this window the cached data is assumed to still be valid.
const CHECK_INTERVAL: Duration = Duration::from_secs(15);

pub struct BotState {
    pub sheets: Arc<SheetsClient>,

    // --- Cached sheet data ---------------------------------------------------
    students: Arc<RwLock<HashMap<String, Student>>>,
    teachers: Arc<RwLock<HashMap<String, Teacher>>>,

    // --- Staleness tracking --------------------------------------------------
    /// The `modifiedTime` value returned by the Drive API the last time we
    /// successfully loaded data.  `None` means we have not yet confirmed the
    /// modification time (forces a load on the first check).
    last_modified: Mutex<Option<DateTime<Utc>>>,
    /// Instant of the most recent modification-time check (successful or not).
    /// Starts at `Instant::now() - CHECK_INTERVAL` so the very first command
    /// triggers an immediate check.
    last_checked: Mutex<Instant>,
}

impl BotState {
    pub fn new(sheets: Arc<SheetsClient>) -> Self {
        Self {
            sheets,
            students: Arc::new(RwLock::new(HashMap::new())),
            teachers: Arc::new(RwLock::new(HashMap::new())),
            last_modified: Mutex::new(None),
            // Subtract CHECK_INTERVAL so the very first command fires a check.
            last_checked: Mutex::new(Instant::now() - CHECK_INTERVAL),
        }
    }

    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Unconditionally re-reads students and teachers from Google Sheets.
    ///
    /// Called once at startup.  Use [`refresh_if_needed`] during normal
    /// operation to benefit from caching.
    pub async fn refresh(&self) -> Result<()> {
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
            Ok(()) => true,
            Err(e) => {
                log::error!("Failed to reload data from spreadsheet: {}", e);
                false
            }
        }
    }

    /// Looks up a user by Telegram name (normalised: no `'@'`, lowercase).
    ///
    /// Students are checked before teachers.
    pub async fn get_role(&self, telegram_name: &str) -> Option<UserRole> {
        let normalised = telegram_name.trim_start_matches('@').to_lowercase();

        {
            let students = self.students.read().await;
            if let Some(student) = students.get(&normalised) {
                return Some(UserRole::Student(student.clone()));
            }
        }

        {
            let teachers = self.teachers.read().await;
            if let Some(teacher) = teachers.get(&normalised) {
                return Some(UserRole::Teacher(teacher.clone()));
            }
        }

        None
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Reads both tables from Google Sheets and atomically replaces the caches.
    /// Also fetches and caches the current modification time.
    async fn do_reload(&self) -> Result<()> {
        let students = self.sheets.get_students().await?;
        let teachers = self.sheets.get_teachers().await?;
        let modified_time = self.sheets.get_spreadsheet_modified_time().await?;
        let (ns, nt) = (students.len(), teachers.len());
        *self.students.write().await = students;
        *self.teachers.write().await = teachers;
        *self.last_modified.lock().unwrap() = Some(modified_time);
        log::info!("Data reloaded: {ns} students, {nt} teachers.");
        Ok(())
    }
}
