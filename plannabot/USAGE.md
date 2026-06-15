# Plannabot Usage Guide

This is the detailed operational reference for running Plannabot from scratch and using it daily without routine manual spreadsheet edits.

---

## 1. Bootstrap from zero (correct workflow)

Plannabot can manage almost all operational data via Telegram commands, but one initial bootstrap seed is required so the first admin can authenticate.

### Step 1: Start bot once to initialize schema

Run the bot (`cargo run --release`) after setting env vars.

On startup it ensures these tabs/headers exist:
- `Students`
- `Teachers`
- `Schedule`
- `Payments`
- `Pairings`
- `Worktime`

### Step 2: One-time seed of first admin (manual)

Add one row in `Teachers`:
- `telegram_name`: your Telegram username (with or without `@`)
- `name`: display name
- `timezone`: e.g. `Europe/Berlin`
- `admin`: `true`
- `chat_id`: leave `0`/empty (bot will update it after first command)

### Step 3: Enter admin mode

In Telegram:
1. `/start`
2. `/admin`

If `admin=true` for your teacher row, you enter Admin mode.

### Step 4: Create users from bot (no sheet editing)

Use `/user`:
- add teachers
- add students
- edit/delete users

### Step 5: Configure teaching structure

For each teacher, configure:
1. **Pairings** (`/student` in Teacher mode): link teacher <-> student and set default lesson duration/cost (+ optional Zoom/Board URLs).
2. **Availability** (`/worktime` in Teacher mode): set weekly slots and exceptions.

If you are admin and need to configure another teacher quickly:
- `/impersonate teacher @teacher_name`
- run `/student` and `/worktime` as that teacher

### Step 6: Start operations

Use:
- `/book` for scheduling
- `/schedule` for weekly view
- `/payment` and `/balance` for money tracking
- `/notification` for reminders

After step 2, routine operation does not require manual sheet edits.

---

## 2. Mode model and transitions

Plannabot has chat-scoped modes:

| Mode | How entered | How exited |
|------|-------------|------------|
| Student | User exists in `Students`, no elevated mode | `/start` (reset) |
| Teacher | User exists in `Teachers`, no elevated mode | `/start` (reset) |
| Admin | Teacher with `admin=true` runs `/admin` | `/quit` or `/start` |
| Impersonate Student | Admin runs `/impersonate student ...` | `/quit` or `/start` |
| Impersonate Teacher | Admin runs `/impersonate teacher ...` | `/quit` or `/start` |

Important behavior:
- `/start` always clears admin/impersonation for that chat.
- entering admin clears impersonation; entering impersonation clears admin.
- users without Telegram usernames cannot use the bot.

---

## 3. Input format rules (used by many commands)

- **Telegram username**: `@name` form (5-32 chars; letters/digits/underscore constraints enforced).
- **Duration**: humantime style (`30m`, `1h`, `1h30m`, `0s` for disable in notifications).
- **Date**: `YYYY-MM-DD`.
- **Time**: `HH:MM` (24h).
- **Timezone**:
  - IANA (`Europe/Berlin`)
  - numeric offset (`+3`, `-5`, `0`) where applicable.

Interactive inline keyboards are the recommended path.  
Most commands also accept advanced direct parameters used by callback flows.

---

## 4. Command reference by mode

## Student mode

| Command | Purpose | Notes |
|--------|---------|-------|
| `/start` | Reset mode state and show welcome | Exits any elevated mode if active |
| `/help` | Show student command list | |
| `/schedule [week_offset]` | Weekly schedule view | `week_offset`: `0` current week, `1` next, `-1` previous |
| `/balance` | Show own balance | Student is forced to own scope |
| `/book` | Book/list/manage own lessons | Uses pairings + worktime + conflict checks |
| `/notification` | Reminder setup and preview | Setup flow supports forced set (`setup! <duration>`) |

## Teacher mode

| Command | Purpose | Notes |
|--------|---------|-------|
| `/start` | Reset mode state and show welcome | |
| `/help` | Show teacher command list | |
| `/schedule [week_offset]` | Weekly schedule for this teacher | |
| `/book` | Book/list/manage lessons as teacher | Student selection comes from paired students |
| `/balance [@student [year month]]` | Student balance views | Without args -> paginated students assigned to teacher |
| `/payment` | Payment add/list/delete for paired students | Interactive by default |
| `/worktime` | Manage weekly slots and date exceptions | Add/remove flows are interactive |
| `/student` | Manage teacher-student pairings | Add/edit/remove; lesson params + Zoom/Board links |
| `/admin` | Enter Admin mode | Requires `Teachers.admin=true` |
| `/refresh` | Force immediate data refresh | |

## Admin mode

| Command | Purpose | Notes |
|--------|---------|-------|
| `/start` | Exit admin and reset | |
| `/help` | Show admin command list | |
| `/status` | Show global bot stats | Includes student/teacher counts and refresh timing |
| `/refresh` | Force immediate refresh | |
| `/balance` | Balance views for all students | Paginated |
| `/payment` | Payment management for all students | |
| `/impersonate [student|teacher] [@name]` | Enter impersonation flows | If role/name omitted, shows selection UI |
| `/user` | Manage students and teachers | Add/edit/delete via interactive forms |
| `/quit` | Exit admin mode | |

## Impersonate Student mode

| Command | Purpose | Notes |
|--------|---------|-------|
| `/start` | Exit impersonation and reset | |
| `/help` | Show impersonated-student command list | |
| `/schedule [week_offset]` | Schedule of impersonated student | |
| `/balance` | Balance of impersonated student | |
| `/book` | Booking flows as impersonated student | |
| `/notification` | Notification setup for impersonated student | |
| `/quit` | Exit impersonation | Return to acting teacher |

## Impersonate Teacher mode

| Command | Purpose | Notes |
|--------|---------|-------|
| `/start` | Exit impersonation and reset | |
| `/help` | Show impersonated-teacher command list | |
| `/schedule [week_offset]` | Schedule of impersonated teacher | |
| `/balance` | Student balances scoped to impersonated teacher | |
| `/book` | Booking flows as impersonated teacher | |
| `/payment` | Payment flows as impersonated teacher | |
| `/worktime` | Worktime management for impersonated teacher | |
| `/quit` | Exit impersonation | Return to acting teacher |

---

## 5. Command families and workflow semantics

## `/user` (admin-only user management)

Main flow:
1. choose role (student/teacher)
2. choose add/edit/delete
3. follow generated prefilled command templates

Student add/edit expects:
- `@telegram_name`
- timezone
- currency (`RUB`, `USD`, etc.; edit supports `-` to remove currency)
- display name

Teacher add/edit expects:
- `@telegram_name`
- timezone
- display name

## `/student` (teacher pairing management)

Manages `Pairings` for current (or impersonated) teacher:
- add student pairing with duration and cost
- edit lesson defaults (duration/cost)
- edit Zoom URL
- edit Board URL
- remove pairing

Pairings are prerequisites for standard booking flows.

## `/worktime` (teacher availability)

Two branches:
- **Week days**: recurring availability by weekday/time ranges
- **Exceptions**: date-specific windows

Booking availability calculation uses worktime plus existing schedule conflicts.

## `/book` (booking/list/update)

Major capabilities:
- create lesson
- list lessons and open lesson details
- delete lesson
- reschedule lesson
- update lesson status (teacher-side flow)

Validation includes:
- pairing existence checks
- conflict checks
- slot selection from computed availability

## `/payment` and `/balance`

- `/payment`: list, add, delete payment records.
- `/balance`: monthly/all-time financial view based on schedule + payment data.

## `/notification`

Student-facing reminder control:
- view nearest planned lesson and current reminder setting
- set reminder delay (`setup! <duration>`)
- disable with `0s`

Background task sends reminder before next planned lesson when delay condition is met.

---

## 6. No-manual-sheet day-to-day workflow

After initial admin seed:

1. Admin maintains users through `/user`.
2. Teachers maintain pairings through `/student`.
3. Teachers maintain availability through `/worktime`.
4. Students/teachers schedule through `/book`.
5. Teachers/admin track money with `/payment` and `/balance`.
6. Students manage reminders with `/notification`.

This keeps spreadsheet edits mostly bot-driven, while still allowing manual overrides when needed.
