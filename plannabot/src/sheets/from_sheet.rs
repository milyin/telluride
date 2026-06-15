//! [`FromSheetValue`] and [`FromSheet`] traits and their implementations.
//!
//! * [`FromSheetValue`] — low-level: parses a value from a raw `&str` cell
//!   value, returning `Result<T, String>`.
//! * [`FromSheet`] — high-level: reads a named column from a [`super::SheetSchema`]
//!   row, parses it, logs any errors into the provided `Vec<SheetParseError>`,
//!   and returns a fallback or `None` on failure.  Replaces the old
//!   `parse_timezone_or_utc` / `parse_telegram_name` helpers.

use std::str::FromStr;

use crate::types::Duration;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc, Weekday};
use chrono_tz::Tz;

use crate::models::{LessonStatus, SheetParseError, TelegramName};

use super::SheetSchema;

// ---------------------------------------------------------------------------
// FromSheetValue — raw string → T
// ---------------------------------------------------------------------------

/// Parses a value from a raw Google Sheets cell string.
pub trait FromSheetValue: Sized {
    fn from_sheet_value(s: &str) -> Result<Self, String>;
}

impl FromSheetValue for String {
    fn from_sheet_value(s: &str) -> Result<Self, String> {
        Ok(s.to_string())
    }
}

impl FromSheetValue for i64 {
    fn from_sheet_value(s: &str) -> Result<Self, String> {
        s.trim()
            .parse()
            .map_err(|_| format!("cannot parse integer '{s}'"))
    }
}

impl FromSheetValue for f64 {
    fn from_sheet_value(s: &str) -> Result<Self, String> {
        s.trim()
            .replace(',', ".")
            .parse()
            .map_err(|_| format!("cannot parse number '{s}'"))
    }
}

impl FromSheetValue for bool {
    fn from_sheet_value(s: &str) -> Result<Self, String> {
        match s.trim().to_lowercase().as_str() {
            "true" | "yes" | "1" => Ok(true),
            _ => Ok(false),
        }
    }
}

impl<T: FromSheetValue> FromSheetValue for Option<T> {
    fn from_sheet_value(s: &str) -> Result<Self, String> {
        if s.trim().is_empty() {
            Ok(None)
        } else {
            T::from_sheet_value(s).map(Some)
        }
    }
}

impl FromSheetValue for TelegramName {
    fn from_sheet_value(s: &str) -> Result<Self, String> {
        TelegramName::try_from(s).map_err(|e| e.to_string())
    }
}

impl FromSheetValue for LessonStatus {
    fn from_sheet_value(s: &str) -> Result<Self, String> {
        LessonStatus::from_str(s).ok_or_else(|| format!("unrecognised lesson status '{s}'"))
    }
}

/// Parses a timezone from an IANA name (`"Europe/Berlin"`) or a numeric UTC
/// offset in hours (`"+3"`, `"-5"`, `"3"`).
///
/// For numeric offsets the nearest whole-hour `Etc/GMT` zone is used.
/// POSIX sign convention: `Etc/GMT-3` = UTC+3, `Etc/GMT+5` = UTC-5.
impl FromSheetValue for Tz {
    fn from_sheet_value(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if s.is_empty() {
            return Err("empty timezone".to_string());
        }
        if let Ok(tz) = Tz::from_str(s) {
            return Ok(tz);
        }
        let hours: f64 = s.parse().map_err(|_| {
            format!("cannot parse timezone '{s}': not an IANA name or numeric offset")
        })?;
        if !hours.is_finite() {
            return Err(format!("invalid timezone offset '{s}'"));
        }
        let h = hours.round() as i32;
        if !(-14..=12).contains(&h) {
            return Err(format!("timezone offset {h}h is out of range [-14, +12]"));
        }
        // POSIX: UTC+N → "Etc/GMT-N", UTC-N → "Etc/GMT+N"
        let name = match h.cmp(&0) {
            std::cmp::Ordering::Equal => "Etc/GMT".to_string(),
            std::cmp::Ordering::Greater => format!("Etc/GMT-{h}"),
            std::cmp::Ordering::Less => format!("Etc/GMT+{}", -h),
        };
        Tz::from_str(&name).map_err(|_| format!("cannot resolve offset {h}h to an Etc/GMT zone"))
    }
}

impl FromSheetValue for NaiveTime {
    fn from_sheet_value(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if s.is_empty() {
            return Err("empty time".to_string());
        }
        for fmt in ["%H:%M:%S", "%H:%M"] {
            if let Ok(t) = NaiveTime::parse_from_str(s, fmt) {
                return Ok(t);
            }
        }
        // Google Sheets returns time as a decimal fraction of the day when
        // the cell has no explicit TIME format (e.g. "0,3333333333" = 08:00).
        if let Ok(frac) = s.replace(',', ".").parse::<f64>() {
            if frac >= 0.0 && frac < 1.0 {
                let total_secs = (frac * 86400.0).round() as u32;
                if let Some(t) = NaiveTime::from_hms_opt(
                    total_secs / 3600,
                    (total_secs % 3600) / 60,
                    total_secs % 60,
                ) {
                    return Ok(t);
                }
            }
        }
        Err(format!("cannot parse time '{s}'"))
    }
}

impl FromSheetValue for NaiveDate {
    fn from_sheet_value(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if s.is_empty() {
            return Err("empty date".to_string());
        }
        for fmt in ["%Y-%m-%d", "%d/%m/%Y", "%d.%m.%Y"] {
            if let Ok(d) = NaiveDate::parse_from_str(s, fmt) {
                return Ok(d);
            }
        }
        Err(format!("cannot parse date '{s}'"))
    }
}

impl FromSheetValue for Weekday {
    fn from_sheet_value(s: &str) -> Result<Self, String> {
        match s.trim().to_lowercase().as_str() {
            "monday" | "mon" | "1" => Ok(Weekday::Mon),
            "tuesday" | "tue" | "2" => Ok(Weekday::Tue),
            "wednesday" | "wed" | "3" => Ok(Weekday::Wed),
            "thursday" | "thu" | "4" => Ok(Weekday::Thu),
            "friday" | "fri" | "5" => Ok(Weekday::Fri),
            "saturday" | "sat" | "6" => Ok(Weekday::Sat),
            "sunday" | "sun" | "7" | "0" => Ok(Weekday::Sun),
            other => Err(format!("cannot parse weekday '{other}'")),
        }
    }
}

/// Tries several common datetime formats (in order):
/// 1. RFC 3339 / ISO 8601 with timezone  (`2024-05-01T14:30:00+03:00`)
/// 2. `YYYY-MM-DD HH:MM:SS`
/// 3. `YYYY-MM-DD HH:MM`
/// 4. `DD/MM/YYYY HH:MM:SS`
/// 5. `DD/MM/YYYY HH:MM`
///
/// All naive formats are treated as UTC.
impl FromSheetValue for DateTime<Utc> {
    fn from_sheet_value(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if s.is_empty() {
            return Err("empty datetime".to_string());
        }
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Ok(dt.with_timezone(&Utc));
        }
        for fmt in [
            "%Y-%m-%d %H:%M:%S",
            "%Y-%m-%d %H:%M",
            "%d/%m/%Y %H:%M:%S",
            "%d/%m/%Y %H:%M",
        ] {
            if let Ok(ndt) = NaiveDateTime::parse_from_str(s, fmt) {
                return Ok(ndt.and_utc());
            }
        }
        for fmt in ["%Y-%m-%d", "%d/%m/%Y", "%d.%m.%Y"] {
            if let Ok(d) = NaiveDate::parse_from_str(s, fmt) {
                return Ok(d.and_hms_opt(0, 0, 0).unwrap().and_utc());
            }
        }
        Err(format!("cannot parse datetime '{s}'"))
    }
}

// ---------------------------------------------------------------------------
// FromSheet — schema-aware, error-reporting
// ---------------------------------------------------------------------------

/// Reads a named column from a sheet row, parses it, logs errors into
/// `errors`, and returns a value (or fallback / `None` on failure).
///
/// This replaces the old `parse_timezone_or_utc` / `parse_telegram_name`
/// free functions.
pub trait FromSheet: Sized {
    fn from_sheet(
        schema: &SheetSchema,
        row: &[String],
        row_num: usize,
        col_name: &str,
        errors: &mut Vec<SheetParseError>,
    ) -> Self;
}

// --- helpers ----------------------------------------------------------------

fn push_error(
    schema: &SheetSchema,
    row_num: usize,
    col_name: &str,
    message: String,
    errors: &mut Vec<SheetParseError>,
) {
    let err = SheetParseError {
        sheet: schema.sheet_name.clone(),
        row: row_num,
        column: col_name.to_string(),
        message,
    };
    log::error!("{err}");
    errors.push(err);
}

// --- infallible primitives --------------------------------------------------

impl FromSheet for String {
    fn from_sheet(
        schema: &SheetSchema,
        row: &[String],
        _row_num: usize,
        col_name: &str,
        _errors: &mut Vec<SheetParseError>,
    ) -> Self {
        schema.get_str(row, col_name).to_string()
    }
}

impl FromSheet for Option<String> {
    fn from_sheet(
        schema: &SheetSchema,
        row: &[String],
        _row_num: usize,
        col_name: &str,
        _errors: &mut Vec<SheetParseError>,
    ) -> Self {
        schema.get_optional(row, col_name).map(|s| s.to_string())
    }
}

/// Parses as f64 with comma-or-dot decimal separator; returns 0.0 on failure
/// without logging an error (matches the sheet convention for optional costs).
impl FromSheet for f64 {
    fn from_sheet(
        schema: &SheetSchema,
        row: &[String],
        _row_num: usize,
        col_name: &str,
        _errors: &mut Vec<SheetParseError>,
    ) -> Self {
        f64::from_sheet_value(schema.get_str(row, col_name)).unwrap_or(0.0)
    }
}

/// Parses as i64; falls back to f64-then-round to handle CURRENCY-formatted cells
/// (e.g. "100.00" returned by the Sheets API when a column has `#,##0.00` format);
/// returns 0 on failure without logging an error.
impl FromSheet for i64 {
    fn from_sheet(
        schema: &SheetSchema,
        row: &[String],
        _row_num: usize,
        col_name: &str,
        _errors: &mut Vec<SheetParseError>,
    ) -> Self {
        let s = schema.get_str(row, col_name);
        i64::from_sheet_value(s)
            .or_else(|_| f64::from_sheet_value(s).map(|f| f.round() as i64))
            .unwrap_or(0)
    }
}

fn normalize_currency_code(s: &str) -> &str {
    match s.trim() {
        "₽" => "RUB",
        "$" => "USD",
        "€" => "EUR",
        "£" => "GBP",
        other => other,
    }
}

impl FromSheetValue for &'static rusty_money::iso::Currency {
    fn from_sheet_value(s: &str) -> Result<Self, String> {
        let code = normalize_currency_code(s).to_uppercase();
        rusty_money::iso::find(&code).ok_or_else(|| format!("unknown ISO 4217 currency code '{s}'"))
    }
}

/// Empty cell → `None` (silent). Non-empty but unrecognised → `None` + error logged.
impl FromSheet for Option<&'static rusty_money::iso::Currency> {
    fn from_sheet(
        schema: &SheetSchema,
        row: &[String],
        row_num: usize,
        col_name: &str,
        errors: &mut Vec<SheetParseError>,
    ) -> Self {
        let raw = schema.get_str(row, col_name);
        if raw.trim().is_empty() {
            return None;
        }
        match <&'static rusty_money::iso::Currency>::from_sheet_value(raw) {
            Ok(c) => Some(c),
            Err(msg) => {
                push_error(schema, row_num, col_name, msg, errors);
                None
            }
        }
    }
}

/// Empty or unrecognised cell → `false` (silent).  "true"/"yes"/"1" → `true`.
impl FromSheet for bool {
    fn from_sheet(
        schema: &SheetSchema,
        row: &[String],
        _row_num: usize,
        col_name: &str,
        _errors: &mut Vec<SheetParseError>,
    ) -> Self {
        bool::from_sheet_value(schema.get_str(row, col_name)).unwrap_or(false)
    }
}

// --- domain types -----------------------------------------------------------

/// Parses as IANA name or numeric offset; on failure logs an error and returns
/// UTC.
impl FromSheet for Tz {
    fn from_sheet(
        schema: &SheetSchema,
        row: &[String],
        row_num: usize,
        col_name: &str,
        errors: &mut Vec<SheetParseError>,
    ) -> Self {
        let raw = schema.get_str(row, col_name);
        match Tz::from_sheet_value(raw) {
            Ok(tz) => tz,
            Err(msg) => {
                push_error(
                    schema,
                    row_num,
                    col_name,
                    format!("{msg}; using UTC"),
                    errors,
                );
                chrono_tz::UTC
            }
        }
    }
}

/// Empty cell → `None` (silent).  Non-empty but invalid → `None` + error
/// logged.
impl FromSheet for Option<TelegramName> {
    fn from_sheet(
        schema: &SheetSchema,
        row: &[String],
        row_num: usize,
        col_name: &str,
        errors: &mut Vec<SheetParseError>,
    ) -> Self {
        let raw = schema.get_str(row, col_name);
        match TelegramName::from_sheet_value(raw) {
            Ok(name) => Some(name),
            Err(msg) => {
                if !raw.trim().is_empty() {
                    push_error(schema, row_num, col_name, msg, errors);
                }
                None
            }
        }
    }
}

/// Empty or unrecognised cell → `None` (no error; treated as "planned").
impl FromSheet for Option<LessonStatus> {
    fn from_sheet(
        schema: &SheetSchema,
        row: &[String],
        _row_num: usize,
        col_name: &str,
        _errors: &mut Vec<SheetParseError>,
    ) -> Self {
        LessonStatus::from_str(schema.get_str(row, col_name))
    }
}

/// Empty cell → `None` (silent).  Non-empty but unparseable → `None` + error
/// logged.
impl FromSheet for Option<DateTime<Utc>> {
    fn from_sheet(
        schema: &SheetSchema,
        row: &[String],
        row_num: usize,
        col_name: &str,
        errors: &mut Vec<SheetParseError>,
    ) -> Self {
        let raw = schema.get_str(row, col_name);
        if raw.is_empty() {
            return None;
        }
        match DateTime::<Utc>::from_sheet_value(raw) {
            Ok(dt) => Some(dt),
            Err(msg) => {
                push_error(schema, row_num, col_name, msg, errors);
                None
            }
        }
    }
}

/// Empty cell → `None` (silent).  Non-empty but unparseable → `None` + error logged.
impl FromSheet for Option<NaiveDate> {
    fn from_sheet(
        schema: &SheetSchema,
        row: &[String],
        row_num: usize,
        col_name: &str,
        errors: &mut Vec<SheetParseError>,
    ) -> Self {
        let raw = schema.get_str(row, col_name);
        if raw.is_empty() {
            return None;
        }
        match NaiveDate::from_sheet_value(raw) {
            Ok(d) => Some(d),
            Err(msg) => {
                push_error(schema, row_num, col_name, msg, errors);
                None
            }
        }
    }
}

/// Empty cell → `None` (silent).  Non-empty but unparseable → `None` + error logged.
impl FromSheet for Option<Weekday> {
    fn from_sheet(
        schema: &SheetSchema,
        row: &[String],
        row_num: usize,
        col_name: &str,
        errors: &mut Vec<SheetParseError>,
    ) -> Self {
        let raw = schema.get_str(row, col_name);
        if raw.is_empty() {
            return None;
        }
        match Weekday::from_sheet_value(raw) {
            Ok(d) => Some(d),
            Err(msg) => {
                push_error(schema, row_num, col_name, msg, errors);
                None
            }
        }
    }
}

/// Empty cell → `None` (silent). Non-empty but unparseable → `None` + error logged.
impl FromSheet for Option<Duration> {
    fn from_sheet(
        schema: &SheetSchema,
        row: &[String],
        row_num: usize,
        col_name: &str,
        errors: &mut Vec<SheetParseError>,
    ) -> Self {
        let raw = schema.get_str(row, col_name);
        if raw.trim().is_empty() {
            return None;
        }
        match raw.parse::<Duration>() {
            Ok(d) => Some(d),
            Err(e) => {
                push_error(schema, row_num, col_name, e.to_string(), errors);
                None
            }
        }
    }
}

/// Missing or unparseable cell logs an error and falls back to midnight.
impl FromSheet for NaiveTime {
    fn from_sheet(
        schema: &SheetSchema,
        row: &[String],
        row_num: usize,
        col_name: &str,
        errors: &mut Vec<SheetParseError>,
    ) -> Self {
        let raw = schema.get_str(row, col_name);
        match NaiveTime::from_sheet_value(raw) {
            Ok(t) => t,
            Err(msg) => {
                push_error(
                    schema,
                    row_num,
                    col_name,
                    format!("{msg}; using midnight"),
                    errors,
                );
                NaiveTime::from_hms_opt(0, 0, 0).unwrap()
            }
        }
    }
}
