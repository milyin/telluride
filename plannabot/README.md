# Plannabot

A Telegram bot for student/teacher lesson management backed by Google Sheets.
It is implemented in Rust with [`teloxide`](https://github.com/teloxide/teloxide) and [`telluride`](https://github.com/milyin/telluride).

---

## Features

- **Role and mode aware**: Student, Teacher, Admin, Teacher→Student impersonation, Teacher→Teacher impersonation
- **Google Sheets as source of truth** with **auto schema creation** and **header backfilling**
- **Bookings with availability checks** (pairings, worktime windows, and schedule conflict detection)
- **Balance and payment workflows** with interactive menus
- **Notification system** (`/notification`) with background reminders for upcoming lessons
- **Hot data refresh** via Drive `modifiedTime` checks (throttled to once per 15 seconds)
- **Parse error reporting**: latest sheet parsing errors are pushed to teachers

---

## Spreadsheet data model

The bot uses one spreadsheet with these tabs:

| Tab | Required columns |
|-----|-------------------|
| **Students** | `telegram_name`, `name`, `timezone`, `currency`, `chat_id`, `notification_delay` |
| **Teachers** | `telegram_name`, `name`, `timezone`, `admin`, `chat_id` |
| **Schedule** | `student_telegram`, `teacher_telegram`, `datetime`, `duration_minutes`, `cost`, `status` |
| **Payments** | `student_telegram`, `date`, `sum` |
| **Pairings** | `teacher_telegram`, `student_telegram`, `cost`, `duration_minutes`, `zoom_url`, `board_url` |
| **Worktime** | `teacher_telegram`, `day_of_week`, `date`, `start_time`, `end_time` |

Extra columns are preserved as custom fields.

`Schedule.status` values:
- empty = planned (future) or completed (past, inferred from time)
- `absent`
- `cancelled`

Accepted datetime/date/time inputs include:
- RFC3339 datetime (e.g. `2024-01-15T10:00:00+03:00`)
- `YYYY-MM-DD HH:MM[:SS]`
- `DD/MM/YYYY HH:MM[:SS]`
- `YYYY-MM-DD` / `DD/MM/YYYY` / `DD.MM.YYYY` for date-only fields

---

## Setup

### 1. Create a Telegram bot

1. Open [@BotFather](https://t.me/botfather)
2. Run `/newbot`
3. Copy the bot token

### 2. Set up Google Cloud

Create/select a project in [Google Cloud Console](https://console.cloud.google.com/), then:

1. Enable **Google Sheets API**
2. Enable **Google Drive API**
3. Create a **Service Account**
4. Download a JSON key
5. Share your spreadsheet with the service account email as **Editor**

Drive API is used only for `files.get?fields=modifiedTime` to detect spreadsheet changes efficiently.

### 3. Configure environment

```bash
cp .env.example .env
```

Set required values:

```env
TELOXIDE_TOKEN=<token from BotFather>
GOOGLE_CREDENTIALS_PATH=credentials.json
SPREADSHEET_ID=<spreadsheet id from URL>
```

### 4. Build and run

```bash
cargo build --release
cargo run --release
```

On first startup the bot ensures all six tabs exist:
`Students`, `Teachers`, `Schedule`, `Payments`, `Pairings`, `Worktime`.

## Detailed usage

For complete operational documentation (bootstrap from zero, no-manual-sheet workflow, mode transitions, and per-command behavior), see **[USAGE.md](USAGE.md)**.

---

## Commands by mode

| Mode | Commands |
|------|----------|
| **Student** | `/start`, `/help`, `/schedule`, `/balance`, `/book`, `/notification` |
| **Teacher** | `/start`, `/help`, `/schedule`, `/book`, `/balance`, `/payment`, `/worktime`, `/student`, `/admin`, `/refresh` |
| **Admin** | `/start`, `/help`, `/status`, `/refresh`, `/balance`, `/payment`, `/impersonate`, `/user`, `/quit` |
| **Impersonate Student** | `/start`, `/help`, `/schedule`, `/balance`, `/book`, `/notification`, `/quit` |
| **Impersonate Teacher** | `/start`, `/help`, `/schedule`, `/balance`, `/book`, `/payment`, `/worktime`, `/quit` |

Most command flows are interactive (inline keyboards + callback actions), but command parameters are also supported for direct entry.

---

## Access control

- Users must have a Telegram username and be present in `Students` or `Teachers`.
- `Teachers.admin = true` is required to enter admin mode.
- `/start` always resets admin/impersonation mode for the current chat.

---

## Further reading

- Architecture and module map: **[ARCHITECTURE.md](ARCHITECTURE.md)**
- Telluride message formatting rules: **[TELLURIDE.md](TELLURIDE.md)**
- Development checklist/conventions: **[CLAUDE.md](CLAUDE.md)**

---

## License

MIT OR Apache-2.0
