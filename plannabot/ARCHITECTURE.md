# Plannabot — Architecture

## Technology stack

| Crate | Purpose | Version |
|-------|---------|---------|
| **teloxide** | Telegram Bot API framework | 0.17 |
| **telluride** | Compile-time-safe MarkdownV2 formatting; calendar keyboard builder | 0.2 |
| **tokio** | Async runtime (multi-threaded) | 1.8+ |
| **dptree** | Handler routing / dependency injection | 0.5 |
| **google-sheets4** | Google Sheets API client (re-exports `hyper`, `hyper_rustls`, `oauth2`) | 5 |
| **reqwest** | HTTP client for Drive API metadata calls | 0.12 |
| **chrono** / **chrono-tz** | Date/time parsing and timezone conversion | 0.4 / 0.9 |
| **anyhow** | Error handling | 1.0 |
| **dotenv** | `.env` file loading | 0.15 |

---

## File layout

```
plannabot/
├── src/
│   ├── main.rs          – startup sequence: config → auth → schema → data load → dispatcher
│   ├── config.rs        – Config struct, reads TELOXIDE_TOKEN / GOOGLE_CREDENTIALS_PATH / SPREADSHEET_ID
│   ├── models.rs        – data types: Student, Teacher, ScheduleEntry, Payment, UserRole, LessonStatus
│   ├── state.rs         – BotState: in-memory cache + Drive-based staleness detection
│   ├── sheets/
│   │   ├── mod.rs       – SheetsClient (Sheets hub + Drive auth + reqwest client), SheetSchema,
│   │   │                  schema management (ensure_sheet / ensure_all_sheets),
│   │   │                  get_spreadsheet_modified_time()
│   │   ├── students.rs  – SheetsClient::get_students()
│   │   ├── teachers.rs  – SheetsClient::get_teachers()
│   │   ├── schedule.rs  – SheetsClient::get_schedule() / get_student_schedule() / get_teacher_schedule()
│   │   └── payments.rs  – SheetsClient::get_payments() (stub)
│   ├── api/
│   │   ├── mod.rs       – module declarations
│   │   ├── common.rs    – start() (universal, resets to default mode), format_duration()
│   │   ├── student.rs   – help(), schedule(), book()
│   │   ├── teacher.rs   – help(), schedule(), admin()
│   │   ├── impersonate.rs – impersonate() (enter), show_student_selection(), help(), schedule(), quit()
│   │   └── admin.rs     – refresh(), help(), status(), quit()
│   └── bot/
│       ├── mod.rs       – Action enum, get_username(), callback_action_handler(), message_handler()
│       ├── student.rs   – StudentCommand enum, is_student(), student_command_handler()
│       ├── teacher.rs   – TeacherCommand enum, is_teacher(), teacher_command_handler()
│       ├── impersonate.rs – ImpersonateCommand enum, is_impersonate(), impersonate_command_handler()
│       └── admin.rs     – AdminCommand enum, is_admin(), admin_command_handler()
├── .env.example
├── Cargo.toml
└── README.md
```

---

## Telegram Message Formatting

All messages sent to Telegram **must** use the `telluride` library for compile-time-safe MarkdownV2 formatting. For complete details on macros, escaping rules, imports, examples, and testing, see **[TELLURIDE.md](TELLURIDE.md)**.

---

## User Modes

The bot operates in one of four mutually exclusive modes per chat, each with its own command enum, filter function, and handler:

| Mode | Active when | Enum | Filter | Handler |
|------|------------|------|--------|---------|
| Student | UserRole::Student | `StudentCommand` | `is_student` | `student_command_handler` |
| Teacher | Teacher, not impersonating, not admin | `TeacherCommand` | `is_teacher` | `teacher_command_handler` |
| Impersonate | Teacher + impersonation active | `ImpersonateCommand` | `is_impersonate` | `impersonate_command_handler` |
| Admin | Teacher (admin=true) + admin mode active | `AdminCommand` | `is_admin` | `admin_command_handler` |

**Command cross-mode rules:**
- All enums contain `/start` → `api::common::start` (clears mode state and shows the welcome)
- All enums contain `/help` → mode-specific help
- Teacher and Admin both have admin-level management commands
- Admin and Impersonate both have `/quit` → exits that mode

**Mode semantics:**
- Student mode exposes the normal student experience, including booking and schedule queries.
- Teacher mode exposes teacher actions such as impersonation, admin, and schedule operations.
- Impersonation mode exposes the impersonated student's view plus mode control commands like `/quit`.
- Admin mode exposes administrator-only operations and status commands.

**Mode exclusivity:** entering admin mode clears impersonation; entering impersonation clears admin mode.

---

## Module responsibilities

### `config.rs`
Loads the three required environment variables at startup. Calls `dotenv::dotenv()` so a `.env` file is picked up automatically.

### `models.rs`
Pure data types with no I/O. `UserRole` is the runtime discriminant that routes every request to the correct handler branch.

### `sheets/mod.rs` — `SheetsClient`
Owns three resources built once at startup:

| Field | Type | Purpose |
|-------|------|---------|
| `hub` | `Sheets<HttpsConnector<…>>` | All Sheets API calls |
| `drive_auth` | `Authenticator<HttpsConnector<…>>` | Cloned from the same authenticator as `hub`; used to mint Drive API tokens |
| `http_client` | `reqwest::Client` | One-field Drive REST call (`files.get?fields=modifiedTime`) |

Because `Authenticator<C>` wraps its token cache in `Arc<Mutex<…>>`, cloning it before moving it into the Sheets hub gives two handles to the *same* cache — no duplicate token refreshes.

**Schema management** (`ensure_sheet` / `ensure_all_sheets`) always reads before writing:
1. Fetch the list of existing tab names.
2. If a tab is missing → create it with the full header row.
3. If a tab exists → read its header row, append only the columns that are absent.
4. Apply inferred column formats for known column names (e.g. `date`, `time`, `datetime`, `cost`, `sum`, `currency`, `duration_minutes`).

This makes startup idempotent: running the bot against an already-configured spreadsheet is safe and keeps schema/formatting aligned with the bot expectations.

### `state.rs` — `BotState`

Holds the in-memory cache and all mode/staleness state:

```
BotState
├── sheets: Arc<SheetsClient>
├── students: Arc<RwLock<HashMap<String, Student>>>   ← keyed by normalised telegram_name
├── teachers: Arc<RwLock<HashMap<String, Teacher>>>   ← keyed by normalised telegram_name
├── impersonations: RwLock<HashMap<ChatId, String>>   ← teacher ChatId → impersonated student name
├── admin_modes: RwLock<HashSet<ChatId>>              ← chats currently in admin mode
├── last_modified: Mutex<Option<DateTime<Utc>>>       ← Drive modifiedTime at last reload
└── last_checked:  Mutex<Instant>                     ← wall-clock time of last Drive check
```

`RwLock` is used for the caches (many concurrent readers, rare writers).  
`std::sync::Mutex` (not `tokio::sync::Mutex`) is used for the two staleness fields because they are only held for a few nanoseconds — never across an `.await`.

### `api/` layer
Business logic, separated from Telegram routing:

- **`api::common`** — `start()` (universal welcome + mode reset), `format_duration()` (shared helper)
- **`api::student`** — `help()`, `schedule()`, `book()` (month calendar picker)
- **`api::teacher`** — `help()`, `schedule()`, `admin()` (enter admin mode)
- **`api::impersonate`** — `impersonate()` (enter impersonation), `help()`, `schedule()`, `quit()`
- **`api::admin`** — `refresh()`, `help()`, `status()`, `quit()`

### `bot/` layer
Telegram routing. Each module is self-contained with its command enum, filter, and handler:

Each handler follows the same pattern:
```
refresh_if_needed()       ← may reload from Sheets
get_username()            ← early return if no username set
get_role() / verify mode  ← early return if wrong role/mode
try_send_errors()         ← teacher handlers only
match cmd { ... }         ← one api:: call per arm, no inline logic
```

---

## Data refresh flow

On every command or plain-text message:

```
Incoming update
      │
      ▼
Has CHECK_INTERVAL (15 s) elapsed since the last Drive check?
      │
  No ─┘   Yes
      │     │
      │     ▼
      │   GET drive/v3/files/{spreadsheetId}?fields=modifiedTime
      │     │
      │     ├─ HTTP error → log WARN, keep cache, reset timer
      │     │
      │     └─ modifiedTime == last_modified?
      │               │
      │          Yes ─┘   No
      │               │    │
      │               │    ▼
      │               │  get_students() + get_teachers() from Sheets
      │               │  update cache + last_modified
      │               │    │
      ▼               ▼    ▼
   Handle command with (possibly freshly reloaded) cache
```

**Key properties:**
- At most one Drive API call per 15 seconds, regardless of concurrent users.
- Drive errors are non-fatal: the bot continues serving the last good cache.
- Sheets reads happen only when `modifiedTime` actually changed — not on every command.
- The Schedule tab is read lazily (on `/schedule`) rather than being cached, since it can be large and is only needed per-command.

---

## Telegram update routing

```
Dispatcher (teloxide / dptree)
      │
      ├─ filter_message → is_student   → filter_command::<StudentCommand>    → student_command_handler
      ├─ filter_message → is_teacher   → filter_command::<TeacherCommand>    → teacher_command_handler
      ├─ filter_message → is_impersonate → filter_command::<ImpersonateCommand> → impersonate_command_handler
      ├─ filter_message → is_admin     → filter_command::<AdminCommand>      → admin_command_handler
      ├─ filter_callback_query → is_packed_data → callback_action_handler
      ├─ filter_message → msg.text().is_some() → message_handler
      └─ fallback endpoint → log debug, respond(())
```

`Arc<BotState>` and `Arc<InMemStore<CallbackKey, Action>>` are injected by dptree via `.dependencies(...)` in `main.rs`; handlers simply declare them as parameters.

---

## Normalisation conventions

- All Telegram usernames are stored and compared **without `@`**, **lowercase**.
- Schedule datetimes are stored as UTC internally; displayed in the user's `timezone` field (parsed via `chrono-tz`).
- Spreadsheet cells with a decimal comma (e.g. `1,5`) are accepted in addition to the standard dot.
