//! Google Sheets integration module.
//!
//! Provides [`SheetsClient`] for reading and writing data to a Google Spreadsheet,
//! along with [`SheetSchema`] for column-name–to–index mapping.

use std::collections::HashMap;

use anyhow::{Context, Result};
use google_sheets4::{api, hyper, hyper_rustls, oauth2, Sheets};

pub mod payments;
pub mod schedule;
pub mod students;
pub mod teachers;

// ---------------------------------------------------------------------------
// Sheet tab names
// ---------------------------------------------------------------------------

pub const SHEET_STUDENTS: &str = "Students";
pub const SHEET_TEACHERS: &str = "Teachers";
pub const SHEET_SCHEDULE: &str = "Schedule";
pub const SHEET_PAYMENTS: &str = "Payments";

// ---------------------------------------------------------------------------
// Required column names for each sheet
// ---------------------------------------------------------------------------

pub const STUDENTS_COLS: &[&str] = &[
    "telegram_name",
    "name",
    "timezone",
    "currency",
    "zoom_url",
    "board_url",
];

pub const TEACHERS_COLS: &[&str] = &["telegram_name", "timezone"];

pub const SCHEDULE_COLS: &[&str] = &[
    "student_telegram",
    "teacher_telegram",
    "datetime",
    "duration_minutes",
    "cost",
    "status",
];

pub const PAYMENTS_COLS: &[&str] = &["student_telegram", "date", "sum"];

// ---------------------------------------------------------------------------
// Helper: column index → A1-notation letter(s)
// ---------------------------------------------------------------------------

/// Converts a zero-based column index to an A1-notation column letter string.
///
/// ```
/// assert_eq!(col_index_to_letter(0),  "A");
/// assert_eq!(col_index_to_letter(25), "Z");
/// assert_eq!(col_index_to_letter(26), "AA");
/// assert_eq!(col_index_to_letter(27), "AB");
/// ```
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
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
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
// Hub type alias
// ---------------------------------------------------------------------------

type SheetsHub = Sheets<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>;

// ---------------------------------------------------------------------------
// SheetsClient
// ---------------------------------------------------------------------------

/// Thin async wrapper around the Google Sheets API hub.
pub struct SheetsClient {
    hub: SheetsHub,
    spreadsheet_id: String,
}

impl SheetsClient {
    /// Creates a new client, authenticating with the service-account key at
    /// `credentials_path`.
    pub async fn new(credentials_path: &str, spreadsheet_id: String) -> Result<Self> {
        let key = oauth2::read_service_account_key(credentials_path)
            .await
            .context("Failed to read service account key")?;

        let auth = oauth2::ServiceAccountAuthenticator::builder(key)
            .build()
            .await
            .context("Failed to build service account authenticator")?;

        let hub = Sheets::new(
            hyper::Client::builder().build(
                hyper_rustls::HttpsConnectorBuilder::new()
                    .with_native_roots()
                    .unwrap()
                    .https_or_http()
                    .enable_http1()
                    .build(),
            ),
            auth,
        );

        Ok(SheetsClient {
            hub,
            spreadsheet_id,
        })
    }

    // -----------------------------------------------------------------------
    // Low-level primitives
    // -----------------------------------------------------------------------

    /// Reads all cells in `range` (e.g. `"Students!A:Z"`) and returns them as
    /// a 2-D `Vec<Vec<String>>`.  Trailing empty cells within a row are
    /// omitted by the Sheets API, so callers must handle short rows.
    pub async fn get_values(&self, range: &str) -> Result<Vec<Vec<String>>> {
        let (_, value_range) = self
            .hub
            .spreadsheets()
            .values_get(&self.spreadsheet_id, range)
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
    /// Returns the final [`SheetSchema`] reflecting the actual on-disk headers.
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

            log::info!(
                "Created sheet '{sheet_name}' with {} columns.",
                required_cols.len()
            );
            return Ok(SheetSchema::new(
                sheet_name.to_string(),
                required_cols.iter().map(|s| s.to_string()).collect(),
            ));
        }

        // --------------------------------------------------------------------
        // Sheet already exists — read its current headers
        // --------------------------------------------------------------------
        let header_range = format!("{sheet_name}!1:1");
        let rows = self
            .get_values(&header_range)
            .await
            .with_context(|| format!("Failed to read header row for sheet '{sheet_name}'"))?;

        let existing_headers: Vec<String> = rows.into_iter().next().unwrap_or_default();
        let mut schema = SheetSchema::new(sheet_name.to_string(), existing_headers);

        // Append any missing required columns
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

        Ok(schema)
    }

    /// Ensures all four standard sheets (`Students`, `Teachers`, `Schedule`,
    /// `Payments`) exist with their required columns.
    pub async fn ensure_all_sheets(&self) -> Result<()> {
        self.ensure_sheet(SHEET_STUDENTS, STUDENTS_COLS).await?;
        self.ensure_sheet(SHEET_TEACHERS, TEACHERS_COLS).await?;
        self.ensure_sheet(SHEET_SCHEDULE, SCHEDULE_COLS).await?;
        self.ensure_sheet(SHEET_PAYMENTS, PAYMENTS_COLS).await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Converts a `serde_json::Value` cell to a plain `String`.
/// The Sheets API returns formatted values as JSON strings by default, but we
/// also handle numbers and booleans defensively.
fn json_value_to_string(v: serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s,
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}
