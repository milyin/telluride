# Plannabot

A Telegram bot for managing student-teacher scheduling, lessons, and payments.
Built with Rust using [`teloxide`](https://github.com/teloxide/teloxide) and [`telluride`](https://github.com/milyin/telluride).
Data is stored in a Google Spreadsheet (editable by humans, read-synced by the bot).

---

## Features

- **Role-based access** — students and teachers see different commands and data
- **Google Sheets backend** — all data lives in a human-editable spreadsheet
- **Auto-schema** — missing sheet tabs or columns are created automatically at startup
- **Auto-formatting** — known columns are automatically formatted (`date`, `time`, `datetime`, currency-like fields, duration)
- **`/schedule`** — students see their upcoming lessons; teachers see their full list
- **Live data** — on every command the bot checks whether the spreadsheet was modified (via Google Drive API) and reloads data only when needed, with a 15-second throttle
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

### 2. Set up Google Cloud project

Go to the [Google Cloud Console](https://console.cloud.google.com/) and create or select a project.

You need to enable **two APIs** and create a service account:

#### 2a. Enable the Google Sheets API

`APIs & Services → Library → search "Google Sheets API" → Enable`

Or visit directly:
`https://console.developers.google.com/apis/api/sheets.googleapis.com/overview?project=<YOUR_PROJECT_ID>`

#### 2b. Enable the Google Drive API

`APIs & Services → Library → search "Google Drive API" → Enable`

Or visit directly:
`https://console.developers.google.com/apis/api/drive.googleapis.com/overview?project=<YOUR_PROJECT_ID>`

> **Why Drive API?** The bot calls `drive.files.get` with the `drive.metadata.readonly` scope
> to read only the spreadsheet's last-modification timestamp — no file content is ever accessed
> through Drive. This lets the bot detect when someone edits the spreadsheet and reload the
> data automatically, without re-fetching everything on every command.

#### 2c. Create a Service Account and download credentials

1. Go to `IAM & Admin → Service Accounts → Create Service Account`
2. Give it any name (e.g. `plannabot`)
3. Skip the optional role and user access steps
4. Open the created service account → **Keys** tab → **Add Key → Create new key → JSON**
5. Save the downloaded `.json` file (e.g. as `credentials.json` next to the binary)

#### 2d. Share the spreadsheet with the service account

1. Create a new Google Spreadsheet (or use an existing one)
2. Click **Share** and add the service account email  
   (looks like `plannabot@<project>.iam.gserviceaccount.com`) with **Editor** access
3. Copy the Spreadsheet ID from the URL:  
   `https://docs.google.com/spreadsheets/d/<SPREADSHEET_ID>/edit`

### 3. Configure environment

```bash
cp .env.example .env
```

Edit `.env` and fill in the three required values:

```
TELOXIDE_TOKEN=<token from BotFather>
GOOGLE_CREDENTIALS_PATH=credentials.json   # path to the JSON key file
SPREADSHEET_ID=<the long ID from the spreadsheet URL>
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
| `/book [teacher] [date] [hour] [duration]` | Month calendar picker for booking a lesson (all parameters optional) | — |

Parameters for `/book`:
- `teacher` — teacher's Telegram handle (e.g., `john_doe`)
- `date` — lesson date in YYYY-MM-DD format (e.g., `2024-12-25`)
- `hour` — lesson start time in HH:MM format (e.g., `14:30`)
- `duration` — lesson duration in minutes (e.g., `60`)

Example: `/book john_doe 2024-12-25 14:30 60`

Lesson times are shown in each user's own timezone (from the spreadsheet).

---

## Access control

Only users whose Telegram username appears in the **Students** or **Teachers** tab
are allowed to interact with the bot. Everyone else receives an "unauthorized" message.
Users without a Telegram username are asked to set one first.

---

## Further reading

For file layout, module responsibilities, data refresh logic, and Telegram routing see **[ARCHITECTURE.md](ARCHITECTURE.md)**.

For message formatting with telluride see **[TELLURIDE.md](TELLURIDE.md)**.

For development checklist and contribution guidelines see **[CLAUDE.md](CLAUDE.md)**.

---

## License

MIT OR Apache-2.0
