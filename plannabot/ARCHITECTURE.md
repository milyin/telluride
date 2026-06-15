# Plannabot — Architecture

## Runtime overview

`main.rs` wires startup and dispatcher:

1. Load env config (`TELOXIDE_TOKEN`, `GOOGLE_CREDENTIALS_PATH`, `SPREADSHEET_ID`)
2. Build `SheetsClient`
3. Ensure required sheet tabs/columns exist
4. Load initial cache into `BotState`
5. Start teloxide dispatcher (commands + callbacks + plain text)
6. Spawn `notification_task` loop

---

## Major modules

```
plannabot/src/
├── main.rs
├── config.rs
├── models.rs
├── types.rs
├── state.rs
├── notification_task.rs
├── sheets/
│   ├── mod.rs
│   ├── from_sheet.rs
│   ├── students.rs
│   ├── teachers.rs
│   ├── schedule.rs
│   ├── payments.rs
│   ├── pairings.rs
│   └── worktime.rs
├── api/
│   ├── context.rs
│   ├── common.rs
│   ├── menus.rs
│   ├── student/mod.rs
│   ├── teacher.rs
│   ├── admin.rs
│   ├── impersonate.rs
│   ├── impersonate_student.rs
│   ├── impersonate_teacher.rs
│   ├── schedule.rs
│   ├── book.rs
│   ├── balance.rs
│   ├── payment.rs
│   ├── pairing.rs
│   ├── worktime.rs
│   ├── notification.rs
│   ├── user.rs
│   └── traits/*.rs
└── bot/
    ├── mod.rs
    ├── student.rs
    ├── teacher.rs
    ├── admin.rs
    ├── impersonate_student.rs
    └── impersonate_teacher.rs
```

---

## Data model and sheets

`SheetsClient::ensure_all_sheets()` maintains six tabs:

1. `Students`
2. `Teachers`
3. `Schedule`
4. `Payments`
5. `Pairings`
6. `Worktime`

Schema behavior:
- missing tabs are created
- missing required columns are appended
- known date/time/currency/duration columns are auto-formatted
- unknown columns are preserved as custom metadata

`from_sheet.rs` handles tolerant parsing and accumulates `SheetParseError`s.

---

## Session roles and modes

Effective role is computed in `BotState::get_effective_role` from:
- persistent identity (`Students`/`Teachers`)
- chat-scoped mode flags (admin, impersonate student, impersonate teacher)

Modes:

| Mode | Condition |
|------|-----------|
| Student | user is a student, no elevated mode |
| Teacher | user is a teacher, no elevated mode |
| Admin | teacher with `admin=true` who entered `/admin` |
| ImpersonateStudent | teacher impersonating a student |
| ImpersonateTeacher | teacher impersonating a teacher |

`/start` always clears admin and impersonation state for that chat.

---

## Dispatcher and routing

Each mode has:
- its own `BotCommands` enum
- role filter function
- text command handler
- callback command handler

Dispatcher branches:
- 5 message command branches (student/teacher/admin/impersonate-student/impersonate-teacher)
- 5 callback branches (same split)
- plain text fallback (`message_handler`)
- final generic fallback (debug log)

Callbacks use packed command payloads in per-mode `InMemStore<CallbackKey, Command>`.

---

## State and refresh strategy

`BotState` stores:
- cached `students`, `teachers`, `pairings`
- mode maps/sets (`admin_modes`, impersonations)
- staleness metadata (`last_modified`, `last_checked`)
- parse error tracking (`last_errors`, `notified_teachers`)
- notification de-dup set (`sent_notifications`)

Refresh behavior:

1. On each command/message, `refresh_if_needed()` runs
2. Drive `modifiedTime` is checked at most once per 15 seconds
3. If changed, bot reloads Students/Teachers/Pairings caches
4. Schedule/Payments/Worktime are fetched on-demand by command flows

If Drive check fails, cache remains in use (warning logged, no crash).

---

## API layer responsibilities

- `api/book.rs`: booking, listing lesson details, delete/reschedule, teacher status updates
- `api/schedule.rs`: schedule listing by role
- `api/balance.rs`: student balances (monthly/all-time, paginated)
- `api/payment.rs`: payment list/add/delete flows
- `api/pairing.rs`: teacher student-pair management (+ cost/duration/links)
- `api/worktime.rs`: weekly slots + dated exceptions
- `api/notification.rs`: reminder config and preview
- `api/user.rs`: admin student/teacher CRUD flows
- `api/impersonate*.rs`: impersonation mode UX
- `api/admin.rs`, `api/teacher.rs`, `api/student/mod.rs`: mode-level commands/help

`api/context.rs::BotCtx` centralizes "send or edit markdown message" behavior.

---

## Background notifications

`notification_task.rs` runs every 30 seconds:

1. Find students with `notification_delay` and known `chat_id`
2. Read schedule and pick each student’s next planned lesson
3. Send reminder when `lesson.datetime - delay <= now`
4. De-duplicate with `(student, lesson_datetime)` set

Reminder content includes lesson info and optional pairing links (`zoom_url`, `board_url`).

---

## Key design choices

- **Google Sheets first**: human-editable operational data
- **Drive metadata check**: cheap staleness detection before expensive reloads
- **Mode-separated command enums**: explicit routing and permission boundaries
- **Telluride markdown types**: safer Telegram MarkdownV2 message generation
