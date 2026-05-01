# Plannabot

A Telegram bot for managing student-teacher scheduling, lessons, and payments.
Built with Rust using [`teloxide`](https://github.com/teloxide/teloxide) and [`telluride`](https://github.com/milyin/telluride).
Data is stored in a Google Spreadsheet (editable by humans, read-synced by the bot).

---

## Features

- **Role-based access** — students and teachers see different commands and data
- **Google Sheets backend** — all data lives in a human-editable spreadsheet
- **Auto-schema** — missing sheet tabs or columns are created automatically at startup
- **`/schedule`** — students see their upcoming lessons; teachers see their full list
- **Safe MarkdownV2** — all messages use the `telluride` library for compile-time-validated formatting
- **Graceful Ctrl+C shutdown**

---

## Data Model

All tables live in separate tabs of the same Google Spreadsheet.
Extra columns beyond the required ones are preserved as custom properties.

| Tab | Key columns |
|-----|-------------|
| **Students** | `telegram_name`, `name`, `timezone`, `currency`, `zoom_url`, `board_url` |
| **Teachers** | `telegram_name`, `timezone` |
| **Schedule** | `student_telegram`, `teacher_telegram`, `datetime`, `duration_minutes`, `cost`, `status` |
| **Payments** | `student_telegram`, `date`, `sum` |

`status` in Schedule: empty = planned, `done` = completed, `cancelled` = cancelled.

`datetime` formats accepted: `2024-01-15T10:00:00+03:00`, `2024-01-15 10:00`, `2024-01-15 10:00:00`, `15/01/2024 10:00`.

---

## Setup

### 1. Create a Telegram bot

1. Open [@BotFather](https://t.me/botfather) in Telegram
2. Send `/newbot` and follow the prompts
3. Copy the bot token

### 2. Set up Google Sheets access

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a project (or use an existing one)
3. Enable the **Google Sheets API**
4. Create a **Service Account** (IAM & Admin → Service Accounts)
5. Download the JSON key file for the service account
6. Create a new Google Spreadsheet (or use an existing one)
7. **Share the spreadsheet** with the service account email (Editor access)
8. Copy the Spreadsheet ID from the URL:
   `https://docs.google.com/spreadsheets/d/<SPREADSHEET_ID>/edit`

### 3. Configure environment

```bash
cp .env.example .env
# Edit .env and fill in:
#   TELOXIDE_TOKEN
#   GOOGLE_CREDENTIALS_PATH   (path to the downloaded JSON key)
#   SPREADSHEET_ID
```

### 4. Build and run

```bash
cargo build --release
cargo run --release
```

On first startup the bot will create the four required sheet tabs
(`Students`, `Teachers`, `Schedule`, `Payments`) if they don't exist yet.

---

## Commands

| Command | Student | Teacher |
|---------|---------|---------|
| `/start` | Greeting with student name | Greeting with Telegram handle |
| `/help` | List of available commands | Same (labelled "Teacher Mode") |
| `/schedule` | Upcoming planned lessons (with teacher) | Upcoming planned lessons (with students) |

Lesson times are shown in each user's own timezone (from the spreadsheet).

---

## Access control

Only users whose Telegram username appears in the **Students** or **Teachers** tab
are allowed to interact with the bot. Everyone else receives an "unauthorized" message.
Users without a Telegram username are asked to set one first.

---

## Project structure

```
plannabot/
├── src/
│   ├── main.rs          # Entry point: init, schema setup, dispatcher
│   ├── config.rs        # Environment-based configuration
│   ├── models.rs        # Data types: Student, Teacher, ScheduleEntry, Payment, UserRole
│   ├── state.rs         # BotState: Arc-wrapped cache + refresh logic
│   ├── sheets/
│   │   ├── mod.rs       # SheetsClient + SheetSchema + schema management
│   │   ├── students.rs  # get_students()
│   │   ├── teachers.rs  # get_teachers()
│   │   ├── schedule.rs  # get_schedule(), get_student/teacher_schedule()
│   │   └── payments.rs  # get_payments() (stub, future use)
│   └── bot/
│       ├── mod.rs       # Command enum + dispatcher + shared utilities
│       ├── student.rs   # Student command handlers
│       └── teacher.rs   # Teacher command handlers
├── .env.example         # Environment variable template
├── Cargo.toml
└── README.md
```

---

## License

MIT OR Apache-2.0
