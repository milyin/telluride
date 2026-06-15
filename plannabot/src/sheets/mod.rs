//! Google Sheets integration module.
//!
//! Provides [`SheetsClient`] for reading and writing data to a Google Spreadsheet,
//! along with [`SheetSchema`] for column-name–to–index mapping.
//!
//! The client also exposes [`SheetsClient::get_spreadsheet_modified_time`] which
//! calls the Google Drive API to retrieve the last modification timestamp of the
//! spreadsheet file.  This is used by [`crate::state::BotState`] to decide
//! whether cached data needs to be reloaded.

use std::collections::HashMap;

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use google_sheets4::{FieldMask, Sheets, api, hyper, hyper_rustls, oauth2};
use serde::Deserialize;

use crate::models::SheetParseError;

pub mod from_sheet;
pub mod pairings;
pub mod payments;
pub mod schedule;
pub mod students;
pub mod teachers;
pub mod worktime;

// ---------------------------------------------------------------------------
// Sheet tab names
// ---------------------------------------------------------------------------

pub const SHEET_STUDENTS: &str = "Students";
pub const SHEET_TEACHERS: &str = "Teachers";
pub const SHEET_SCHEDULE: &str = "Schedule";
pub const SHEET_PAYMENTS: &str = "Payments";
pub const SHEET_PAIRINGS: &str = "Pairings";
pub const SHEET_WORKTIME: &str = "Worktime";

// ---------------------------------------------------------------------------
// Required column names for each sheet
// ---------------------------------------------------------------------------

pub const STUDENTS_COLS: &[&str] = crate::models::Student::SHEET_COLS;

pub const TEACHERS_COLS: &[&str] = crate::models::Teacher::SHEET_COLS;

pub const SCHEDULE_COLS: &[&str] = crate::models::ScheduleEntry::SHEET_COLS;

pub const PAYMENTS_COLS: &[&str] = crate::models::Payment::SHEET_COLS;

pub const PAIRINGS_COLS: &[&str] = crate::models::TeacherStudentPairing::SHEET_COLS;

pub const WORKTIME_COLS: &[&str] = crate::models::Worktime::SHEET_COLS;

// ---------------------------------------------------------------------------
// Helper: column index → A1-notation letter(s)
// ---------------------------------------------------------------------------

/// Converts a zero-based column index to an A1-notation column letter string.
///
/// 0 → "A", 25 → "Z", 26 → "AA", 27 → "AB", …
pub fn col_index_to_letter(index: usize) -> String {
    let mut result = String::new();
    let mut n = index;
    loop {
        result.insert(0, (b'A' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    result
}

/// Infers a Google Sheets number format for a column name.
///
/// The matching is case-insensitive and based on common schema conventions.
fn infer_number_format(column_name: &str) -> Option<api::NumberFormat> {
    let name = column_name.trim().to_lowercase();

    let (type_, pattern) = match name.as_str() {
        "date" | "lesson_date" | "payment_date" => ("DATE", "yyyy-mm-dd"),
        "time" | "lesson_time" | "start_time" | "end_time" => ("TIME", "hh:mm"),
        "datetime" | "lesson_datetime" => ("DATE_TIME", "yyyy-mm-dd hh:mm"),
        "cost" | "sum" | "currency" | "price" | "amount" => ("CURRENCY", "#,##0.00"),
        "duration_minutes" | "duration" => ("NUMBER", "0"),
        _ => return None,
    };

    Some(api::NumberFormat {
        type_: Some(type_.to_string()),
        pattern: Some(pattern.to_string()),
    })
}

// ---------------------------------------------------------------------------
// SheetSchema
// ---------------------------------------------------------------------------

/// Maps column names to their indices within a sheet, and provides
/// typed accessor helpers for reading rows.
pub struct SheetSchema {
    pub sheet_name: String,
    /// Ordered list of column names (mirrors the header row).
    pub headers: Vec<String>,
    /// Fast column-name → index lookup.
    pub col_map: HashMap<String, usize>,
}

impl SheetSchema {
    /// Build a schema from a sheet name and a (possibly empty) header row.
    pub fn new(sheet_name: String, headers: Vec<String>) -> Self {
        let col_map = headers
            .iter()
            .enumerate()
            .map(|(i, h)| (h.clone(), i))
            .collect();
        SheetSchema {
            sheet_name,
            headers,
            col_map,
        }
    }

    /// Returns `true` if the schema has a column with the given name.
    pub fn has_column(&self, name: &str) -> bool {
        self.col_map.contains_key(name)
    }

    /// Returns the zero-based index of `name`, or `None` if absent.
    pub fn get_col(&self, name: &str) -> Option<usize> {
        self.col_map.get(name).copied()
    }

    /// Appends a new column to the schema (updates both `headers` and `col_map`).
    pub fn add_column(&mut self, name: &str) {
        let idx = self.headers.len();
        self.headers.push(name.to_string());
        self.col_map.insert(name.to_string(), idx);
    }

    /// Returns the string value of `col_name` in `row`, or `""` if the column
    /// is unknown or the row is too short.
    pub fn get_str<'a>(&self, row: &'a [String], col_name: &str) -> &'a str {
        match self.get_col(col_name) {
            Some(idx) => row.get(idx).map(|s| s.as_str()).unwrap_or(""),
            None => "",
        }
    }

    /// Returns `Some(&str)` if the cell is non-empty, `None` otherwise.
    pub fn get_optional<'a>(&self, row: &'a [String], col_name: &str) -> Option<&'a str> {
        let s = self.get_str(row, col_name);
        if s.is_empty() { None } else { Some(s) }
    }

    /// Reads `col_name` from `row`, parses it as `T` using [`from_sheet::FromSheet`],
    /// logs any errors into `errors`, and returns the parsed value or a fallback.
    pub fn get_field<T: from_sheet::FromSheet>(
        &self,
        row: &[String],
        row_num: usize,
        col_name: &str,
        errors: &mut Vec<SheetParseError>,
    ) -> T {
        T::from_sheet(self, row, row_num, col_name, errors)
    }

    /// Collects every column whose name is **not** in `known_cols` into a
    /// `HashMap`, skipping blank values.
    pub fn get_custom(&self, row: &[String], known_cols: &[&str]) -> HashMap<String, String> {
        let known_set: std::collections::HashSet<&str> = known_cols.iter().copied().collect();
        let mut custom = HashMap::new();
        for (name, &idx) in &self.col_map {
            if !known_set.contains(name.as_str()) {
                if let Some(val) = row.get(idx) {
                    if !val.is_empty() {
                        custom.insert(name.clone(), val.clone());
                    }
                }
            }
        }
        custom
    }
}

// ---------------------------------------------------------------------------
// Hub + auth type aliases
// ---------------------------------------------------------------------------

type SheetsHub = Sheets<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>;

/// The concrete authenticator type produced by `ServiceAccountAuthenticator::builder`
/// when the `hyper-rustls` feature is active (i.e. `DefaultAuthenticator`).
/// Stored separately so we can request Drive API tokens independently of the
/// Sheets hub.
type DriveAuth = oauth2::authenticator::Authenticator<
    hyper_rustls::HttpsConnector<hyper::client::HttpConnector>,
>;

// ---------------------------------------------------------------------------
// Drive API response shape
// ---------------------------------------------------------------------------

/// Minimal shape of the `files.get` Drive API response that we care about.
#[derive(Deserialize)]
struct DriveFileInfo {
    #[serde(rename = "modifiedTime")]
    modified_time: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// SheetsClient
// ---------------------------------------------------------------------------

/// Thin async wrapper around the Google Sheets API hub.
///
/// Also holds a cloned authenticator and a lightweight [`reqwest::Client`] for
/// calling the Google Drive REST API to determine the spreadsheet's last
/// modification time.
pub struct SheetsClient {
    hub: SheetsHub,
    /// Cloned from the same authenticator that the Sheets hub uses; token
    /// cache is shared via the `Arc` inside `Authenticator`.
    drive_auth: DriveAuth,
    /// Plain HTTPS client used for Drive API metadata requests.
    http_client: reqwest::Client,
    spreadsheet_id: String,
}

impl SheetsClient {
    /// Creates a new client, authenticating with the service-account key at
    /// `credentials_path`.
    ///
    /// The key is read once; a single `Authenticator` is built and then
    /// **cloned** – one copy goes into the Sheets hub, the other is kept for
    /// Drive API requests.  Because `Authenticator` wraps its state in an
    /// `Arc`, both copies share the same token cache.
    pub async fn new(credentials_path: &str, spreadsheet_id: String) -> Result<Self> {
        let key = oauth2::read_service_account_key(credentials_path)
            .await
            .context("Failed to read service account key")?;

        let auth = oauth2::ServiceAccountAuthenticator::builder(key)
            .build()
            .await
            .context("Failed to build service account authenticator")?;

        // Clone *before* moving into the hub so we can use it for Drive calls.
        let drive_auth = auth.clone();

        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .unwrap()
            .https_or_http()
            .enable_http1()
            .build();

        let hub = Sheets::new(hyper::Client::builder().build(connector), auth);

        let http_client = reqwest::Client::builder()
            .build()
            .context("Failed to build HTTP client for Drive API")?;

        Ok(SheetsClient {
            hub,
            drive_auth,
            http_client,
            spreadsheet_id,
        })
    }

    /// Returns the spreadsheet ID.
    pub fn get_spreadsheet_id(&self) -> &str {
        &self.spreadsheet_id
    }

    // -----------------------------------------------------------------------
    // Drive API: modification time
    // -----------------------------------------------------------------------

    /// Fetches the last modification timestamp of the spreadsheet from the
    /// Google Drive API (`files.get?fields=modifiedTime`).
    ///
    /// Uses the `drive.metadata.readonly` scope.  The Google Cloud project
    /// must have the Drive API enabled, and the service account must be
    /// granted read access to the spreadsheet.
    ///
    /// Returns the `modifiedTime` as a UTC [`DateTime`].
    pub async fn get_spreadsheet_modified_time(&self) -> Result<DateTime<Utc>> {
        // Request a bearer token with Drive metadata read access.
        let scopes = ["https://www.googleapis.com/auth/drive.metadata.readonly"];
        let token = self
            .drive_auth
            .token(&scopes)
            .await
            .context("Failed to obtain Drive API token")?;

        let bearer = token
            .token()
            .ok_or_else(|| anyhow!("Drive API token is empty"))?;

        let url = format!(
            "https://www.googleapis.com/drive/v3/files/{}?fields=modifiedTime",
            self.spreadsheet_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(bearer)
            .send()
            .await
            .context("Drive API request failed")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Drive API returned HTTP {}: {}", status, body));
        }

        let file_info: DriveFileInfo = response
            .json()
            .await
            .context("Failed to parse Drive API response")?;

        Ok(file_info.modified_time)
    }

    // -----------------------------------------------------------------------
    // Low-level Sheets primitives
    // -----------------------------------------------------------------------

    /// Reads all cells in `range` (e.g. `"Students!A:Z"`) and returns them as
    /// a 2-D `Vec<Vec<String>>`.  Trailing empty cells within a row are
    /// omitted by the Sheets API, so callers must handle short rows.
    pub async fn get_values(&self, range: &str) -> Result<Vec<Vec<String>>> {
        let (_, value_range) = self
            .hub
            .spreadsheets()
            .values_get(&self.spreadsheet_id, range)
            .value_render_option("FORMATTED_VALUE")
            .doit()
            .await
            .with_context(|| format!("Failed to get values for range '{range}'"))?;

        let rows = value_range.values.unwrap_or_default();
        let string_rows = rows
            .into_iter()
            .map(|row| row.into_iter().map(json_value_to_string).collect())
            .collect();
        Ok(string_rows)
    }

    /// Appends a new row to `sheet_name` using `INSERT_ROWS` mode.
    pub async fn append_row(&self, sheet_name: &str, values: Vec<serde_json::Value>) -> Result<()> {
        let range = format!("{sheet_name}!A1");
        let value_range = api::ValueRange {
            range: Some(range.clone()),
            values: Some(vec![values]),
            ..Default::default()
        };
        self.hub
            .spreadsheets()
            .values_append(value_range, &self.spreadsheet_id, &range)
            .value_input_option("USER_ENTERED")
            .insert_data_option("INSERT_ROWS")
            .doit()
            .await
            .with_context(|| format!("Failed to append row to sheet '{sheet_name}'"))?;
        Ok(())
    }

    /// Writes `values` into `range` using the `USER_ENTERED` input option.
    pub async fn update_values(
        &self,
        range: &str,
        values: Vec<Vec<serde_json::Value>>,
    ) -> Result<()> {
        let value_range = api::ValueRange {
            range: Some(range.to_string()),
            values: Some(values),
            ..Default::default()
        };
        self.hub
            .spreadsheets()
            .values_update(value_range, &self.spreadsheet_id, range)
            .value_input_option("USER_ENTERED")
            .doit()
            .await
            .with_context(|| format!("Failed to update values for range '{range}'"))?;
        Ok(())
    }

    /// Deletes the row at `row_index` (0-based) from `sheet_name`.
    pub async fn delete_row(&self, sheet_name: &str, row_index: usize) -> Result<()> {
        let sheet_id = self.get_sheet_id(sheet_name).await?;
        let idx = row_index as i32;
        let batch_req = api::BatchUpdateSpreadsheetRequest {
            requests: Some(vec![api::Request {
                delete_dimension: Some(api::DeleteDimensionRequest {
                    range: Some(api::DimensionRange {
                        sheet_id: Some(sheet_id),
                        dimension: Some("ROWS".to_string()),
                        start_index: Some(idx),
                        end_index: Some(idx + 1),
                    }),
                }),
                ..Default::default()
            }]),
            ..Default::default()
        };
        self.hub
            .spreadsheets()
            .batch_update(batch_req, &self.spreadsheet_id)
            .doit()
            .await
            .with_context(|| {
                format!("Failed to delete row {row_index} from sheet '{sheet_name}'")
            })?;
        Ok(())
    }

    /// Returns the titles of all existing sheet tabs in the spreadsheet.
    pub async fn list_sheets(&self) -> Result<Vec<String>> {
        let (_, spreadsheet) = self
            .hub
            .spreadsheets()
            .get(&self.spreadsheet_id)
            .doit()
            .await
            .context("Failed to get spreadsheet metadata")?;

        let names = spreadsheet
            .sheets
            .unwrap_or_default()
            .into_iter()
            .filter_map(|s| s.properties.and_then(|p| p.title))
            .collect();
        Ok(names)
    }

    /// Returns the numeric `sheetId` for `sheet_name`.
    pub async fn get_sheet_id(&self, sheet_name: &str) -> Result<i32> {
        let (_, spreadsheet) = self
            .hub
            .spreadsheets()
            .get(&self.spreadsheet_id)
            .doit()
            .await
            .context("Failed to get spreadsheet metadata")?;

        let sheet_id = spreadsheet
            .sheets
            .unwrap_or_default()
            .into_iter()
            .find_map(|s| {
                s.properties.and_then(|p| {
                    if p.title.as_deref() == Some(sheet_name) {
                        p.sheet_id
                    } else {
                        None
                    }
                })
            })
            .ok_or_else(|| anyhow!("Sheet '{}' not found", sheet_name))?;

        Ok(sheet_id)
    }

    /// Applies number/date/time/currency formatting to known columns.
    pub(crate) async fn apply_column_formats(
        &self,
        sheet_name: &str,
        schema: &SheetSchema,
    ) -> Result<()> {
        let sheet_id = self.get_sheet_id(sheet_name).await?;

        let mut requests: Vec<api::Request> = Vec::new();
        for (idx, column_name) in schema.headers.iter().enumerate() {
            let Some(number_format) = infer_number_format(column_name) else {
                continue;
            };

            requests.push(api::Request {
                repeat_cell: Some(api::RepeatCellRequest {
                    range: Some(api::GridRange {
                        sheet_id: Some(sheet_id),
                        start_row_index: Some(1),
                        end_row_index: Some(1_000_000),
                        start_column_index: Some(idx as i32),
                        end_column_index: Some(idx as i32 + 1),
                    }),
                    cell: Some(api::CellData {
                        user_entered_format: Some(api::CellFormat {
                            number_format: Some(number_format),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                    fields: Some(FieldMask::new(&["userEnteredFormat.numberFormat"])),
                }),
                ..Default::default()
            });
        }

        if requests.is_empty() {
            return Ok(());
        }

        self.hub
            .spreadsheets()
            .batch_update(
                api::BatchUpdateSpreadsheetRequest {
                    requests: Some(requests),
                    ..Default::default()
                },
                &self.spreadsheet_id,
            )
            .doit()
            .await
            .with_context(|| format!("Failed to apply column formats for sheet '{sheet_name}'"))?;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Schema management
    // -----------------------------------------------------------------------

    /// Ensures that `sheet_name` exists and contains at least `required_cols`.
    ///
    /// * If the sheet does **not** exist it is created and the required columns
    ///   are written as the header row.
    /// * If the sheet **does** exist but is missing some columns, the missing
    ///   columns are appended to the header row.
    ///
    /// Always reads before writing (creates/appends only what is missing).
    pub async fn ensure_sheet(
        &self,
        sheet_name: &str,
        required_cols: &[&str],
    ) -> Result<SheetSchema> {
        let existing = self.list_sheets().await?;

        if !existing.contains(&sheet_name.to_string()) {
            // ----------------------------------------------------------------
            // Create the tab
            // ----------------------------------------------------------------
            let batch_req = api::BatchUpdateSpreadsheetRequest {
                requests: Some(vec![api::Request {
                    add_sheet: Some(api::AddSheetRequest {
                        properties: Some(api::SheetProperties {
                            title: Some(sheet_name.to_string()),
                            ..Default::default()
                        }),
                    }),
                    ..Default::default()
                }]),
                ..Default::default()
            };
            self.hub
                .spreadsheets()
                .batch_update(batch_req, &self.spreadsheet_id)
                .doit()
                .await
                .with_context(|| format!("Failed to create sheet '{sheet_name}'"))?;

            // Write the header row
            let last_col = col_index_to_letter(required_cols.len().saturating_sub(1));
            let range = format!("{sheet_name}!A1:{last_col}1");
            let header_values: Vec<serde_json::Value> = required_cols
                .iter()
                .map(|c| serde_json::Value::String(c.to_string()))
                .collect();
            self.update_values(&range, vec![header_values])
                .await
                .with_context(|| format!("Failed to write headers for sheet '{sheet_name}'"))?;

            let schema = SheetSchema::new(
                sheet_name.to_string(),
                required_cols.iter().map(|s| s.to_string()).collect(),
            );

            self.apply_column_formats(sheet_name, &schema).await?;

            log::info!(
                "Created sheet '{sheet_name}' with {} columns.",
                required_cols.len()
            );
            return Ok(schema);
        }

        // --------------------------------------------------------------------
        // Sheet already exists — read its current headers first
        // --------------------------------------------------------------------
        let header_range = format!("{sheet_name}!1:1");
        let rows = self
            .get_values(&header_range)
            .await
            .with_context(|| format!("Failed to read header row for sheet '{sheet_name}'"))?;

        let existing_headers: Vec<String> = rows.into_iter().next().unwrap_or_default();
        let mut schema = SheetSchema::new(sheet_name.to_string(), existing_headers);

        // Append any missing required columns (read → decide → write only if needed)
        let mut added = false;
        for &col in required_cols {
            if !schema.has_column(col) {
                log::info!("Adding missing column '{col}' to sheet '{sheet_name}'.");
                schema.add_column(col);
                added = true;
            }
        }

        if added {
            let last_col = col_index_to_letter(schema.headers.len().saturating_sub(1));
            let range = format!("{sheet_name}!A1:{last_col}1");
            let header_values: Vec<serde_json::Value> = schema
                .headers
                .iter()
                .map(|h| serde_json::Value::String(h.clone()))
                .collect();
            self.update_values(&range, vec![header_values])
                .await
                .with_context(|| format!("Failed to update headers for sheet '{sheet_name}'"))?;
        }

        self.apply_column_formats(sheet_name, &schema).await?;

        Ok(schema)
    }

    /// Ensures all standard sheets exist with their required columns.
    pub async fn ensure_all_sheets(&self) -> Result<()> {
        self.ensure_sheet(SHEET_STUDENTS, STUDENTS_COLS).await?;
        self.ensure_sheet(SHEET_TEACHERS, TEACHERS_COLS).await?;
        self.ensure_sheet(SHEET_SCHEDULE, SCHEDULE_COLS).await?;
        self.ensure_sheet(SHEET_PAYMENTS, PAYMENTS_COLS).await?;
        self.ensure_sheet(SHEET_PAIRINGS, PAIRINGS_COLS).await?;
        self.ensure_sheet(SHEET_WORKTIME, WORKTIME_COLS).await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Converts a `serde_json::Value` cell to a plain `String`.
fn json_value_to_string(v: serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s,
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}
